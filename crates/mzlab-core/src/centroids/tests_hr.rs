//! The width estimate on continuous high-resolution profile scans: narrow peaks are measured, not given the fallback, and too few
//! isolated peaks give the fallback with no peak count (so the step never calls it estimated).

use super::tests::Rng;
use super::*;

/// Continuous high-resolution profile scans: Gaussian peaks of FWHM `fwhm` on a 1.5 mDa grid with noise of a few units.
fn hr_scans(fwhm: f64, peaks: usize) -> Vec<(Vec<f64>, Vec<f64>)> {
    let mut rng = Rng(11);
    let sd = fwhm / 2.3548;
    (0..8)
        .map(|_| {
            let mz: Vec<f64> = (0..6000).map(|k| 300.0 + 0.0015 * k as f64).collect();
            let y = mz
                .iter()
                .map(|&m| {
                    let s: f64 = (0..peaks)
                        .map(|j| {
                            let c = 300.2 + 8.5 * j as f64 / peaks as f64;
                            (2000.0 + 300.0 * j as f64) * (-0.5 * ((m - c) / sd).powi(2)).exp()
                        })
                        .sum();
                    s + 20.0 + 6.0 * rng.next()
                })
                .collect();
            (mz, y)
        })
        .collect()
}

#[test]
fn on_high_resolution_profile_the_width_is_measured_not_the_fallback() {
    for fwhm in [0.006, 0.012] {
        let scans = hr_scans(fwhm, 24);
        let refs: Vec<(&[f64], &[f64])> = scans
            .iter()
            .map(|(a, b)| (a.as_slice(), b.as_slice()))
            .collect();
        let e = estimate(&refs, 0.7);
        assert_eq!(e.params.mode, Mode::Continuous);
        assert!(e.peaks >= 8, "{fwhm}: {e:?}");
        assert!((e.params.w / fwhm - 1.0).abs() < 0.35, "{fwhm}: {e:?}");
    }
}

#[test]
fn too_few_isolated_peaks_give_the_fallback_with_no_peak_count() {
    let scans = hr_scans(0.006, 3);
    let refs: Vec<(&[f64], &[f64])> = scans[..1]
        .iter()
        .map(|(a, b)| (a.as_slice(), b.as_slice()))
        .collect();
    let e = estimate(&refs, 0.7);
    assert_eq!((e.peaks, e.params.w), (0, 0.7), "{e:?}");
}
