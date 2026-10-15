//! `api.detail(id)` of a low-resolution Dig: the criteria, the cells of the row by file and the XIC of every full-scan column (m/z of the
//! extraction ± the tolerance, read from its `.mzidx`), with the peak of the pipeline on it.

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{round, Out};

use super::labels::Cell;
use super::ms2;
use super::spec;
use super::summary::{criteria, level, strs};
use super::{Entry, LrDig};
use crate::legacy::Res;

/// At most this many points of a trace go to the page.
const MAX_POINTS: usize = 700;

/// The XIC of one file: RT and summed intensity of the MS1 scans after `rt_min`.
pub fn xic<R: ReadAt>(
    ix: &mut Index<R>,
    lo: f64,
    hi: f64,
    rt_min: f64,
) -> Res<(Vec<f64>, Vec<f64>)> {
    let Some(si) = ix.section_of(1, None) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let e = |e: mzlab_core::error::Error| e.to_string();
    let rt = ix.arrays(si).map_err(e)?.rt.clone();
    let (a, b) = (
        ix.search(si, lo, false).map_err(e)?,
        ix.search(si, hi, true).map_err(e)?,
    );
    let y = ix.sum_window(si, a, b).map_err(e)?;
    Ok(rt.into_iter().zip(y).filter(|p| p.0 >= rt_min).unzip())
}

/// What the trace says of the peak of the pipeline: S/N (height over the median of the positive points) and points inside its limits.
fn on_trace(rt: &[f64], y: &[f64], c: &Cell) -> (f64, usize) {
    let mut pos: Vec<f64> = y.iter().copied().filter(|v| *v > 0.0).collect();
    mzlab_core::sortc::by(&mut pos, |a, b| a.total_cmp(b));
    let noise = if pos.is_empty() {
        0.0
    } else {
        pos[pos.len() / 2]
    };
    let snr = if noise > 0.0 { c.height / noise } else { 0.0 };
    let pts = rt
        .iter()
        .filter(|t| c.apex_rt.is_some() && **t >= c.lo && **t <= c.hi)
        .count();
    (snr, pts)
}

fn peak_json(o: &mut Out, c: &Cell, snr: f64, pts: usize) {
    if c.apex_rt.is_none() {
        o.raw("null");
        return;
    }
    o.raw("{\"apex_rt\":")
        .opt(c.apex_rt.map(|v| round(v, 4)))
        .raw(",\"height\":")
        .num(round(c.height, 4));
    o.raw(",\"area\":")
        .num(round(c.area, 4))
        .raw(",\"fwhm\":")
        .num(round(c.hi - c.lo, 4));
    o.raw(",\"snr\":")
        .num(round(snr, 4))
        .raw(",\"points\":")
        .int(pts as i64);
    o.raw(",\"left\":")
        .num(round(c.lo, 4))
        .raw(",\"right\":")
        .num(round(c.hi, 4));
    o.raw(",\"ok\":")
        .raw(if c.detected { "true" } else { "false" })
        .raw("}");
}

/// The rows (one per file) and the traces of an entry.
fn files_json<R: ReadAt>(
    o: &mut Out,
    d: &LrDig,
    e: &Entry,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<()> {
    let tol = d.settings.tol_da;
    let mut rows = Out::default();
    let mut traces = Out::default();
    for (k, f) in d.lc.iter().enumerate() {
        let (rt, y) = xic(&mut open(k)?, e.mz_x - tol, e.mz_x + tol, d.settings.rt_min)?;
        let c = &e.cells[k];
        let (snr, pts) = on_trace(&rt, &y, c);
        let sep = if k > 0 { "," } else { "" };
        rows.raw(sep)
            .raw("{\"label\":")
            .str(&f.label)
            .raw(",\"time\":")
            .opt(f.time)
            .raw(",\"type\":")
            .str(&f.kind);
        rows.raw(",\"area\":")
            .num(c.area)
            .raw(",\"height\":")
            .num(c.height)
            .raw(",\"snr\":")
            .num(snr);
        rows.raw(",\"points\":")
            .int(pts as i64)
            .raw(",\"apex_rt\":")
            .opt(c.apex_rt);
        rows.raw(",\"detected\":")
            .raw(if c.detected { "true" } else { "false" })
            .raw("}");
        let step = (rt.len() / MAX_POINTS).max(1);
        let rs: Vec<f64> = rt.iter().step_by(step).copied().collect();
        let ys: Vec<f64> = y.iter().step_by(step).map(|v| v.round()).collect();
        traces
            .raw(sep)
            .raw("{\"label\":")
            .str(&f.label)
            .raw(",\"time\":")
            .opt(f.time)
            .raw(",\"type\":")
            .str(&f.kind);
        traces
            .raw(",\"rt\":")
            .rounded(&rs, 3)
            .raw(",\"y\":")
            .floats(&ys)
            .raw(",\"peak\":");
        peak_json(&mut traces, c, snr, pts);
        traces.raw("}");
    }
    o.raw(",\"rows\":[")
        .raw(&rows.0)
        .raw("],\"traces\":[")
        .raw(&traces.0)
        .raw("]");
    Ok(())
}

/// `api.detail(id)`; `open(k)` gives the index of column `k`.
pub fn detail<R: ReadAt>(
    d: &LrDig,
    id: i64,
    mut open: impl FnMut(usize) -> Res<Index<R>>,
) -> Res<String> {
    let e = d
        .entries
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("no candidate {id}"))?;
    let mut o = Out::default();
    o.raw("{\"id\":")
        .int(e.id)
        .raw(",\"name\":")
        .str(&e.name)
        .raw(",\"formula\":")
        .str(&e.formula);
    o.raw(",\"mz\":")
        .num(round(e.mz, 4))
        .raw(",\"mz_extracted\":")
        .num(round(e.mz_x, 3));
    o.raw(",\"mz_observed\":")
        .opt(e.observed.map(|v| round(v, 3)));
    o.raw(",\"delta\":").str(&e.delta).raw(",\"alternatives\":");
    strs(&mut o, &e.alternatives);
    o.raw(",\"score\":")
        .opt(e.score)
        .raw(",\"label\":")
        .str(e.label)
        .raw(",\"criteria\":");
    criteria(&mut o, &e.criteria);
    let (lev, ltxt) = level(e);
    o.raw(",\"level\":")
        .opt(lev.map(f64::from))
        .raw(",\"level_text\":")
        .str(ltxt);
    o.raw(",\"ref_rt\":").opt(e.ref_rt);
    files_json(&mut o, d, e, &mut open)?;
    let m = ms2::ms2(d, e, &mut open)?;
    o.raw(",\"ms2\":");
    ms2::ms2_json(&mut o, &m);
    o.raw(",\"ms2_links\":");
    ms2::links_json(&mut o, d, e, &mut open)?;
    if let Some((_, c)) = d
        .links
        .as_ref()
        .and_then(|l| l.sims.iter().find(|s| s.0 == e.id))
    {
        o.raw(",\"sim\":");
        c.json(&mut o);
    }
    o.raw(",\"transitions\":");
    if e.unexpected || e.steps > 0 || d.entries.first().is_some_and(|p| p.id == e.id) {
        ms2::transitions_json(&mut o, d, e, &m);
    } else {
        o.raw("[]");
    }
    o.raw(",\"specificity\":");
    spec::specificity_json(&mut o, d, e, &mut open)?;
    o.raw("}");
    Ok(o.0)
}
