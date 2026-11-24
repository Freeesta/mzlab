//! The evidence of each point of a Dig row (keys `table_row`, `points`, `k_points` and `controls` of `rows[]`): per column the area, the S/N of its
//! XIC and its state in the shared words of `aligned::EVIDENCE` (detected = passed `peakchk`: S/N ≥ `min_snr` and a peak shape); how many kinetic
//! points carry a detected peak (the others enter the kinetics as zeros); and the detected peak of the row in the blank, the dark control and the
//! standard against the treated maximum. A forced or filled area (noise integrated where no peak passed) never counts as detected. In a series
//! that compares conditions (one alignment of every file), each point names its condition and `by_cond` repeats the kinetic points and the
//! control ratios in the view of each condition (`cond::members`); `cond` is the condition whose criteria the row carries. `sn_floor` and
//! `checks` are the one-ion noise floor (R1, `floorchk`), evidence only.

use mzlab_core::json::{round, Out};

use super::super::series::FileIn;
use super::rows::is_treated;
use super::{cond, peakchk, Entry, LrDig};
use crate::aligned::{evidence_word, CELL_DETECTED};

/// The role of a column as `samples/1` names it.
pub fn role(f: &FileIn) -> &'static str {
    match f.kind.as_str() {
        "sample" if f.time == Some(0.0) => "t0",
        "sample" => "sample",
        "blank" => "blank",
        "control" => "dark",
        "standard" => "std",
        _ => "other",
    }
}

/// Among the columns `among`: the largest detected area of the row in those of `kind` over the largest detected area in the treated samples (0
/// when no column of `kind` holds a detected peak); `None` without such a column or without a treated peak.
fn ratio(d: &LrDig, r: usize, kind: &str, among: &[usize]) -> Option<f64> {
    let t = &d.table;
    let top = |sel: &dyn Fn(&FileIn) -> bool| {
        among
            .iter()
            .filter(|&&k| sel(&d.lc[k]) && t.state[t.cell(r, k)] == CELL_DETECTED)
            .map(|&k| t.area[t.cell(r, k)])
            .filter(|a| a.is_finite())
            .fold(0.0, f64::max)
    };
    let amax = top(&is_treated);
    if !among.iter().any(|&k| d.lc[k].kind == kind) || amax <= 0.0 {
        return None;
    }
    Some(top(&|f| f.kind == kind) / amax)
}

/// Among the columns `among`, the kinetic points (timed treated samples, t0 included) with a detected peak: the points that inform a rate constant.
fn k_in(d: &LrDig, r: usize, among: &[usize]) -> usize {
    let t = &d.table;
    among
        .iter()
        .filter(|&&k| {
            d.lc[k].kind == "sample"
                && d.lc[k].time.is_some()
                && t.state[t.cell(r, k)] == CELL_DETECTED
        })
        .count()
}

/// The keys of the evidence of row `e`, appended to its object.
pub fn json(o: &mut Out, d: &LrDig, e: &Entry) {
    let Some(r) = e.row.filter(|&r| r < d.table.len()) else {
        o.raw(",\"table_row\":null,\"points\":[],\"k_points\":0,\"controls\":{\"blank\":null,\"dark\":null,\"standard\":null}")
            .raw(",\"cond\":")
            .str(&e.cond)
            .raw(",\"by_cond\":[]");
        super::floorchk::json(o, d, e);
        return;
    };
    let t = &d.table;
    // the conditions of a series that compares them (none with one condition: its bytes stay those of a single kinetic)
    let conds = cond::conditions(&d.lc);
    let multi = conds.len() >= 2;
    o.raw(",\"table_row\":").int(r as i64).raw(",\"points\":[");
    for (k, f) in d.lc.iter().enumerate() {
        let c = t.cell(r, k);
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"file\":")
            .str(&f.name)
            .raw(",\"role\":")
            .str(role(f))
            .raw(",\"cond\":")
            .str(if multi { &f.cond } else { "" })
            .raw(",\"time\":")
            .opt(f.time)
            .raw(",\"area\":")
            .opt(t.area[c].is_finite().then(|| round(t.area[c], 1)))
            .raw(",\"snr\":")
            .opt(peakchk::snr(t, &d.checks, r, k).map(|x| round(x, 1)))
            .raw(",\"sn_floor\":")
            .opt(super::floorchk::sn_floor(d, r, k).map(|x| round(x, 1)))
            .raw(",\"state\":")
            .str(evidence_word(
                t.state[c],
                d.checks.q[c].is_some_and(|q| !q.ok()),
            ))
            .raw("}");
    }
    let all: Vec<usize> = (0..d.lc.len()).collect();
    o.raw("],\"k_points\":").int(k_in(d, r, &all) as i64);
    o.raw(",\"controls\":{\"blank\":")
        .opt(ratio(d, r, "blank", &all).map(|x| round(x, 3)))
        .raw(",\"dark\":")
        .opt(ratio(d, r, "control", &all).map(|x| round(x, 3)))
        .raw(",\"standard\":")
        .opt(ratio(d, r, "standard", &all).map(|x| round(x, 3)))
        .raw("}");
    o.raw(",\"cond\":").str(&e.cond).raw(",\"by_cond\":[");
    for (i, c) in conds.iter().filter(|_| multi).enumerate() {
        // the view that judges the condition: its treated samples, its own t0 and controls or else the shared ones
        let m = cond::members(&d.lc, c);
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"cond\":")
            .str(c)
            .raw(",\"k_points\":")
            .int(k_in(d, r, &m) as i64)
            .raw(",\"blank\":")
            .opt(ratio(d, r, "blank", &m).map(|x| round(x, 3)))
            .raw(",\"dark\":")
            .opt(ratio(d, r, "control", &m).map(|x| round(x, 3)))
            .raw("}");
    }
    o.raw("]");
    super::floorchk::json(o, d, e);
}
