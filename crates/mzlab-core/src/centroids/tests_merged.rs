//! MERGED: a centroid that holds two peaks closer than about one FWHM. On a synthetic Orbitrap-like file (FWHM ~ m^1.5, the width model estimated
//! from the file as a conversion does) no single peak is flagged and every 1:1 doublet 0.5 or 0.75 FWHM apart is.

use super::tests::Rng;
use super::*;

fn gauss(m: f64, c: f64, h: f64, fw: f64) -> f64 {
    h * (-4.0 * std::f64::consts::LN_2 * ((m - c) / fw).powi(2)).exp()
}

/// FWHM at `m` of the synthetic instrument: R = 60 000 at m/z 200, falling as m^-0.5.
fn fwhm(m: f64) -> f64 {
    200.0 / 60_000.0 * (m / 200.0).powf(1.5)
}

/// A profile scan with peaks at `peaks` (centre, height) on a grid of 10 points per FWHM, noise of amplitude `1/snr` of the tallest.
fn scan(peaks: &[(f64, f64)], snr: f64, rng: &mut Rng) -> (Vec<f64>, Vec<f64>) {
    let (mut mz, mut y) = (Vec::new(), Vec::new());
    let mut m = 140.0;
    while m < 1300.0 {
        // only the points within 6 FWHM of a peak: the rest of the scan is empty
        let near: Vec<(f64, f64)> = peaks
            .iter()
            .copied()
            .filter(|&(c, _)| (m - c).abs() < 6.0 * fwhm(c))
            .collect();
        if near.is_empty() {
            match peaks
                .iter()
                .map(|&(c, _)| c - 6.0 * fwhm(c))
                .filter(|&s| s > m)
                .fold(f64::INFINITY, f64::min)
            {
                s if s.is_finite() => m = s,
                _ => break,
            }
            continue;
        }
        let v: f64 = near.iter().map(|&(c, h)| gauss(m, c, h, fwhm(c))).sum();
        mz.push(m);
        y.push((v + 1e4 / snr * (rng.next() - 0.5) * 3.46).max(0.0));
        m += fwhm(m) / 10.0;
    }
    (mz, y)
}

fn singles(rng: &mut Rng) -> Vec<(f64, f64)> {
    (0..40)
        .map(|i| {
            (
                150.0 + 26.0 * i as f64 + 5.0 * rng.next(),
                1e4 * (0.5 + rng.next()),
            )
        })
        .collect()
}

#[test]
fn merged_flags_doublets_and_no_single_peak() {
    for snr in [1000.0, 50.0] {
        let mut rng = Rng(21);
        let scans: Vec<_> = (0..6)
            .map(|_| scan(&singles(&mut rng), snr, &mut rng))
            .collect();
        let refs: Vec<(&[f64], &[f64])> = scans
            .iter()
            .map(|(a, b)| (a.as_slice(), b.as_slice()))
            .collect();
        let p = estimate(&refs, 0.7).params;
        let (a, b) = p.wm.expect("a width model for an HR file");
        assert!((b - 1.5).abs() < 0.15, "b = {b} at SNR {snr}");
        let mut buf = Buf::default();
        let (mut single, mut false_pos) = (0, 0);
        for _ in 0..20 {
            let (mz, y) = scan(&singles(&mut rng), snr, &mut rng);
            for c in centroids(&mz, &y, &p, &mut buf)
                .iter()
                .filter(|c| c.value > 2e3)
            {
                single += 1;
                false_pos += usize::from(c.flags & MERGED != 0);
            }
        }
        assert!(single >= 400, "{single} single peaks at SNR {snr}");
        assert_eq!(
            false_pos, 0,
            "{false_pos} of {single} single peaks flagged at SNR {snr} (a {a}, b {b})"
        );
        for sep in [0.5, 0.75] {
            let mut found = 0;
            for i in 0..40 {
                let c = 180.0 + 25.0 * i as f64 + rng.next();
                // two halves of a peak the file has (below its highest point, which would read as saturation)
                let (mz, y) = scan(&[(c, 5e3), (c + sep * fwhm(c), 5e3)], snr, &mut rng);
                let cs = centroids(&mz, &y, &p, &mut buf);
                let main = cs
                    .iter()
                    .max_by(|x, y| x.value.total_cmp(&y.value))
                    .unwrap();
                found += usize::from(main.flags & MERGED != 0);
            }
            assert_eq!(
                found, 40,
                "doublets {sep} FWHM apart flagged {found}/40 at SNR {snr}"
            );
        }
    }
}
