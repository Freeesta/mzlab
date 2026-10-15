//! The rules of the match (`docs/studi/TABUI_MS2_LR.md` §4), applied experiment by experiment: polarity, m/z, isotope, time, drift, one peak and
//! several rows, in-source fragments, corrections of the user and the experiments that no row got.

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::round;

use super::exp::{Exp, PeakL};
use super::follow::{corrections, leftovers};
use super::{
    runs, small, Link, Links, Override, State, Unmatched, FEW_OUTSIDE, ISO, PREC_TOL, SN_DRIFT,
    SN_LINK,
};
use crate::legacy::lr::detail::xic;
use crate::legacy::lr::peak1d::find_peak;
use crate::legacy::lr::{summary, Entry, LrDig};
use crate::legacy::Res;

/// Two peaks of a trace closer than this are the same peak, minutes.
pub(super) const SAME_PEAK: f64 = 0.05;
/// A correction finds its peak within this of the apex it was made on (fraction of `rt_tol_min`), and a precursor / CE within this.
pub(super) const KEY_APEX: f64 = 0.5;
pub(super) const KEY_TOL: f64 = 0.05;
/// An isotope peak of a molecule of this size is never more than about this fraction of the ion it comes from.
pub(super) const ISO_MAX: f64 = 0.5;

#[derive(Clone, Copy)]
pub(super) struct Hit {
    pub(super) apex: f64,
    pub(super) lo: f64,
    pub(super) hi: f64,
    pub(super) sn: f64,
    pub(super) window: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Pair<'a> {
    pub(super) row: &'a Entry,
    pub(super) hit: Hit,
    pub(super) dmz: f64,
    pub(super) drt: f64,
}

pub(super) fn median(v: &mut [f64]) -> f64 {
    small(v, |a, b| a.total_cmp(b));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Rule 4: the peak of the trace around `at`. An experiment acquired only around that time (fewer than `FEW_OUTSIDE` scans farther than `tol`) is
/// one peak, the scans inside the window.
pub(super) fn find(e: &Exp, at: f64, tol: f64) -> Option<Hit> {
    let outside = e.rt.iter().filter(|t| (**t - at).abs() > tol).count();
    if outside < FEW_OUTSIDE {
        let inside: Vec<usize> = (0..e.rt.len())
            .filter(|&i| (e.rt[i] - at).abs() <= tol)
            .collect();
        let top = inside.iter().copied().fold(None::<usize>, |b, i| match b {
            Some(j) if e.tic[j] >= e.tic[i] => Some(j),
            _ => Some(i),
        })?;
        return Some(Hit {
            apex: e.rt[top],
            lo: e.rt[inside[0]],
            hi: e.rt[inside[inside.len() - 1]],
            sn: 0.0,
            window: true,
        });
    }
    let f = find_peak(&e.rt, &e.tic, Some(at), tol, 3, 3).filter(|f| f.ok && f.snr >= SN_LINK)?;
    Some(Hit {
        apex: f.apex_rt,
        lo: f.left,
        hi: f.right,
        sn: f.snr,
        window: false,
    })
}

/// The index of the peak of the trace that `h` is, added when the list does not have it.
pub(super) fn register(e: &mut Exp, h: &Hit) -> usize {
    let n = e.rt.iter().filter(|t| **t >= h.lo && **t <= h.hi).count();
    if let Some(i) = e
        .peaks
        .iter()
        .position(|p| (p.apex - h.apex).abs() <= SAME_PEAK)
    {
        if h.window {
            (e.peaks[i].lo, e.peaks[i].hi, e.peaks[i].scans) = (h.lo, h.hi, n);
        }
        return i;
    }
    e.peaks.push(PeakL {
        apex: h.apex,
        lo: h.lo,
        hi: h.hi,
        sn: h.sn,
        scans: n,
    });
    e.peaks.len() - 1
}

pub(super) fn dmz(r: &Entry, p: f64) -> f64 {
    (p - r.mz).abs().min((p - r.mz_x).abs())
}

/// Rule 2: the rows whose m/z (calculated or extracted) is within `PREC_TOL` of the precursor. A row that is the isotope peak of another is not one.
pub(super) fn compatible(d: &LrDig, p: f64) -> Vec<&Entry> {
    d.entries
        .iter()
        .filter(|e| e.ref_rt.is_some() && e.isotope_of.is_none() && dmz(e, p) <= PREC_TOL)
        .collect()
}

/// Time of the row in the full-scan file with the sampling time of the experiment (its apex there), else its reference RT.
pub(super) fn expected(d: &LrDig, r: &Entry, e: &Exp) -> f64 {
    let at = e.time.and_then(|t| {
        d.lc.iter()
            .position(|f| f.time.is_some_and(|x| (x - t).abs() < 1e-6))
    });
    at.and_then(|k| r.cells.get(k))
        .filter(|c| c.detected)
        .and_then(|c| c.apex_rt)
        .or(r.ref_rt)
        .unwrap_or(0.0)
}

/// Rule 5: the drift of every MS2 file, median of (apex of the trace - expected time) over the rows alone at their precursor with S/N >= `SN_DRIFT`;
/// from two matches, at most `rt_tol_min`.
pub(super) fn drifts(d: &LrDig, exps: &[Exp]) -> Vec<(usize, f64)> {
    let tol = d.settings.rt_tol_min;
    let mut by: Vec<(usize, f64)> = Vec::new();
    for e in exps {
        if let [r] = compatible(d, e.precursor).as_slice() {
            let at = expected(d, r, e);
            if let Some(h) = find(e, at, tol).filter(|h| !h.window && h.sn >= SN_DRIFT) {
                by.push((e.k, h.apex - at));
            }
        }
    }
    small(&mut by, |a, b| a.0.cmp(&b.0));
    runs(&by)
        .into_iter()
        .filter(|(_, v)| v.len() >= 2)
        .map(|(k, mut v)| (k, median(&mut v).clamp(-tol, tol)))
        .collect()
}

pub(super) struct Ctx<'a> {
    pub(super) d: &'a LrDig,
    pub(super) polarity: i8,
    pub(super) ov: &'a [Override],
    pub(super) used: Vec<bool>,
    pub(super) links: Vec<Link>,
    pub(super) un: Vec<Unmatched>,
}

