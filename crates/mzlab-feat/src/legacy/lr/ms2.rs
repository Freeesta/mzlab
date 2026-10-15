//! The MS2 of a row of the low-resolution Dig (`engine.py::ms2`, `transitions`): the product-ion scans of the MS2 files whose precursor is the ion
//! (within the tolerance, as extracted or as calculated) and whose time lies inside its peak in the full-scan files, pooled by fragment and
//! compared with the parent's.

use std::collections::BTreeMap;

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{round, Out};

use super::plan::Settings;
use super::{Entry, LrDig};
use crate::legacy::ms2::read_scans;
use crate::legacy::Res;

/// Fragments closer than this to the precursor are the precursor itself, Da.
const PRECURSOR_GAP: f64 = 1.5;
/// Width of the bins that pool the fragments of the scans, Da.
const BIN: f64 = 0.3;
/// A fragment of the product matches one of the parent within this, Da.
const MATCH: f64 = 0.5;
/// Margin added to the half width of the window when the scans are picked, minutes.
const MARGIN: f64 = 0.05;
/// A file with fewer scans of the precursor outside the window is acquired only around the row: no signal control.
const MIN_OUTSIDE: usize = 10;
/// The scans of the window count when their strongest TIC is at least this times the median TIC of the whole trace.
const SN_MS2: f64 = 3.0;
/// Fragments kept: product and parent (`top`).
const TOP: usize = 6;
const TOP_PARENT: usize = 10;
/// Fragments of the parent that go to the supposed tree.
pub const TOP_TREE: usize = 12;
const MIN_REL: f64 = 0.05;

#[derive(Debug, Clone, Default)]
pub struct Frag {
    pub mz: f64,
    pub rel: f64,
    pub loss: Option<f64>,
    pub note: String,
}

#[derive(Debug, Clone, Default)]
pub struct Pool {
    pub scans: usize,
    pub precursor: Option<f64>,
    pub collision_energy: Option<f64>,
    pub frags: Vec<Frag>,
    pub shared: usize,
    pub shifted: usize,
    /// Compared with the parent (a product with fragments): the fragments carry `loss` and `note`.
    pub annotated: bool,
}

fn median(v: &mut [f64]) -> f64 {
    mzlab_core::sortc::by(v, |a, b| a.total_cmp(b));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Window of the MS2 scans of `e`: centred on its reference RT, as wide as the usual half width of its peak (median over the cells whose
/// apex lies within `rt_tol_min` of it) plus `rt_tol_min` and `MARGIN`; `None` when no cell is on that peak.
fn window(e: &Entry, s: &Settings) -> Option<(f64, f64)> {
    let centre = e.ref_rt?;
    let mut half: Vec<f64> = e
        .cells
        .iter()
        .filter(|c| {
            c.apex_rt
                .is_some_and(|a| (a - centre).abs() <= s.rt_tol_min)
        })
        .map(|c| (c.hi - c.lo) / 2.0)
        .collect();
    if half.is_empty() {
        return None;
    }
    let w = s.rt_tol_min + median(&mut half) + MARGIN;
    Some((centre - w, centre + w))
}

/// Whether the trace of the level-2 scans of one precursor has a signal inside `[lo, hi]`: always when the file was acquired only around
/// the window, else when the strongest TIC inside is `SN_MS2` times the median TIC of the whole trace.
fn has_signal(trace: &[(f64, f64)], lo: f64, hi: f64) -> bool {
    let outside = trace.iter().filter(|t| t.0 < lo || t.0 > hi).count();
    if outside < MIN_OUTSIDE {
        return true;
    }
    let mut all: Vec<f64> = trace.iter().map(|t| t.1).collect();
    let top = trace
        .iter()
        .filter(|t| t.0 >= lo && t.0 <= hi)
        .map(|t| t.1)
        .fold(f64::NEG_INFINITY, f64::max);
    top >= SN_MS2 * median(&mut all)
}

/// `(scan, precursor, RT)` of the level-2 scans of `ix` for the precursor of `e` that lie in `win`, if the trace has a signal there.
fn picked_scans<R: ReadAt>(
    ix: &Index<R>,
    e: &Entry,
    tol: f64,
    win: (f64, f64),
) -> Vec<(usize, f64, f64)> {
    let near: Vec<(usize, f64, f64, f64)> = ix
        .scans
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let p = s.precursor?;
            let ok = (p - e.mz_x).abs() <= tol || (p - e.mz).abs() <= tol;
            (s.level == 2 && ok).then_some((i, p, s.rt, s.tic))
        })
        .collect();
    let trace: Vec<(f64, f64)> = near.iter().map(|x| (x.2, x.3)).collect();
    if !has_signal(&trace, win.0, win.1) {
        return vec![];
    }
    near.iter()
        .filter(|x| x.2 >= win.0 && x.2 <= win.1)
        .map(|x| (x.0, x.1, x.2))
        .collect()
}

/// One (weighted m/z, mean intensity per scan) per bin.
fn average(bins: &BTreeMap<i64, Vec<(f64, f64)>>, n: usize) -> Vec<(f64, f64)> {
    bins.values()
        .map(|v| {
            let w: f64 = v.iter().map(|x| x.1).sum();
            let m = if w > 0.0 {
                v.iter().map(|x| x.0 * x.1).sum::<f64>() / w
            } else {
                v[0].0
            };
            (m, w / n as f64)
        })
        .collect()
}

