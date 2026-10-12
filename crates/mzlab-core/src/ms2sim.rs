//! Similarity of the product-ion spectra of an ion X and of a candidate precursor P: ports of `ionfamily.ms2_similarity` and
//! `ionfamily.ms2_membership` (cosine, modified cosine, subset score, "X among the products of P"). Compatible, not proven.

use crate::json::Value;
use crate::jsonb::{int, Obj};
use crate::stats::psum;
use crate::tracestat::argmin;

/// Global greedy one-to-one matching of the peaks of `a` to those of `b` (heaviest product of intensities first, ties by position).
/// A pair is a candidate when `|b - a - d| <= tol` for `d` in `shifts`. Returns `(ia, ib)` pairs, heaviest first.
fn matching(
    a: &[f64],
    ya: &[f64],
    b: &[f64],
    yb: &[f64],
    tol: f64,
    shifts: &[f64],
) -> Vec<(usize, usize)> {
    let mut cand: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..a.len() {
        for j in 0..b.len() {
            if shifts.iter().any(|d| (b[j] - a[i] - d).abs() <= tol) {
                cand.push((i, j, ya[i] * yb[j]));
            }
        }
    }
    cand.sort_by(|x, y| y.2.total_cmp(&x.2));
    let (mut ua, mut ub) = (vec![false; a.len()], vec![false; b.len()]);
    let mut out = Vec::new();
    for (i, j, _) in cand {
        if !ua[i] && !ub[j] {
            ua[i] = true;
            ub[j] = true;
            out.push((i, j));
        }
    }
    out
}

fn norm(v: &[f64]) -> f64 {
    psum(&v.iter().map(|x| x * x).collect::<Vec<_>>()).sqrt()
}

/// Spectra are `(m/z, intensity)` slices. `mz_cut` excludes P's own precursor peak when looking for X among its products.
pub fn ms2_similarity(
    mz_x: f64,
    spec_x: (&[f64], &[f64]),
    mz_p: f64,
    spec_p: (&[f64], &[f64]),
) -> Value {
    let (tol, mz_cut) = (0.5, 1.5);
    let ((ax, ix), (ap, ip)) = (spec_x, spec_p);
    let mut out = Obj::new()
        .i("n_x", ax.len() as i64)
        .i("n_p", ap.len() as i64);
    if ax.is_empty() || ap.is_empty() {
        return out
            .n("cosine", f64::NAN)
            .n("modified_cosine", f64::NAN)
            .b("x_in_p", false)
            .n("subset_score", f64::NAN)
            .done();
    }
    let sx: Vec<f64> = ix.iter().map(|v| v.sqrt()).collect();
    let sp: Vec<f64> = ip.iter().map(|v| v.sqrt()).collect();
    let den = norm(&sx) * norm(&sp);
    let sum = |pairs: &[(usize, usize)]| pairs.iter().fold(0.0, |s, &(i, j)| s + sx[i] * sp[j]);
    let pairs0 = matching(ax, &sx, ap, &sp, tol, &[0.0]);
    let pairs = matching(ax, &sx, ap, &sp, tol, &[0.0, mz_p - mz_x]);
    let ratio = |v: f64| if den > 0.0 { v / den } else { f64::NAN };
    out = out
        .n("cosine", ratio(sum(&pairs0)))
        .n("modified_cosine", ratio(sum(&pairs)));
    let in_p = ap
        .iter()
        .filter(|m| (*m - mz_p).abs() > mz_cut)
        .any(|m| (m - mz_x).abs() <= tol);
    let found: f64 = pairs.iter().fold(0.0, |s, &(i, _)| s + sx[i] * sx[i]);
    let tot = psum(&sx.iter().map(|v| v * v).collect::<Vec<_>>());
    out.b("x_in_p", in_p)
        .n("subset_score", found / tot.max(1e-300))
        .done()
}

/// When only the product-ion spectrum of P was acquired: is there a product of P at the m/z of X (not P's own precursor peak)?
pub fn ms2_membership(mz_x: f64, mz_p: f64, spec_p: (&[f64], &[f64])) -> Value {
    let (tol, mz_cut) = (0.5, 1.5);
    let keep: Vec<usize> = (0..spec_p.0.len())
        .filter(|&i| (spec_p.0[i] - mz_p).abs() > mz_cut)
        .collect();
    if keep.is_empty() {
        return Obj::new().b("x_in_p", false).i("n_p", 0).done();
    }
    let ap: Vec<f64> = keep.iter().map(|&i| spec_p.0[i]).collect();
    let ip: Vec<f64> = keep.iter().map(|&i| spec_p.1[i]).collect();
    let j = argmin(&ap.iter().map(|v| (v - mz_x).abs()).collect::<Vec<_>>());
    let imax = ip.iter().cloned().fold(f64::MIN, f64::max);
    Obj::new()
        .b("x_in_p", (ap[j] - mz_x).abs() <= tol)
        .n("nearest_product", ap[j])
        .n("delta", ap[j] - mz_x)
        .n("rel_intensity", 100.0 * ip[j] / imax.max(1e-300))
        .put("n_p", int(ap.len() as i64))
        .done()
}
