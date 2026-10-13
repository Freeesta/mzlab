//! The spectra and the statistics behind the evidence of an edge (`hr/identity.py`): top peaks, matching peaks, ratios over the samples, shared skeleton.

use super::*;

pub(super) fn tol(mz: f64) -> f64 {
    ABS_DA.max(mz.abs() * PPM * 1e-6)
}

/// A spectrum of the payload: (m/z, intensity) of the strongest `MAX_PEAKS` peaks.
pub type Spec = (Vec<f64>, Vec<f64>);

/// `_spec`: None without peaks; above `MAX_PEAKS` the strongest are kept (strongest first, as `np.argsort(-i)`).
pub fn spec(mz: &[f64], it: &[f64]) -> Option<Spec> {
    if mz.is_empty() || it.is_empty() {
        return None;
    }
    if mz.len() <= MAX_PEAKS {
        return Some((mz.to_vec(), it.to_vec()));
    }
    let mut k: Vec<usize> = (0..it.len()).collect();
    k.sort_by(|&a, &b| it[b].total_cmp(&it[a]));
    k.truncate(MAX_PEAKS);
    Some((
        k.iter().map(|&j| mz[j]).collect(),
        k.iter().map(|&j| it[j]).collect(),
    ))
}

/// `_near`: the closest peak to `mz` within the tolerance, the precursor's own peak excluded: (m/z, % of the strongest kept peak).
pub(super) fn near(s: &Spec, mz: f64, prec: Option<f64>) -> Option<(f64, f64)> {
    let keep: Vec<(f64, f64)> =
        s.0.iter()
            .zip(&s.1)
            .filter(|(a, _)| prec.is_none_or(|p| (*a - p).abs() > 1.5 * tol(p)))
            .map(|(a, i)| (*a, *i))
            .collect();
    if keep.is_empty() {
        return None;
    }
    let mut j = 0;
    for (k, p) in keep.iter().enumerate() {
        if (p.0 - mz).abs() < (keep[j].0 - mz).abs() {
            j = k;
        }
    }
    let top = keep.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    ((keep[j].0 - mz).abs() <= tol(mz)).then(|| (keep[j].0, 100.0 * keep[j].1 / top.max(1e-300)))
}

pub(super) fn peaks_besides(s: &Spec, prec: f64) -> usize {
    s.0.iter()
        .filter(|a| (*a - prec).abs() > 1.5 * tol(prec))
        .count()
}

/// `numpy.mean`.
pub fn mean(v: &[f64]) -> f64 {
    v.iter().fold(0.0, |a, b| a + b) / v.len() as f64
}

/// `numpy.std(ddof)`.
pub fn std(v: &[f64], ddof: usize) -> f64 {
    let mu = mean(v);
    (v.iter()
        .map(|x| (x - mu) * (x - mu))
        .fold(0.0, |a, b| a + b)
        / (v.len() - ddof) as f64)
        .sqrt()
}

pub(super) struct RatioStats {
    pub(super) n: usize,
    pub(super) cv: Option<f64>,
}

/// `ratio_stats`: the coefficient of variation of the ratio heavy/light over the samples where both are positive (`ionfamily.ratio_constancy`).
pub(super) fn ratio_stats(lo: &[f64], up: &[f64]) -> RatioStats {
    let r: Vec<f64> = lo
        .iter()
        .zip(up)
        .filter(|(a, b)| **a > 0.0 && **b > 0.0)
        .map(|(a, b)| b / a)
        .filter(|x| x.is_finite())
        .collect();
    let n = lo
        .iter()
        .zip(up)
        .filter(|(a, b)| **a > 0.0 && **b > 0.0)
        .count();
    if n < MIN_SAMPLES {
        return RatioStats { n, cv: None };
    }
    let mu = mean(&r);
    let cv = if r.len() < 2 || mu == 0.0 {
        f64::NAN
    } else {
        std(&r, 1) / mu.abs()
    };
    RatioStats {
        n,
        cv: cv.is_finite().then_some(cv),
    }
}

/// `skeleton`: (fragments of X, shared with Y, weighted fraction).
pub(super) fn skeleton(
    lo: &Spec,
    up: &Spec,
    mz_lo: f64,
    mz_up: f64,
) -> (usize, usize, Option<f64>) {
    let shift = mz_up - mz_lo;
    let (mut tot, mut shared, mut n, mut n_sh) = (0.0, 0.0, 0, 0);
    for (&a, &i) in lo.0.iter().zip(&lo.1) {
        if (a - mz_lo).abs() <= 1.5 * tol(mz_lo) {
            continue;
        }
        let w = i.sqrt();
        tot += w;
        n += 1;
        let hit = |c: f64| {
            up.0.iter()
                .map(|b| (b - c).abs())
                .fold(f64::INFINITY, f64::min)
                <= tol(a)
        };
        if !up.0.is_empty() && (hit(a) || hit(a + shift)) {
            shared += w;
            n_sh += 1;
        }
    }
    (n, n_sh, (tot > 0.0).then(|| shared / tot))
}