/// Pooled fragments of `e` (without the comparison with the parent). `open(k)` gives the index of file `k` of the columns then the MS2 files.
pub fn pool<R: ReadAt>(
    d: &LrDig,
    e: &Entry,
    top: usize,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Pool> {
    let mut out = Pool::default();
    let Some((lo, hi)) = window(e, &d.settings) else {
        return Ok(out);
    };
    let tol = d.settings.tol_da;
    let (mut bins, mut ces, mut precs, mut n) = (
        BTreeMap::<i64, Vec<(f64, f64)>>::new(),
        vec![],
        vec![],
        0usize,
    );
    let skip = d.lc.len();
    for j in 0..d.files.iter().filter(|f| f.kind == "ms2").count() {
        let mut ix = open(skip + j)?;
        let picked = picked_scans(&ix, e, tol, (lo, hi));
        let ces_of: Vec<Option<f64>> = picked
            .iter()
            .map(|p| ix.scans[p.0].collision_energy)
            .collect();
        let want: Vec<usize> = picked.iter().map(|p| p.0).collect();
        for ((peaks, p), ce) in read_scans(&mut ix, &want)
            .map_err(|x| x.to_string())?
            .into_iter()
            .zip(&picked)
            .zip(ces_of)
        {
            n += 1;
            precs.push(p.1);
            ces.extend(ce.filter(|c| *c != 0.0));
            for (m, i) in peaks.0.iter().zip(&peaks.1) {
                if (m - p.1).abs() >= PRECURSOR_GAP {
                    bins.entry((m / BIN).round() as i64)
                        .or_default()
                        .push((*m, *i));
                }
            }
        }
    }
    if n == 0 {
        return Ok(out);
    }
    let mut pooled = average(&bins, n);
    let mx = pooled.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    pooled.retain(|p| p.1 >= MIN_REL * mx);
    pooled.sort_by(|a, b| b.1.total_cmp(&a.1));
    pooled.truncate(top);
    out.scans = n;
    out.precursor = Some(round(median(&mut precs), 2));
    out.collision_energy = (!ces.is_empty()).then(|| median(&mut ces));
    out.frags = pooled
        .iter()
        .map(|p| Frag {
            mz: round(p.0, 2),
            rel: round(100.0 * p.1 / mx, 1),
            ..Frag::default()
        })
        .collect();
    Ok(out)
}

/// `engine.ms2(e)`: the pool of `e` annotated against the parent (loss from the precursor, shared or shifted by the mass difference).
pub fn ms2<R: ReadAt>(
    d: &LrDig,
    e: &Entry,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Pool> {
    let mut res = pool(d, e, TOP, open)?;
    let Some(parent) = d.entries.first().filter(|p| p.id != e.id) else {
        return Ok(res);
    };
    if res.frags.is_empty() {
        return Ok(res);
    }
    let par = pool(d, parent, TOP_PARENT, open)?;
    res.annotated = true;
    let dm = e.neutral_mass.zip(parent.neutral_mass).map(|(a, b)| a - b);
    for f in &mut res.frags {
        f.loss = res.precursor.map(|p| round(p - f.mz, 2));
        for pf in &par.frags {
            if (pf.mz - f.mz).abs() <= MATCH {
                f.note = "shared with the parent".into();
                res.shared += 1;
                break;
            }
            if let Some(dm) = dm.filter(|dm| (pf.mz + dm - f.mz).abs() <= MATCH) {
                f.note = format!("parent fragment {} + Δ ({dm:+.2})", pf.mz);
                res.shifted += 1;
                break;
            }
        }
    }
    Ok(res)
}

/// The `ms2` object of `api.detail`.
pub fn ms2_json(o: &mut Out, m: &Pool) {
    let annotated = m.annotated;
    o.raw("{\"scans\":").int(m.scans as i64);
    o.raw(",\"precursor\":").opt(m.precursor);
    if m.scans > 0 {
        o.raw(",\"collision_energy\":").opt(m.collision_energy);
    }
    o.raw(",\"fragments\":[");
    for (i, f) in m.frags.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(f.mz)
            .raw(",\"rel\":")
            .num(f.rel);
        if annotated {
            o.raw(",\"loss\":")
                .opt(f.loss)
                .raw(",\"note\":")
                .str(&f.note);
        }
        o.raw("}");
    }
    o.raw("]");
    if annotated && !m.frags.is_empty() {
        o.raw(",\"shared\":")
            .int(m.shared as i64)
            .raw(",\"shifted\":")
            .int(m.shifted as i64);
    }
    o.raw("}");
}

/// `engine.transitions([id])`: one candidate transition (Q1 > Q3) per pooled fragment.
pub fn transitions_json(o: &mut Out, d: &LrDig, e: &Entry, m: &Pool) {
    let offset = d.offset.as_ref().map_or(0.0, |x| x.offset);
    o.raw("[");
    for (i, f) in m.frags.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"compound\":")
            .str(&e.name)
            .raw(",\"formula\":")
            .str(&e.formula);
        o.raw(",\"Q1\":")
            .num(round(e.mz, 1))
            .raw(",\"Q3_osservato\":")
            .num(f.mz);
        o.raw(",\"Q3_consigliato\":")
            .num(round(f.mz - offset, 1))
            .raw(",\"CE\":");
        match m.collision_energy.filter(|c| *c != 0.0) {
            Some(c) => o.num(c),
            None => o.str(""),
        };
        o.raw(",\"RT_attesa_min\":");
        match e.ref_rt.filter(|r| *r != 0.0) {
            Some(r) => o.num(round(r, 2)),
            None => o.str(""),
        };
        o.raw(",\"finestra_RT_min\":")
            .num(round(2.0 * d.settings.rt_tol_min, 2));
        o.raw(",\"intensita_rel_%\":")
            .num(f.rel)
            .raw(",\"scansioni_ms2\":")
            .int(m.scans as i64);
        o.raw(",\"nota\":")
            .str("candidate: to be optimised on the instrument")
            .raw("}");
    }
    o.raw("]");
}