pub(super) fn side(drt: f64) -> &'static str {
    if drt >= 0.0 {
        "after"
    } else {
        "before"
    }
}

pub(super) fn link_of(
    x: usize,
    peak: usize,
    p: &Pair,
    state: State,
    alt: Vec<i64>,
    tail: &str,
) -> Link {
    let when = if p.hit.window {
        "acquired only around this time".to_string()
    } else {
        format!(
            "MS2 peak {:.2} min {} the full-scan apex",
            p.drt.abs(),
            side(p.drt)
        )
    };
    Link {
        exp: x,
        peak,
        row: p.row.id,
        state,
        dmz: round(p.dmz, 2),
        drt: round(p.drt, 3),
        alternatives: alt,
        why: format!(
            "precursor {:.2} Da from the m/z of the row; {when}{tail}",
            p.dmz
        ),
    }
}

pub(super) fn pair<'a>(d: &LrDig, e: &Exp, r: &'a Entry) -> Option<Pair<'a>> {
    let at = expected(d, r, e);
    let hit = find(e, at + e.drift, d.settings.rt_tol_min)?;
    Some(Pair {
        row: r,
        dmz: dmz(r, e.precursor),
        drt: hit.apex - at,
        hit,
    })
}

/// Height of the XIC of `mz` ± `half` in column `k` within `tol` of `apex`.
pub(super) fn height<R: ReadAt>(
    ix: &mut Index<R>,
    mz: f64,
    half: f64,
    apex: f64,
    tol: f64,
    rt_min: f64,
) -> Res<f64> {
    let (rt, y) = xic(ix, mz - half, mz + half, rt_min)?;
    Ok(rt
        .iter()
        .zip(&y)
        .filter(|(t, _)| (**t - apex).abs() <= tol)
        .map(|(_, v)| *v)
        .fold(0.0, f64::max))
}

