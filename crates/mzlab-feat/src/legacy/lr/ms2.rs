//! The MS2 of a row of the low-resolution Dig (`ms2`, `transitions`): the product-ion scans of the peaks of the traces that
//! `ms2link` matched to the row, pooled by fragment and compared with the parent's.

use std::collections::BTreeMap;

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{round, Out};

use super::ms2link::{runs, small, Link, Links};
use super::{Entry, LrDig};
use crate::legacy::ms2::read_scans;
use crate::legacy::Res;

/// Fragments closer than this to the precursor are the precursor itself, Da.
const PRECURSOR_GAP: f64 = 1.5;
/// Width of the bins that pool the fragments of the scans, Da.
const BIN: f64 = 0.3;
/// A fragment of the product matches one of the parent within this, Da.
const MATCH: f64 = 0.5;
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
    small(v, |a, b| a.total_cmp(b));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
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

/// The scans of a link: (file in the order of `open`, rows of its index) of its experiment inside its peak.
type Part = (usize, Vec<usize>);

fn part_of(l: &Links, k: &Link) -> Part {
    let x = &l.exps[k.exp];
    (x.k, x.within(k.lo, k.hi))
}

/// Pooled fragments of the scans of `parts` (the binning and the cut of the fragments).
fn pool_of<R: ReadAt>(
    parts: &[Part],
    top: usize,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Pool> {
    let mut out = Pool::default();
    let (mut bins, mut ces, mut precs, mut n) = (
        BTreeMap::<i64, Vec<(f64, f64)>>::new(),
        vec![],
        vec![],
        0usize,
    );
    for (k, want) in parts.iter().filter(|p| !p.1.is_empty()) {
        let mut ix = open(*k)?;
        let found: Vec<(Option<f64>, Option<f64>)> = want
            .iter()
            .map(|i| (ix.scans[*i].precursor, ix.scans[*i].collision_energy))
            .collect();
        for (peaks, (prec, ce)) in read_scans(&mut ix, want)
            .map_err(|x| x.to_string())?
            .into_iter()
            .zip(found)
        {
            n += 1;
            let prec = prec.unwrap_or(f64::NAN);
            precs.push(prec);
            ces.extend(ce.filter(|c| *c != 0.0));
            for (m, i) in peaks.0.iter().zip(&peaks.1) {
                if (m - prec).abs() >= PRECURSOR_GAP {
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
    small(&mut pooled, |a, b| b.1.total_cmp(&a.1));
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

/// Pooled fragments of `e` (without the comparison with the parent): the scans of its links (`ms2link`), those of the collision energy with the most
/// scans when it has several. `open(k)` gives the index of file `k` of the columns then the MS2 files.
pub fn pool<R: ReadAt>(
    d: &LrDig,
    e: &Entry,
    top: usize,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Pool> {
    let Some(l) = &d.links else {
        return Ok(Pool::default());
    };
    let parts: Vec<Part> = l.of(e.id).map(|(_, k)| part_of(l, k)).collect();
    let mut tagged: Vec<(i64, usize)> = l
        .of(e.id)
        .enumerate()
        .map(|(i, (_, k))| {
            (
                l.exps[k.exp]
                    .ce
                    .map_or(i64::MIN, |c| (c * 10.0).round() as i64),
                i,
            )
        })
        .collect();
    small(&mut tagged, |a, b| a.cmp(b));
    let size = |g: &Vec<Part>| g.iter().map(|p| p.1.len()).sum::<usize>();
    let best = runs(&tagged)
        .into_iter()
        .map(|(_, ix)| {
            ix.into_iter()
                .map(|i| parts[i].clone())
                .collect::<Vec<Part>>()
        })
        .fold(None::<Vec<Part>>, |b, g| match b {
            Some(x) if size(&x) >= size(&g) => Some(x),
            _ => Some(g),
        });
    match best {
        Some(g) => pool_of(&g, top, open),
        None => Ok(Pool::default()),
    }
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

fn fragments_json(o: &mut Out, m: &Pool) {
    o.raw("\"fragments\":[");
    for (i, f) in m.frags.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(f.mz)
            .raw(",\"rel\":")
            .num(f.rel);
        if m.annotated {
            o.raw(",\"loss\":")
                .opt(f.loss)
                .raw(",\"note\":")
                .str(&f.note);
        }
        o.raw("}");
    }
    o.raw("]");
}

/// The `ms2` object of `api.detail`.
pub fn ms2_json(o: &mut Out, m: &Pool) {
    o.raw("{\"scans\":").int(m.scans as i64);
    o.raw(",\"precursor\":").opt(m.precursor);
    if m.scans > 0 {
        o.raw(",\"collision_energy\":").opt(m.collision_energy);
    }
    o.raw(",");
    fragments_json(o, m);
    if m.annotated && !m.frags.is_empty() {
        o.raw(",\"shared\":")
            .int(m.shared as i64)
            .raw(",\"shifted\":")
            .int(m.shifted as i64);
    }
    o.raw("}");
}

/// The `ms2_links` array of `api.detail`: one object per match of the row with the fragments of its scans.
pub fn links_json<R: ReadAt>(
    o: &mut Out,
    d: &LrDig,
    e: &Entry,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<()> {
    o.raw("[");
    if let Some(l) = &d.links {
        for (n, (_, k)) in l.of(e.id).enumerate() {
            let (x, p) = (&l.exps[k.exp], &l.exps[k.exp].peaks[k.peak]);
            let m = pool_of(&[part_of(l, k)], TOP, open)?;
            o.raw(if n > 0 { "," } else { "" })
                .raw("{\"file\":")
                .str(&x.file);
            o.raw(",\"precursor\":")
                .num(x.precursor)
                .raw(",\"ce\":")
                .opt(x.ce);
            o.raw(",\"apex\":")
                .num(round(p.apex, 3))
                .raw(",\"lo\":")
                .num(round(p.lo, 3));
            o.raw(",\"hi\":")
                .num(round(p.hi, 3))
                .raw(",\"state\":")
                .str(k.state.name());
            o.raw(",\"why\":")
                .str(&k.why)
                .raw(",\"scans\":")
                .int(m.scans as i64)
                .raw(",");
            fragments_json(o, &m);
            o.raw("}");
        }
    }
    o.raw("]");
    Ok(())
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
