//! GNPS-style cosine and its matching on prepared peaks (windowed over sorted m/z).

use super::{ascending, window_of, Spec};
use mzlab_picchi::sortc;

#[derive(Debug, Clone, PartialEq)]
pub struct GnpsMatch {
    pub cosine: f64,
    pub reverse: f64,
    pub matched: usize,
    pub n_shifted: usize,
    /// `(index in the cleaned sample, index in the cleaned reference, shifted)`.
    pub pairs: Vec<(usize, usize, bool)>,
}

/// Peaks at or above `precursor - 0.5` and below 1% of the base peak are removed; intensities become square roots.
pub fn gnps_prepare(mz: &[f64], it: &[f64], prec: f64) -> (Vec<f64>, Vec<f64>) {
    let mx = it.iter().cloned().fold(0.0, f64::max);
    let (mut m, mut v) = (Vec::new(), Vec::new());
    for (x, y) in mz.iter().zip(it) {
        if *y > 0.0 && *x < prec - 0.5 && *y >= 0.01 * mx {
            m.push(*x);
            v.push(y.sqrt());
        }
    }
    (m, v)
}

/// The matching of `gnps_cosine` on spectra already prepared (`gnps_prepare`: the intensities are the square roots) and an explicit shift `delta`
/// (sample precursor minus reference precursor): callers that clean the peaks their own way, or score against a precursor that is not the reference's, use this.
pub fn gnps_match_prepared(
    am: &[f64],
    ai: &[f64],
    bm: &[f64],
    bi: &[f64],
    delta: f64,
    tol: f64,
    modified: bool,
) -> GnpsMatch {
    let mut c: Vec<(usize, usize, f64, bool)> = Vec::new();
    let shifting = modified && delta.abs() > tol;
    if ascending(bm) {
        // The ordering below is total (product, unshifted first, then the indices), so the candidates need not come in any order.
        for (i, &x) in am.iter().enumerate() {
            let w1 = window_of(bm, x, 0.0, tol);
            for j in w1.clone() {
                c.push((i, j, ai[i] * bi[j], false));
            }
            if shifting {
                // a reference peak at `m` pairs with the sample peak at `m + delta` when `x - (m + delta)` is within `tol`
                let lo = bm.partition_point(|&m| x - (m + delta) > tol);
                let hi = bm.partition_point(|&m| x - (m + delta) >= -tol);
                for j in (lo..hi.max(lo)).filter(|j| !w1.contains(j)) {
                    c.push((i, j, ai[i] * bi[j], true));
                }
            }
        }
    } else {
        for i in 0..am.len() {
            for j in 0..bm.len() {
                if (am[i] - bm[j]).abs() <= tol {
                    c.push((i, j, ai[i] * bi[j], false));
                } else if shifting && (am[i] - (bm[j] + delta)).abs() <= tol {
                    c.push((i, j, ai[i] * bi[j], true));
                }
            }
        }
    }
    sortc::by(&mut c, |x, y| {
        y.2.total_cmp(&x.2)
            .then(x.3.cmp(&y.3))
            .then(y.0.cmp(&x.0))
            .then(y.1.cmp(&x.1))
    });
    let (mut ua, mut ub) = (vec![false; am.len()], vec![false; bm.len()]);
    let (mut dot, mut na_m, mut n_shifted) = (0.0, 0.0, 0usize);
    let mut pairs = Vec::new();
    for (i, j, w, sh) in c {
        if !ua[i] && !ub[j] {
            ua[i] = true;
            ub[j] = true;
            dot += w;
            na_m += ai[i] * ai[i];
            pairs.push((i, j, sh));
            if sh {
                n_shifted += 1;
            }
        }
    }
    let na: f64 = ai.iter().map(|v| v * v).sum::<f64>().sqrt();
    let nb: f64 = bi.iter().map(|v| v * v).sum::<f64>().sqrt();
    let ratio = |d: f64, x: f64, y: f64| if x > 0.0 && y > 0.0 { d / (x * y) } else { 0.0 };
    GnpsMatch {
        cosine: ratio(dot, na, nb),
        reverse: ratio(dot, na_m.sqrt(), nb),
        matched: pairs.len(),
        n_shifted,
        pairs,
    }
}

/// GNPS cosine of a sample against a reference with an absolute tolerance in Da. `modified`: a reference peak at `m` may also pair with a sample
/// peak at `m + delta`, `delta = prec(sample) - prec(reference)`. Ties in the product: the unshifted pair first, then the higher indices.
pub fn gnps_cosine(sample: &Spec, reference: &Spec, tol: f64, modified: bool) -> GnpsMatch {
    let (am, ai) = gnps_prepare(&sample.mz, &sample.it, sample.prec);
    let (bm, bi) = gnps_prepare(&reference.mz, &reference.it, reference.prec);
    gnps_match_prepared(
        &am,
        &ai,
        &bm,
        &bi,
        sample.prec - reference.prec,
        tol,
        modified,
    )
}
