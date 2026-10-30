//! TIC and XIC over parsed spectra, as `Item.total` / `PeakTable.xic` of the Python engine (the sums are done in `f64`).

use crate::mzml::Spectrum;

/// (RT in minutes, summed intensity) of the scans of one MS level; scans without peaks count 0.
pub fn tic<'a>(spectra: impl IntoIterator<Item = &'a Spectrum>, level: u8) -> (Vec<f64>, Vec<f64>) {
    spectra
        .into_iter()
        .filter(|s| s.level == level)
        .map(|s| (s.rt, s.intensity.iter().map(|&v| v as f64).sum::<f64>()))
        .unzip()
}

/// Sum of the intensities with `lo <= m/z <= hi` in every scan of one level (the peaks of a scan are sorted by m/z or not: both work).
pub fn xic<'a>(
    spectra: impl IntoIterator<Item = &'a Spectrum>,
    level: u8,
    lo: f64,
    hi: f64,
) -> (Vec<f64>, Vec<f64>) {
    spectra
        .into_iter()
        .filter(|s| s.level == level)
        .map(|s| {
            let y: f64 =
                s.mz.iter()
                    .zip(&s.intensity)
                    .filter(|(m, _)| **m >= lo && **m <= hi)
                    .map(|(_, &i)| i as f64)
                    .sum();
            (s.rt, y)
        })
        .unzip()
}

/// True when the Python engine reads the file with the "low" mass profile (unit resolution: m/z with 3 decimals, centroids binned by
/// 0.1 Da): no Orbitrap or time-of-flight component in the instrument and no Thermo filter string starting with FTMS or TOFMS.
/// (Python also calls "high resolution" an unknown analyzer whose scans declare a resolving power of 10000 or more; the reader does
/// not keep that number, so such a file is not recognised here.)
pub fn is_unit_resolution<'a>(
    analyzers: &[String],
    spectra: impl IntoIterator<Item = &'a Spectrum>,
) -> bool {
    let hr_part = analyzers.iter().any(|a| {
        a.contains("orbitrap")
            || a.contains("time-of-flight")
            || a.contains("time of flight")
            || a.contains("cyclotron")
    });
    let hr_filter = spectra.into_iter().any(|s| {
        let t = s.filter.split(' ').next().unwrap_or("").to_uppercase();
        t == "FTMS" || t == "TOFMS"
    });
    !hr_part && !hr_filter
}

/// Bins of `bin_da` Da, as `Item.scans` does for unit-resolution centroids: peaks with the same `floor(m/z / bin_da)` merge into one
/// (summed intensity, intensity-weighted m/z), in increasing cell order.
pub fn bin_centroids(mz: &[f64], inten: &[f32], bin_da: f64) -> (Vec<f64>, Vec<f64>) {
    use std::collections::BTreeMap;
    let mut cells: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for (&m, &i) in mz.iter().zip(inten) {
        let c = cells
            .entry((m / bin_da).floor() as i64)
            .or_insert((0.0, 0.0));
        c.0 += i as f64;
        c.1 += i as f64 * m;
    }
    cells
        .values()
        .map(|&(sy, smy)| (smy / sy.max(1e-12), sy))
        .unzip()
}

/// At most two points per pixel column (the lowest and the highest y of the column, in x order) for drawing; `x` is increasing.
/// Returns the kept indices. With `px == 0` or few points everything is kept.
pub fn minmax_indices(x: &[f64], y: &[f64], px: usize) -> Vec<usize> {
    let n = x.len();
    if px == 0 || n <= 2 * px {
        return (0..n).collect();
    }
    let (x0, x1) = (x[0], x[n - 1]);
    let span = (x1 - x0).max(1e-300);
    let col = |v: f64| (((v - x0) / span * px as f64) as usize).min(px - 1);
    let mut out = Vec::with_capacity(2 * px);
    let mut i = 0;
    while i < n {
        let c = col(x[i]);
        let (mut lo, mut hi) = (i, i);
        let mut j = i;
        while j < n && col(x[j]) == c {
            if y[j] < y[lo] {
                lo = j;
            }
            if y[j] > y[hi] {
                hi = j;
            }
            j += 1;
        }
        let (a, b) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        out.push(a);
        if b != a {
            out.push(b);
        }
        i = j;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_resolution_is_everything_but_orbitrap_and_tof() {
        let q = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let none: [&Spectrum; 0] = [];
        assert!(is_unit_resolution(&q(&["quadrupole", "quadrupole", "ion trap"]), none));
        assert!(is_unit_resolution(&[], none));
        assert!(!is_unit_resolution(&q(&["orbitrap"]), none));
        assert!(!is_unit_resolution(&q(&["quadrupole", "time-of-flight"]), none));
        let s = Spectrum { filter: "FTMS + p ESI Full ms".into(), ..Default::default() };
        assert!(!is_unit_resolution(&[], [&s]));
    }

    #[test]
    fn binning_merges_cells_with_weighted_mz() {
        let (m, y) = bin_centroids(&[100.01, 100.05, 100.31], &[1.0, 3.0, 2.0], 0.1);
        assert_eq!(y, vec![4.0, 2.0]);
        assert!((m[0] - (100.01 + 3.0 * 100.05) / 4.0).abs() < 1e-9);
        assert!((m[1] - 100.31).abs() < 1e-9);
    }

    #[test]
    fn minmax_keeps_extremes_and_bounds() {
        let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let mut y = vec![1.0; 1000];
        y[503] = 99.0;
        y[17] = -5.0;
        let k = minmax_indices(&x, &y, 10);
        assert!(k.len() <= 20 && k.contains(&503) && k.contains(&17));
        assert!(k.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(minmax_indices(&x, &y, 0).len(), 1000);
    }
}
