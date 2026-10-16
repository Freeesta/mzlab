//! The representative MS2 scan of a row for `api.detail`: among the scans of its matches that lie inside the peak of its feature (`ms2assoc`), the one
//! with the highest intensity, its spectrum, how it was associated, and the sentence «Fragment ions found: …» (`ms2frag`).

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{round, Out};

use super::ms2assoc::{self, Assoc, Feature, Ms2Scan};
use super::ms2frag::{self, Found, Ion};
use super::ms2link::{Link, Links, PREC_TOL};
use super::{Entry, LrDig};
use crate::legacy::ms2::read_scans;
use crate::legacy::Res;

/// The sentence of a row with no MS2 scan inside the peak of its feature.
pub const NO_MS2: &str = "No MS\u{b2} in the feature peak";

struct Rep {
    /// Row of the scan in the index of its file `file` (order of `open`) and the experiment it belongs to.
    scan: usize,
    file: usize,
    exp: usize,
    assoc: Assoc,
    intensity: f64,
    intensity_src: &'static str,
}

fn represent<R: ReadAt>(
    l: &Links,
    k: &Link,
    mz: f64,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Option<Rep>> {
    let x = &l.exps[k.exp];
    let ix = open(x.k)?;
    let rows = x.within(k.lo, k.hi);
    let mut src = "scan TIC";
    let scans: Vec<Ms2Scan> = rows
        .iter()
        .map(|i| {
            let s = &ix.scans[*i];
            if s.pint.is_some_and(|v| v > 0.0) {
                src = "precursor intensity";
            }
            Ms2Scan {
                rt: s.rt,
                precursor: s.prec(x.precursor),
                half_window: s.iso.map(|(a, b)| (b - a) / 2.0),
                intensity: s.pint.filter(|v| *v > 0.0).unwrap_or(s.tic),
            }
        })
        .collect();
    let f = Feature {
        mz,
        apex: x.peaks[k.peak].apex - x.drift,
        lo: k.lo,
        hi: k.hi,
    };
    Ok(ms2assoc::associate(&f, &scans, PREC_TOL).map(|a| Rep {
        intensity: scans[a.rep].intensity,
        scan: rows[a.rep],
        file: x.k,
        exp: k.exp,
        assoc: a,
        intensity_src: src,
    }))
}

/// The most intense representative among the matches of the row.
fn best<R: ReadAt>(
    d: &LrDig,
    e: &Entry,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Option<Rep>> {
    let Some(l) = &d.links else {
        return Ok(None);
    };
    let mz = e.observed.unwrap_or(e.mz_x);
    let mut out: Option<Rep> = None;
    for (_, k) in l.of(e.id) {
        if let Some(r) = represent(l, k, mz, open)? {
            if out.as_ref().is_none_or(|b| r.intensity > b.intensity) {
                out = Some(r);
            }
        }
    }
    Ok(out)
}

fn found_json(o: &mut Out, found: &[Found]) {
    o.raw(",\"found\":[");
    for (i, f) in found.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(round(f.mz, 4));
        o.raw(",\"intensity\":").num(round(f.intensity, 1));
        o.raw(",\"rel\":")
            .num(round(f.rel, 1))
            .raw(",\"loss\":")
            .num(round(f.loss, 4));
        o.raw(",\"loss_formula\":");
        match &f.loss_formula {
            Some(s) => o.str(s),
            None => o.raw("null"),
        };
        o.raw("}");
    }
    o.raw("]");
}

fn assoc_json(o: &mut Out, r: &Rep) {
    let a = &r.assoc;
    o.raw(",\"intensity\":").num(round(r.intensity, 1));
    o.raw(",\"intensity_src\":").str(r.intensity_src);
    o.raw(",\"dmz\":")
        .num(round(a.dmz, 4))
        .raw(",\"dppm\":")
        .num(round(a.dppm, 1));
    o.raw(",\"drt\":").num(round(a.drt, 3));
    o.raw(",\"tol\":")
        .num(round(a.tol.da, 3))
        .raw(",\"tol_window\":")
        .raw(if a.tol.from_window { "true" } else { "false" });
    o.raw(",\"n_in\":")
        .int(a.n_in as i64)
        .raw(",\"others\":")
        .int(a.n_in as i64 - 1);
}

/// `"ms2_rep"` and `"ms2_text"` of `api.detail` (written after a comma): the representative scan with its association and spectrum and the sentence
/// of the fragments found; null and «No MS² in the feature peak» when the row has no scan inside the peak of its feature.
pub fn rep_json<R: ReadAt>(
    o: &mut Out,
    d: &LrDig,
    e: &Entry,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<()> {
    let Some(r) = best(d, e, open)? else {
        o.raw(",\"ms2_rep\":null,\"ms2_text\":").str(NO_MS2);
        return Ok(());
    };
    let x = d
        .links
        .as_ref()
        .map(|l| &l.exps[r.exp])
        .ok_or("no matches")?;
    let mut ix = open(r.file)?;
    let (rt, ce) = (ix.scans[r.scan].rt, ix.scans[r.scan].collision_energy);
    let prec = ix.scans[r.scan].prec(x.precursor);
    let peaks = read_scans(&mut ix, &[r.scan])
        .map_err(|x| x.to_string())?
        .into_iter()
        .next()
        .unwrap_or_default();
    let neutral = if e.formula.is_empty() {
        &d.parent.formula
    } else {
        &e.formula
    };
    let ions: Vec<Ion> = peaks
        .0
        .iter()
        .zip(&peaks.1)
        .map(|(m, i)| Ion {
            mz: *m,
            intensity: *i,
            formula: None,
        })
        .collect();
    let found = ms2frag::find(
        prec,
        neutral,
        &ions,
        d.settings.frag_min_rel,
        ms2frag::LOSS_TOL_LR,
    );
    o.raw(",\"ms2_rep\":{\"file\":").str(&x.file);
    o.raw(",\"rt\":")
        .num(round(rt, 4))
        .raw(",\"precursor\":")
        .num(round(prec, 4));
    o.raw(",\"ce\":").opt(ce.filter(|c| *c != 0.0));
    assoc_json(o, &r);
    o.raw(",\"mz\":")
        .rounded(&peaks.0, 4)
        .raw(",\"int\":")
        .floats(&peaks.1);
    found_json(o, &found);
    o.raw("},\"ms2_text\":").str(&ms2frag::text(&found));
    Ok(())
}
