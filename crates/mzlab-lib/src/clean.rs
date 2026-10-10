//! Cleaning of a spectrum as Flash entropy search does it (Li & Fiehn, Nat. Methods 2023): ions at or above the precursor − 1.6 Da are removed,
//! then the noise (< 1% of the highest peak), then peaks closer than 0.05 Da are merged (the most intense one takes the neighbours: m/z is the
//! intensity-weighted mean, intensity the sum), then at most `max_peaks` are kept. Intensities are not normalised here.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CleanOpts {
    /// Peaks with m/z ≥ precursor − this are removed (0 or negative: nothing removed).
    pub precursor_removal_da: f64,
    pub noise: f64,
    pub centroid_da: f64,
    pub max_peaks: usize,
}

impl Default for CleanOpts {
    fn default() -> Self {
        CleanOpts {
            precursor_removal_da: 1.6,
            noise: 0.01,
            centroid_da: 0.05,
            max_peaks: MAX_PEAKS,
        }
    }
}

/// At most this many peaks per library spectrum.
pub const MAX_PEAKS: usize = 500;
/// m/z is stored in units of 1e-5 Da (`u32`): the error is at most 5e-6 Da.
pub const MZ_SCALE: f64 = 1e5;
/// Largest m/z the format can hold.
pub const MAX_MZ: f64 = 40_000.0;

/// Cleans `(mz, it)`; the result is sorted by m/z.
pub fn clean(prec: f64, mz: &[f64], it: &[f64], o: &CleanOpts) -> (Vec<f64>, Vec<f64>) {
    let cut = if o.precursor_removal_da > 0.0 && prec > 0.0 {
        prec - o.precursor_removal_da
    } else {
        f64::INFINITY
    };
    let mut p: Vec<(f64, f64)> = mz
        .iter()
        .zip(it)
        .filter(|(m, i)| {
            m.is_finite() && i.is_finite() && **m > 0.0 && **m < MAX_MZ && **i > 0.0 && **m < cut
        })
        .map(|(m, i)| (*m, *i))
        .collect();
    p.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mx = p.iter().fold(0.0f64, |a, x| a.max(x.1));
    p.retain(|x| x.1 >= o.noise * mx);
    if o.centroid_da > 0.0 {
        p = centroid(&p, o.centroid_da);
    }
    if p.len() > o.max_peaks {
        p.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.total_cmp(&b.0)));
        p.truncate(o.max_peaks);
        p.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    p.into_iter().unzip()
}

/// Merges peaks within `tol` of the most intense remaining one (a peak joins one seed only). Input sorted by m/z.
fn centroid(p: &[(f64, f64)], tol: f64) -> Vec<(f64, f64)> {
    let n = p.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| p[b].1.total_cmp(&p[a].1).then(a.cmp(&b)));
    let mut used = vec![false; n];
    let mut out = Vec::with_capacity(n);
    for &s in &order {
        if used[s] {
            continue;
        }
        used[s] = true;
        let (mut wm, mut sum) = (p[s].0 * p[s].1, p[s].1);
        let mut j = s;
        while j > 0 && p[s].0 - p[j - 1].0 <= tol {
            j -= 1;
            if !used[j] {
                used[j] = true;
                wm += p[j].0 * p[j].1;
                sum += p[j].1;
            }
        }
        let mut j = s + 1;
        while j < n && p[j].0 - p[s].0 <= tol {
            if !used[j] {
                used[j] = true;
                wm += p[j].0 * p[j].1;
                sum += p[j].1;
            }
            j += 1;
        }
        out.push((wm / sum, sum));
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

pub fn entropy_of(p: &[f64]) -> f64 {
    -p.iter()
        .filter(|&&v| v > 0.0)
        .map(|&v| v * v.ln())
        .sum::<f64>()
}

/// Intensities normalised to sum 1; if the spectral entropy is below 3 they are raised to 0.25 + 0.25 S and normalised again (Li 2021).
pub fn weighted(it: &[f64]) -> Vec<f64> {
    let s: f64 = it.iter().sum();
    if s.is_nan() || s <= 0.0 {
        return vec![0.0; it.len()];
    }
    let mut p: Vec<f64> = it.iter().map(|v| v / s).collect();
    let big_s = entropy_of(&p);
    if big_s < 3.0 {
        let w = 0.25 + 0.25 * big_s;
        for v in p.iter_mut() {
            *v = v.powf(w);
        }
        let t: f64 = p.iter().sum();
        for v in p.iter_mut() {
            *v /= t;
        }
    }
    p
}

/// m/z in 1e-5 Da units (saturating at the format's limit).
pub fn quant_mz(mz: f64) -> u32 {
    (mz * MZ_SCALE).round().clamp(0.0, u32::MAX as f64) as u32
}

pub fn dequant_mz(q: u32) -> f64 {
    q as f64 / MZ_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_the_reference_cleaning() {
        // values computed with ms_entropy.clean_spectrum(noise_threshold=0.01, min_ms2_difference_in_da=0.05)
        let (mz, it) = clean(
            1000.0,
            &[100.0, 100.03, 100.2, 100.24, 100.27, 300.0],
            &[10.0, 20.0, 5.0, 5.0, 8.0, 1.0],
            &CleanOpts::default(),
        );
        assert_eq!(mz.len(), 4);
        assert!((mz[0] - 100.02).abs() < 1e-9 && (it[0] - 30.0).abs() < 1e-9);
        assert!((mz[1] - 100.2).abs() < 1e-9 && (it[1] - 5.0).abs() < 1e-9);
        assert!((mz[2] - 100.258_461_538).abs() < 1e-6 && (it[2] - 13.0).abs() < 1e-9);
        assert_eq!((mz[3], it[3]), (300.0, 1.0));
    }

    #[test]
    fn precursor_noise_and_the_cap() {
        let o = CleanOpts::default();
        let (mz, _) = clean(
            500.0,
            &[100.0, 498.5, 499.0, 600.0],
            &[10.0, 10.0, 10.0, 10.0],
            &o,
        );
        assert_eq!(mz, [100.0], "ions at or above precursor - 1.6 are removed");
        let (mz, _) = clean(0.0, &[100.0, 200.0], &[1000.0, 5.0], &o);
        assert_eq!(mz, [100.0], "below 1% of the highest");
        let m: Vec<f64> = (0..2000).map(|i| 100.0 + i as f64).collect();
        let it: Vec<f64> = (0..2000)
            .map(|i| 1.0 + (i * 37 % 1000) as f64 / 1000.0)
            .collect();
        let (mz, it) = clean(0.0, &m, &it, &o);
        assert_eq!(mz.len(), 500);
        assert!(mz.windows(2).all(|w| w[0] < w[1]) && it.iter().all(|v| *v > 0.0));
    }

    #[test]
    fn entropy_weighting() {
        let p = weighted(&[1.0, 1.0]);
        assert!((p[0] - 0.5).abs() < 1e-12);
        let w = weighted(&[3.0, 1.0]);
        assert!(
            (w.iter().sum::<f64>() - 1.0).abs() < 1e-12 && w[0] > w[1] && w[0] < 0.75,
            "weighting flattens"
        );
        let big: Vec<f64> = (0..100).map(|_| 1.0).collect();
        assert!(
            (weighted(&big)[0] - 0.01).abs() < 1e-12,
            "entropy >= 3: no weighting"
        );
    }
}
