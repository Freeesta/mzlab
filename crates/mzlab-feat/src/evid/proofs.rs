//! The proofs of one row, graded by the table of points: mass, isotopes, MS2, fragments, network, kinetics, controls, RT, frequency, external.

use super::graph::{Inputs, RT_MARGIN, SOURCE_FEATURES};
use super::*;
use crate::hypo::Dir;
use crate::kinetic::{kinetic_report, Controls};
use crate::table::*;

pub(super) fn graded(
    inp: &Inputs,
    kind: Kind,
    value: f64,
    support: Option<f64>,
    opposed: bool,
    params: Vec<(&'static str, f64)>,
) -> Evidence {
    Evidence::graded(
        inp.table,
        kind,
        value,
        support,
        opposed,
        SOURCE_FEATURES,
        EVID_VERSION,
        params,
    )
}

pub(super) fn frequency(inp: &Inputs, row: usize) -> (Evidence, usize, usize) {
    let nf = inp.aligned.n_files.max(1);
    let cells = &inp.aligned.state[row * nf..(row + 1) * nf];
    let det = cells.iter().filter(|&&s| s == CELL_DETECTED).count();
    let filled = cells.iter().filter(|&&s| s == CELL_FILLED).count();
    let e = graded(
        inp,
        Kind::Frequency,
        det as f64 / nf as f64,
        None,
        false,
        vec![("detected", det as f64), ("files", nf as f64)],
    );
    (e, det, filled)
}

pub(super) fn kinetic_proofs(inp: &Inputs, row: usize) -> Vec<Evidence> {
    let Some(k) = &inp.kinetics else {
        return vec![];
    };
    let nf = inp.aligned.n_files;
    let area = |f: &usize| {
        let a = inp.aligned.area[row * nf + f];
        if a.is_finite() {
            a
        } else {
            0.0
        }
    };
    let (y, ctrl): (Vec<f64>, Vec<f64>) = (
        k.treated.iter().map(area).collect(),
        k.controls.iter().map(area).collect(),
    );
    if y.len() != k.setup.times.len() || y.iter().all(|&v| v <= 0.0) {
        return vec![];
    }
    let rep = kinetic_report(k.setup, &y, &ctrl);
    let kin = Evidence::from_kinetic(inp.table, &rep.evidence);
    let fc = if rep.controls == Controls::NoSignal {
        f64::NAN
    } else {
        rep.log2fc
    };
    let ctl = graded(
        inp,
        Kind::Controls,
        fc,
        None,
        rep.controls == Controls::Present,
        vec![],
    );
    vec![kin, ctl]
}

pub(super) fn rt_proof(inp: &Inputs, row: usize, hyp: Option<usize>) -> Option<Evidence> {
    let (_, prt) = inp.parent?;
    let dir = inp.registry.hyps[hyp?].dir;
    let d = inp.aligned.rt[row] - prt;
    let opposed = match dir {
        Dir::MorePolar => d > RT_MARGIN,
        Dir::LessPolar => d < -RT_MARGIN,
        Dir::Free => return None,
    };
    Some(graded(
        inp,
        Kind::Rt,
        d,
        None,
        opposed,
        vec![("margin", RT_MARGIN)],
    ))
}
