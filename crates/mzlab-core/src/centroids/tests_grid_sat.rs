//! Centroids of high-resolution profile scans where the review found them biased (H4-C, H4-D): an irregular grid whose spacing grows with m/z (no zero
//! inside a peak at high m/z), a saturated flat top (one centroid at the centre, its height and the SATURATED flag), and the tallest unsaturated peak of a
//! densely sampled file (not taken for saturated).

use super::tests::Rng;
use super::*;

fn gauss(m: f64, c: f64, h: f64, fwhm: f64) -> f64 {
    h * (-4.0 * std::f64::consts::LN_2 * ((m - c) / fwhm).powi(2)).exp()
}

fn hr(step: f64, w: f64, vmax: f64) -> Params {
    Params {
        mode: Mode::Continuous,
        unit: 1.0,
        step,
        w,
        k: 3.0,
        vmax,
        area: false,
    }
}

#[test]
fn an_orbitrap_like_grid_keeps_its_peaks_whole_at_high_mz() {
    let mut rng = Rng(3);
    // spacing 0.8 mDa at m/z 400 growing as (m/400)^1.5: at m/z 850 it is 3.1 x the spacing at 400, over 1.5 x the median of the scan
    let (mut mz, mut y) = (Vec::new(), Vec::new());
    let centres: Vec<f64> = (0..12)
        .map(|j| 840.0 + 1.0 * j as f64 + 0.3 * rng.next())
        .collect();
    let mut m = 395.0;
    while m < 853.0 {
        let v: f64 = centres
            .iter()
            .map(|&c| gauss(m, c, 1e5, c / 60_000.0))
            .sum();
        if v > 50.0 || m < 450.0 {
            mz.push(m);
            y.push(v + 20.0 * rng.next());
        }
        m += 0.0008 * (m / 400.0).powf(1.5);
    }
    let cs = centroids(
        &mz,
        &y,
        &hr(0.00105, 850.0 / 60_000.0, 0.0),
        &mut Buf::default(),
    );
    for c in centres {
        let x = cs
            .iter()
            .min_by(|a, b| (a.mz - c).abs().total_cmp(&(b.mz - c).abs()))
            .unwrap();
        assert!(
            ((x.mz - c) / c * 1e6).abs() < 0.05,
            "{c}: {} ({} ppm)",
            x.mz,
            (x.mz - c) / c * 1e6
        );
        // the smoothed apex of a peak sampled 6 times per FWHM is about 0.93 of its height; zeros inside the peak leave about 0.37
        assert!(x.value > 0.85e5, "{c}: height {}", x.value);
    }
}

fn flat_top(mult: f64, ppf: f64, rng: &mut Rng) -> (f64, Vec<Centroid>) {
    let (c, fw) = (500.0 + rng.next(), 0.025);
    let st = fw / ppf;
    let (mut mz, mut y) = (Vec::new(), Vec::new());
    let mut m = c - 3.0 * fw + rng.next() * st;
    while m < c + 3.0 * fw {
        mz.push(m);
        y.push(gauss(m, c, mult * 1e5, fw).min(1e5) + 10.0);
        m += st;
    }
    let vmax = if mult <= 1.0 {
        y.iter().copied().fold(0.0, f64::max)
    } else {
        1e5 + 10.0
    };
    (
        c,
        centroids(&mz, &y, &hr(st, fw, vmax), &mut Buf::default()),
    )
}

#[test]
fn a_saturated_peak_is_one_centroid_at_its_centre_with_its_height() {
    let mut rng = Rng(5);
    for mult in [1.5, 3.0, 10.0] {
        for _ in 0..40 {
            let (c, mut cs) = flat_top(mult, 10.0, &mut rng);
            // the clipped tails leave bumps of a few counts over the offset; the peak is what is above 1 % of the ceiling
            cs.retain(|x| x.value > 1e3);
            assert_eq!(cs.len(), 1, "x{mult}: {cs:?}");
            let x = &cs[0];
            assert!(x.flags & SATURATED != 0, "x{mult}: {x:?}");
            // 1 FWHM is 50 ppm here: the centre stays within 0.4 FWHM of the truth, one side of the flat top never wins
            assert!(
                ((x.mz - c) / c * 1e6).abs() < 20.0,
                "x{mult}: {} ppm",
                (x.mz - c) / c * 1e6
            );
            // the value is a height (the ceiling), the unit of the other peaks of a high-resolution scan, not the sum of the points
            assert!(
                x.value < 1.1e5 && x.value > 0.9e5,
                "x{mult}: value {}",
                x.value
            );
        }
    }
}

#[test]
fn the_tallest_unsaturated_peak_of_a_dense_scan_is_not_saturated() {
    let mut rng = Rng(9);
    for _ in 0..40 {
        let (_, cs) = flat_top(1.0, 30.0, &mut rng);
        assert_eq!(cs.len(), 1, "{cs:?}");
        assert_eq!(cs[0].flags & SATURATED, 0, "{:?}", cs[0]);
        assert!((cs[0].value - 1e5).abs() < 0.02e5, "value {}", cs[0].value);
    }
}
