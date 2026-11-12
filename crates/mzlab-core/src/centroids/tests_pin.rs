//! Without a width model (`wm` = `None`: low resolution, a width given in the plan, or a high-resolution file whose model was not fitted) the centroids
//! are the same bits as before the model could drive the windows: a hash of every centroid of a fixed set of synthetic scans, pinned.

use super::tests::{params, scan, Rng};
use super::*;

/// FNV-1a over the bits of every field of every centroid.
fn hash(cs: &[Centroid], (h, n): &mut (u64, usize)) {
    *n += cs.len();
    for c in cs {
        for v in [
            c.mz.to_bits(),
            c.value.to_bits(),
            c.area.to_bits(),
            c.height.to_bits(),
            u64::from(c.flags),
        ] {
            for b in v.to_le_bytes() {
                *h ^= u64::from(b);
                *h = h.wrapping_mul(0x100_0000_01b3);
            }
        }
    }
}

fn gauss(m: f64, c: f64, h: f64, fw: f64) -> f64 {
    h * (-4.0 * std::f64::consts::LN_2 * ((m - c) / fw).powi(2)).exp()
}

/// An Orbitrap-like scan: points 10 per FWHM (FWHM = 200/60 000 (m/200)^1.5) within 6 FWHM of each peak, uniform noise of `noise`.
fn orbitrap(peaks: &[(f64, f64)], noise: f64, rng: &mut Rng) -> (Vec<f64>, Vec<f64>) {
    let fw = |m: f64| 200.0 / 60_000.0 * (m / 200.0).powf(1.5);
    let (mut mz, mut y) = (Vec::new(), Vec::new());
    for &(c, _) in peaks {
        let mut m = c - 6.0 * fw(c);
        while m < c + 6.0 * fw(c) {
            if mz.last().is_none_or(|&l| m > l) {
                mz.push(m);
                y.push(
                    peaks
                        .iter()
                        .map(|&(x, h)| gauss(m, x, h, fw(x)))
                        .sum::<f64>()
                        + noise * rng.next(),
                );
            }
            m += fw(m) / 10.0;
        }
    }
    (mz, y)
}

#[test]
fn without_a_model_the_centroids_are_the_pinned_bits() {
    let mut rng = Rng(7);
    let mut h = (0xcbf2_9ce4_8422_2325u64, 0);
    let mut buf = Buf::default();
    // unit resolution counts: singles, doublets at 0.5 to 1.5 FWHM, a clipped top, shoulders at 1:0.3
    for k in 0..40 {
        let sep = 0.35 + 0.025 * k as f64;
        let peaks = [
            (103.0, 400.0, 0.7),
            (110.0, 300.0, 0.7),
            (110.0 + sep, 200.0, 0.7),
            (118.0, 900.0, 0.7),
            (118.0 + 0.6, 270.0, 0.7),
        ];
        let (mz, y) = scan(&mut rng, 100.0, 400, &peaks, 0.5, 2.5);
        let mut p = params(2.5);
        if k % 5 == 0 {
            p.vmax = 0.7 * y.iter().fold(0.0f64, |a, &b| a.max(b));
        }
        let y: Vec<f64> = y
            .iter()
            .map(|&v| if p.vmax > 0.0 { v.min(p.vmax) } else { v })
            .collect();
        hash(&centroids(&mz, &y, &p, &mut buf), &mut h);
    }
    // high resolution with one width for every m/z: the same file a model would describe
    let p = Params {
        mode: Mode::Continuous,
        unit: 1.0,
        step: 0.0003,
        w: 0.006,
        k: 3.0,
        vmax: 0.0,
        area: false,
        wm: None,
    };
    for k in 0..30 {
        let fw = |m: f64| 200.0 / 60_000.0 * (m / 200.0).powf(1.5);
        let c = 180.0 + 30.0 * k as f64;
        let peaks = [
            (c, 1e4),
            (c + 5.0, 5e3),
            (c + 5.0 + (0.5 + 0.05 * k as f64) * fw(c + 5.0), 5e3),
            (c + 12.0, 9e3),
            (c + 12.0 + 0.8 * fw(c + 12.0), 2.7e3),
        ];
        let (mz, y) = orbitrap(&peaks, 30.0, &mut rng);
        hash(&centroids(&mz, &y, &p, &mut buf), &mut h);
    }
    assert_eq!(h, (0x4226_a1f6_f319_efe3, 262), "pinned hash {:#x}", h.0);
}
