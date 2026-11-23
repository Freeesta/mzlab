//! The evidence of each point of a Dig row (keys `table_row`, `points`, `k_points` and `controls` of `rows[]`): per column the area, the S/N of its
//! XIC and its state in the shared words of `aligned::EVIDENCE` (detected = passed `peakchk`: S/N ≥ `min_snr` and a peak shape); how many kinetic
//! points carry a detected peak (the others enter the kinetics as zeros); and the detected peak of the row in the blank, the dark control and the
//! standard against the treated maximum. A forced or filled area (noise integrated where no peak passed) never counts as detected.

use mzlab_core::json::{round, Out};

use super::super::series::FileIn;
use super::rows::is_treated;
use super::{peakchk, Entry, LrDig};
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

/// The largest detected area of the row in the columns of `kind` over the largest detected area in the treated samples (0 when no column of `kind`
/// holds a detected peak); `None` without such a column or without a treated peak.
fn ratio(d: &LrDig, r: usize, kind: &str) -> Option<f64> {
    let t = &d.table;
    let cols: Vec<usize> = (0..d.lc.len()).filter(|&k| d.lc[k].kind == kind).collect();
    let amax = (0..d.lc.len())
        .filter(|&k| is_treated(&d.lc[k]) && t.state[t.cell(r, k)] == CELL_DETECTED)
        .map(|k| t.area[t.cell(r, k)])
        .filter(|a| a.is_finite())
        .fold(0.0, f64::max);
    if cols.is_empty() || amax <= 0.0 {
        return None;
    }
    let top = cols
        .iter()
        .filter(|&&k| t.state[t.cell(r, k)] == CELL_DETECTED)
        .map(|&k| t.area[t.cell(r, k)])
        .filter(|a| a.is_finite())
        .fold(0.0, f64::max);
    Some(top / amax)
}

/// The kinetic points (timed treated samples, t0 included) with a detected peak: the points that inform a rate constant.
pub fn k_points(d: &LrDig, r: usize) -> usize {
    let t = &d.table;
    (0..d.lc.len())
        .filter(|&k| {
            d.lc[k].kind == "sample"
                && d.lc[k].time.is_some()
                && t.state[t.cell(r, k)] == CELL_DETECTED
        })
        .count()
}

/// The keys of the evidence of row `e`, appended to its object.
pub fn json(o: &mut Out, d: &LrDig, e: &Entry) {
    let Some(r) = e.row.filter(|&r| r < d.table.len()) else {
        o.raw(",\"table_row\":null,\"points\":[],\"k_points\":0,\"controls\":{\"blank\":null,\"dark\":null,\"standard\":null}");
        return;
    };
    let t = &d.table;
    o.raw(",\"table_row\":").int(r as i64).raw(",\"points\":[");
    for (k, f) in d.lc.iter().enumerate() {
        let c = t.cell(r, k);
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"file\":")
            .str(&f.name)
            .raw(",\"role\":")
            .str(role(f))
            .raw(",\"time\":")
            .opt(f.time)
            .raw(",\"area\":")
            .opt(t.area[c].is_finite().then(|| round(t.area[c], 1)))
            .raw(",\"snr\":")
            .opt(peakchk::snr(t, &d.checks, r, k).map(|x| round(x, 1)))
            .raw(",\"state\":")
            .str(evidence_word(
                t.state[c],
                d.checks.q[c].is_some_and(|q| !q.ok()),
            ))
            .raw("}");
    }
    o.raw("],\"k_points\":").int(k_points(d, r) as i64);
    o.raw(",\"controls\":{\"blank\":")
        .opt(ratio(d, r, "blank").map(|x| round(x, 3)))
        .raw(",\"dark\":")
        .opt(ratio(d, r, "control").map(|x| round(x, 3)))
        .raw(",\"standard\":")
        .opt(ratio(d, r, "standard").map(|x| round(x, 3)))
        .raw("}");
}
