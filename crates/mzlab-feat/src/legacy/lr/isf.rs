//! In-source-fragment evidence of the rows of a low-resolution Dig (`engine._isf_eval`): for the ions that co-elute with the parent, the verdict of
//! `lrisf::isf_eval` over the full-scan files that are not blanks. It is evidence, not a conclusion about the sample: a row without a verdict falls
//! back to the plain "lighter and co-eluting" heuristic (`summary`).

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;

use super::LrDig;
use crate::legacy::lrisf::{isf_eval, IsfSample, IsfVerdict};
use crate::legacy::Res;

/// At most this many ions are evaluated.
const MAX_IONS: usize = 30;
/// An ion heavier than the parent by more than this can only be an adduct (kept for the isotope/adduct split), Da.
const MAX_HEAVIER: f64 = 4.0;

/// The entries evaluated: the ones that elute within twice the tolerance of RT of the parent and are not much heavier, in table order.
fn candidates(d: &LrDig) -> Vec<usize> {
    let Some(par) = d.entries.first() else {
        return Vec::new();
    };
    let Some(p_rt) = par.ref_rt.filter(|t| *t != 0.0) else {
        return Vec::new();
    };
    (1..d.entries.len())
        .filter(|&i| {
            let e = &d.entries[i];
            e.ref_rt
                .is_some_and(|t| t != 0.0 && (t - p_rt).abs() <= 2.0 * d.settings.rt_tol_min)
                && e.mz < par.mz + MAX_HEAVIER
        })
        .take(MAX_IONS)
        .collect()
}

/// `(entry id, verdict)` for the entries that have one; `open(k)` gives the index of column `k`. Never fails on a verdict: an ion that cannot be analysed has none.
pub fn evaluate<R: ReadAt>(
    d: &LrDig,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Vec<(i64, IsfVerdict)>> {
    let cand = candidates(d);
    if cand.is_empty() {
        return Ok(Vec::new());
    }
    let mut idxs = Vec::new();
    for (k, f) in d.lc.iter().enumerate() {
        if f.kind != "blank" {
            idxs.push((k, open(k)?));
        }
    }
    let mut samples: Vec<IsfSample<R>> = Vec::new();
    for (k, idx) in idxs.iter_mut() {
        if let Some(si) = idx.section_of(1, None) {
            let f = &d.lc[*k];
            samples.push(IsfSample {
                label: f.label.clone(),
                time: f.time,
                idx,
                si,
            });
        }
    }
    let par = &d.entries[0];
    let ions: Vec<f64> = cand.iter().map(|&i| d.entries[i].mz_x).collect();
    let formula = Some(d.parent.formula.as_str()).filter(|f| !f.is_empty());
    let got = isf_eval(&mut samples, par.mz_x, &ions, formula);
    Ok(cand
        .into_iter()
        .zip(got)
        .filter_map(|(i, v)| v.map(|v| (d.entries[i].id, v)))
        .collect())
}

/// The `isf` object of a row.
pub fn verdict_json(o: &mut Out, v: &IsfVerdict) {
    o.raw("{\"p_inherited\":")
        .num(v.p_inherited)
        .raw(",\"isf_ness\":")
        .num(v.isf_ness)
        .raw(",\"probs\":{");
    for (i, (k, p)) in v.probs.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").num(*p);
    }
    o.raw("},\"doubtful\":")
        .raw(if v.doubtful { "true" } else { "false" })
        .raw(",\"reasons\":[");
    for (i, r) in v.reasons.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(r);
    }
    o.raw("],\"explanation\":")
        .str(&v.explanation)
        .raw(",\"calibrated\":")
        .raw(if v.calibrated { "true" } else { "false" })
        .raw("}");
}

/// The probability that the ion is an in-source fragment of the parent.
pub fn p_isf(v: &IsfVerdict) -> f64 {
    v.probs
        .iter()
        .find(|(k, _)| k == "isf")
        .map_or(0.0, |p| p.1)
}