/// Rule 3: the row of which the precursor is the isotope peak, when no compatible row is closer, the row has its peak in the trace and the ion at
/// the isotope mass is at most `ISO_MAX` times the row in its strongest file (a stronger one is an ion of its own).
pub(super) fn isotope<'a, R: ReadAt>(
    d: &'a LrDig,
    e: &Exp,
    comp: &[&Entry],
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Option<Pair<'a>>> {
    let near = comp
        .iter()
        .map(|r| dmz(r, e.precursor))
        .fold(f64::INFINITY, f64::min);
    let mut cand: Vec<(f64, f64, &Entry)> = Vec::new();
    for r in d
        .entries
        .iter()
        .filter(|r| r.ref_rt.is_some() && r.isotope_of.is_none())
    {
        for iso in ISO {
            let x = (e.precursor - r.mz - iso)
                .abs()
                .min((e.precursor - r.mz_x - iso).abs());
            if x <= PREC_TOL && x < near {
                cand.push((x, iso, r));
            }
        }
    }
    small(&mut cand, |a, b| a.0.total_cmp(&b.0));
    let (tol, half) = (d.settings.rt_tol_min, d.settings.tol_da);
    for (_, iso, r) in cand {
        let Some(p) = pair(d, e, r) else { continue };
        let k = (0..r.cells.len()).fold(0, |b, i| {
            if r.cells[i].height > r.cells[b].height {
                i
            } else {
                b
            }
        });
        let Some(apex) = r.cells.get(k).and_then(|c| c.apex_rt) else {
            continue;
        };
        let mut ix = open(k)?;
        let strong = height(&mut ix, r.mz_x, half, apex, tol, d.settings.rt_min)?;
        let weak = height(&mut ix, r.mz_x + iso, half, apex, tol, d.settings.rt_min)?;
        if weak < ISO_MAX * strong {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

pub(super) fn lighter_than_parent(d: &LrDig, e: &Exp) -> bool {
    e.precursor < d.parent.mz - PREC_TOL
}

/// Rule 7: pairs at the time of the parent with a precursor lighter than it. An in-source row keeps its pair (state `isf`); the others do not
/// get it: the peak is listed as `parent_rt`. Returns the ids of the rows that are in-source.
pub(super) fn in_source<'a>(
    c: &mut Ctx,
    e: &mut Exp,
    x: usize,
    pairs: &mut Vec<Pair<'a>>,
) -> (Vec<i64>, Vec<usize>) {
    let d = c.d;
    let Some(par) = d
        .entries
        .first()
        .filter(|p| p.id == 0 && p.ref_rt.is_some())
    else {
        return (vec![], vec![]);
    };
    if !lighter_than_parent(d, e) {
        return (vec![], vec![]);
    }
    let at = expected(d, par, e) + e.drift;
    let (mut isf, mut gone) = (vec![], vec![]);
    let mut keep = Vec::new();
    for p in std::mem::take(pairs) {
        if p.row.id == 0 || (p.hit.apex - at).abs() > 0.5 * d.settings.rt_tol_min {
            keep.push(p);
            continue;
        }
        let verdict = d.isf.iter().find(|v| v.0 == p.row.id).map(|v| &v.1);
        if summary::insource(d, p.row, par.ref_rt, verdict) {
            isf.push(p.row.id);
            keep.push(p);
            continue;
        }
        let peak = register(e, &p.hit);
        gone.push(peak);
        c.un.push(Unmatched {
            exp: Some(x),
            peak: Some(peak),
            reason: "parent_rt",
            rows: vec![p.row.id],
            why: format!(
                "lighter than the parent and at its time ({:.2} min): not attached to a product",
                p.hit.apex
            ),
        });
    }
    *pairs = keep;
    (isf, gone)
}

/// Rule 6: one peak, several rows: the row with the smaller |ΔRT| (then |Δm/z|) gets it, `ambiguous`, the others are its alternatives.
pub(super) fn assign(
    c: &mut Ctx,
    e: &mut Exp,
    x: usize,
    pairs: Vec<Pair>,
    isf: &[i64],
    taken: &[usize],
) -> Vec<usize> {
    let mut by: Vec<(usize, Pair)> = Vec::new();
    for p in pairs {
        let peak = register(e, &p.hit);
        if !taken.contains(&peak) {
            by.push((peak, p));
        }
    }
    let mut claimed = Vec::new();
    small(&mut by, |a, b| {
        (a.0.cmp(&b.0))
            .then(a.1.drt.abs().total_cmp(&b.1.drt.abs()))
            .then(a.1.dmz.total_cmp(&b.1.dmz))
    });
    for (peak, v) in runs(&by) {
        let alt: Vec<i64> = v[1..].iter().map(|p| p.row.id).collect();
        let (state, tail) = if !alt.is_empty() {
            (
                State::Ambiguous,
                format!("; {} other row(s) fit as well", alt.len()),
            )
        } else if isf.contains(&v[0].row.id) {
            (State::Isf, "; in-source fragment of the parent".to_string())
        } else {
            (State::Auto, String::new())
        };
        c.links.push(link_of(x, peak, &v[0], state, alt, &tail));
        claimed.push(peak);
    }
    claimed
}

pub(super) fn one<R: ReadAt>(
    c: &mut Ctx,
    e: &mut Exp,
    x: usize,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<()> {
    let d = c.d;
    if c.polarity != 0 && e.polarity != 0 && c.polarity != e.polarity {
        let why = format!(
            "{} polarity experiment in a {} series",
            if e.polarity > 0 {
                "positive"
            } else {
                "negative"
            },
            if c.polarity > 0 {
                "positive"
            } else {
                "negative"
            }
        );
        c.un.push(Unmatched {
            exp: Some(x),
            peak: None,
            reason: "polarity",
            rows: vec![],
            why,
        });
        return Ok(());
    }
    let before = c.links.len();
    let mut taken = corrections(c, e, x);
    let comp = compatible(d, e.precursor);
    let mut pairs: Vec<Pair> = comp.iter().filter_map(|r| pair(d, e, r)).collect();
    if let Some(p) = isotope(d, e, &comp, open)? {
        let peak = register(e, &p.hit);
        let tail = format!("; the isotope peak of row {}", p.row.id);
        c.links
            .push(link_of(x, peak, &p, State::Isotope, vec![], &tail));
        taken.push(peak);
        pairs.clear();
    }
    let any_pair = !pairs.is_empty();
    let (isf, gone) = in_source(c, e, x, &mut pairs);
    taken.extend(&gone);
    let mut claimed = assign(c, e, x, pairs, &isf, &taken);
    claimed.extend(taken);
    leftovers(c, e, x, &comp, &claimed, any_pair, c.links.len() > before);
    Ok(())
}

/// All the rules: the matches of `exps` to the rows of `d`.
pub fn link<R: ReadAt>(
    d: &LrDig,
    mut exps: Vec<Exp>,
    polarity: i8,
    ov: &[Override],
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Links> {
    let drift = drifts(d, &exps);
    let mut c = Ctx {
        d,
        polarity,
        ov,
        used: vec![false; ov.len()],
        links: vec![],
        un: vec![],
    };
    for (x, e) in exps.iter_mut().enumerate() {
        e.drift = drift.iter().find(|x| x.0 == e.k).map_or(0.0, |x| x.1);
        one(&mut c, e, x, open)?;
    }
    for (o, _) in ov.iter().zip(&c.used).filter(|(_, u)| !**u) {
        let exp = exps
            .iter()
            .position(|e| e.file == o.file && (e.precursor - o.precursor).abs() <= KEY_TOL);
        c.un.push(Unmatched {
            exp,
            peak: None,
            reason: "stale",
            rows: o.row.into_iter().collect(),
            why: format!(
                "the correction for {} at m/z {:.2}, {:.2} min finds no peak (or no row) any more",
                o.file, o.precursor, o.apex
            ),
        });
    }
    let parent_has_ms2 = c.links.iter().any(|l| l.row == 0 && l.state.pooled());
    Ok(Links {
        exps,
        links: c.links,
        unmatched: c.un,
        parent_has_ms2,
        rt_tol: d.settings.rt_tol_min,
    })
}
