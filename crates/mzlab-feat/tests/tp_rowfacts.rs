//! The measured facts of the rows (`evid::rowfacts::row_facts`): the share of the MS2 the parent explains, the parent's fragments for gate 2 of the
//! view of the products, and the isotope pattern, which unit resolution never computes.

use mzlab_feat::evid::producers::{FeatIndex, Peaks};
use mzlab_feat::evid::rowfacts::row_facts;
use mzlab_feat::evid::rowproofs::{row_signs, Signs, FOR, NONE};
use mzlab_feat::evid::{Inputs, PointsTable};
use mzlab_feat::hypo::{Dir, Hypothesis, Outcome, Registry, SRC_RULE};
use mzlab_feat::table::{Aligned, FileFeatures, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::tp::*;

const MINUTES: [f64; 7] = [f64::NAN, 0.0, 5.0, 10.0, 20.0, 40.0, 80.0];
const PROTON: f64 = 1.00727646688;

/// The parent (decays) and one product that forms; both at `rt` 5 and 4 minutes.
fn table() -> Aligned {
    let rows = [
        (300.0, 5.0, [0.0, 1e7, 9e6, 7e6, 5e6, 3e6, 1e6]),
        (316.0, 4.0, [0.0, 0.0, 1e5, 3e5, 6e5, 8e5, 9e5]),
    ];
    let mut t = Aligned::with_capacity(7, rows.len());
    for (mz, rt, a) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for v in a {
            t.area.push(v);
            t.height.push((v / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state
                .push(if v > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

fn hyp(mz: f64, formula: &str) -> Hypothesis {
    Hypothesis {
        id: 1,
        sources: SRC_RULE,
        label: "r".into(),
        formula: formula.into(),
        neutral: mz - PROTON,
        mz,
        adduct: "[M+H]+",
        polarity: 1,
        steps: 1,
        level: 3,
        dir: Dir::Free,
        rt_ref: None,
        alternatives: Vec::new(),
    }
}

fn registry(formula: &str) -> Registry {
    let hyps = vec![hyp(316.0, formula), hyp(250.0, "")];
    Registry {
        outcomes: vec![Outcome::NotSearched; hyps.len()],
        hyps,
        rules_version: 1,
        polarity: 1,
        suspects_cut: 0,
    }
}

fn peaks(mz: &[f64]) -> Peaks {
    Peaks {
        mz: mz.to_vec(),
        it: vec![100.0; mz.len()],
    }
}

/// Spectra of the parent (row 0) and of the product (row 1), the product's shifted by 16 from the parent's.
fn spectra(child: bool) -> Vec<Option<(Peaks, f32)>> {
    vec![
        Some((peaks(&[250.0, 200.0, 150.0]), 0.9)),
        child.then(|| (peaks(&[266.0, 216.0, 150.0]), 0.8)),
    ]
}

fn status(
    facts: &[mzlab_feat::evid::Facts],
    reg: &Registry,
    unit: bool,
    frag: Option<&[f64]>,
) -> TpBlock {
    let t = table();
    let table = if unit {
        PointsTable::bundled().unwrap().with_unit()
    } else {
        PointsTable::bundled().unwrap()
    };
    let inp = Inputs {
        table: &table,
        aligned: &t,
        facts,
        arcs: &[],
        registry: reg,
        libs: &[],
        external: &[],
        kinetics: None,
        parent: Some((300.0, 5.0)),
    };
    let signs = row_signs(&inp);
    build(&TpInputs {
        aligned: &t,
        minutes: &MINUTES,
        hyps: &reg.hyps,
        signs: &signs,
        parent: (300.0, 5.0),
        fragments: frag,
        params: &TpParams::default(),
        rules: None,
    })
    .unwrap()
    .expect("the parent is in the table")
}

#[test]
fn shifted_fragments_of_the_parent_raise_the_network_and_the_kinship() {
    let reg = registry("");
    let (facts, parent_ms2) = row_facts(
        &table(),
        &reg,
        Some((300.0, 5.0)),
        &spectra(true),
        &[],
        false,
    );
    assert_eq!(facts[1].network, Some(1.0));
    assert_eq!(
        facts[0].network, None,
        "the parent has no network against itself"
    );
    assert_eq!(facts[1].purity.map(|p| (p * 10.0).round()), Some(8.0));
    assert_eq!(parent_ms2.unwrap().mz, vec![250.0, 200.0, 150.0]);

    // the network proof comes from the facts; the kinetic proof (sixth kind) is the one the series gives
    let mut signs = table_signs(&facts, &reg, &PointsTable::bundled().unwrap());
    assert_eq!(signs[1][4], FOR, "the network is a proof for the row");
    signs[1][5] = FOR;
    let kin = |signs: &[Signs]| build_kinship(&reg, signs).kinship[1];
    assert_eq!(kin(&signs), KIN_CERTAIN);
    signs[1][4] = NONE;
    assert_eq!(kin(&signs), KIN_PROBABLE);
}

fn build_kinship(reg: &Registry, signs: &[Signs]) -> TpBlock {
    let t = table();
    build(&TpInputs {
        aligned: &t,
        minutes: &MINUTES,
        hyps: &reg.hyps,
        signs,
        parent: (300.0, 5.0),
        fragments: None,
        params: &TpParams::default(),
        rules: None,
    })
    .unwrap()
    .unwrap()
}

fn table_signs(
    facts: &[mzlab_feat::evid::Facts],
    reg: &Registry,
    points: &PointsTable,
) -> Vec<Signs> {
    let t = table();
    let inp = Inputs {
        table: points,
        aligned: &t,
        facts,
        arcs: &[],
        registry: reg,
        libs: &[],
        external: &[],
        kinetics: None,
        parent: Some((300.0, 5.0)),
    };
    row_signs(&inp)
}

#[test]
fn without_a_spectrum_there_is_no_network_and_the_status_is_as_without_facts() {
    let reg = registry("");
    let (facts, parent_ms2) = row_facts(
        &table(),
        &reg,
        Some((300.0, 5.0)),
        &spectra(false),
        &[],
        false,
    );
    assert_eq!(facts[1].network, None);
    assert!(parent_ms2.is_some());
    let none: Vec<mzlab_feat::evid::Facts> = Vec::new();
    let a = status(&facts, &reg, false, None);
    let b = status(&none, &reg, false, None);
    assert_eq!((a.status, a.kinship), (b.status, b.kinship));
}

#[test]
fn the_in_source_fragment_of_the_parent_is_taken_out_by_gate_2() {
    let reg = registry("");
    let (_, parent_ms2) = row_facts(
        &table(),
        &reg,
        Some((300.0, 5.0)),
        &spectra(true),
        &[],
        false,
    );
    let frag = parent_ms2.unwrap().mz;
    // a row at m/z 250 that coelutes with the parent at a constant ratio, in a series with no t0: gate 2 takes it out only when the parent's MS2 holds it
    let mut t = table();
    t.mz[1] = 250.0;
    t.rt[1] = 5.02;
    for k in 0..7 {
        let a = t.area[k] * 0.05;
        t.area[7 + k] = a;
        t.height[7 + k] = (a / 10.0) as f32;
        t.apex_rt[7 + k] = 5.02;
        t.state[7 + k] = if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT };
    }
    let minutes = [f64::NAN, 2.0, 5.0, 10.0, 20.0, 40.0, 80.0];
    let hyps = [hyp(250.0, "")];
    let signs = vec![[NONE; 10]; 2];
    let run = |f: Option<&[f64]>| {
        build(&TpInputs {
            aligned: &t,
            minutes: &minutes,
            hyps: &hyps,
            signs: &signs,
            parent: (300.0, 5.0),
            fragments: f,
            params: &TpParams::default(),
            rules: None,
        })
        .unwrap()
        .unwrap()
    };
    assert_eq!(
        run(None).status[1],
        ST_ISF,
        "a parent with no MS2 does not contradict"
    );
    assert_ne!(run(Some(&[200.0, 150.0])).status[1], ST_ISF);
    assert_eq!(run(Some(&frag)).status[1], ST_ISF);
}

/// A file whose peak at the product's m/z has an M+1 at the expected ratio.
fn isotopes() -> FileFeatures {
    let mut f = FileFeatures::default();
    for (mz, area) in [(316.0, 1e6), (316.0 + 1.003355, 3.0e5)] {
        f.mz.push(mz);
        f.rt.push(4.0);
        f.left.push(3.9);
        f.right.push(4.1);
        f.area.push(area);
        f.area_win.push(area);
    }
    f
}

#[test]
fn unit_resolution_computes_no_isotope_proof_and_leaves_the_level() {
    let reg = registry("C20H25N3O2");
    let feats = isotopes();
    let idx = [FeatIndex::new(&feats)];
    let hr = row_facts(
        &table(),
        &reg,
        Some((300.0, 5.0)),
        &spectra(true),
        &idx,
        false,
    )
    .0;
    let lr = row_facts(
        &table(),
        &reg,
        Some((300.0, 5.0)),
        &spectra(true),
        &idx,
        true,
    )
    .0;
    assert!(
        hr[1].isotope.is_some(),
        "the same row at high resolution is scored"
    );
    assert!(lr.iter().all(|f| f.isotope.is_none()));
    assert!(hr
        .iter()
        .all(|f| f.network == lr[1].network || f.network.is_none()));
    // the same table, the same facts but for the isotope: the level of the identities is the one of the run without isotope facts
    let mut no_iso = hr.clone();
    for f in &mut no_iso {
        f.isotope = None;
    }
    let level = |facts: &[mzlab_feat::evid::Facts]| {
        let t = table();
        let table = PointsTable::bundled().unwrap().with_unit();
        let inp = Inputs {
            table: &table,
            aligned: &t,
            facts,
            arcs: &[],
            registry: &reg,
            libs: &[],
            external: &[],
            kinetics: None,
            parent: Some((300.0, 5.0)),
        };
        row_signs(&inp)
    };
    assert_eq!(level(&lr), level(&no_iso));
}
