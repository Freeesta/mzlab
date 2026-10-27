//! The apex, height and extras of the shared-statistics version equal, bit for bit, the version that recomputed them for every peak.

use crate::peakext::*;
use crate::peaks::{detect_full, parabola_height, Params};
use crate::signal::solve;

/// The apex of the version that computed the spacing and the noise at every call, with heap vectors: the oracle of the bits.
fn legacy_apex(rt: &[f64], y: &[f64], i: usize) -> (f64, f64, f64) {
    let n = y.len();
    let (a, b) = (i.saturating_sub(2), (i + 3).min(n));
    if b - a < 3 {
        let mean_dt = if n > 1 {
            (rt[n - 1] - rt[0]) / (n - 1) as f64
        } else {
            0.0
        };
        return (rt[i], mean_dt, y[i]);
    }
    let xs: Vec<f64> = rt[a..b].iter().map(|t| t - rt[i]).collect();
    let mut ata = vec![0.0; 9];
    let mut aty = vec![0.0; 3];
    for (x, &yy) in xs.iter().zip(&y[a..b]) {
        for j in 0..3 {
            aty[j] += x.powi(j as i32) * yy;
            for k in 0..3 {
                ata[j * 3 + k] += x.powi((j + k) as i32);
            }
        }
    }
    let dt = median_dt(rt);
    let coef = match solve(ata.clone(), aty, 3) {
        Some(c) => c,
        None => return (rt[i], dt / 2.0, y[i]),
    };
    let (c0, c1, c2) = (coef[0], coef[1], coef[2]);
    if c2 >= 0.0 {
        return (rt[i], dt / 2.0, y[i]);
    }
    let xa = (-c1 / (2.0 * c2)).clamp(xs[0], xs[xs.len() - 1]);
    let h = c0 + c1 * xa + c2 * xa * xa;
    let rss: f64 = xs
        .iter()
        .zip(&y[a..b])
        .map(|(x, &yy)| (yy - (c0 + c1 * x + c2 * x * x)).powi(2))
        .sum();
    let s2 = (rss / (xs.len() as f64 - 3.0).max(1.0))
        .max(noise_level(y).powi(2))
        .max(1e-12);
    let g = [0.0, -1.0 / (2.0 * c2), c1 / (2.0 * c2 * c2)];
    let var = match solve(ata, g.to_vec(), 3) {
        Some(v) => s2 * (0..3).map(|j| g[j] * v[j]).sum::<f64>(),
        None => 0.0,
    };
    (rt[i] + xa, var.max(0.0).sqrt(), h)
}

fn bits(t: (f64, f64, f64)) -> [u64; 3] {
    [t.0.to_bits(), t.1.to_bits(), t.2.to_bits()]
}

/// Traces of unequal scan spacing with close peaks, plateaus, a flat run and a rise: the apex and the height equal the old ones at every index.
#[test]
fn apex_and_height_equal_the_per_call_version() {
    let mut seed = 99u64;
    let mut r = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for n in [3usize, 4, 5, 9, 40, 300] {
        let mut t = 0.0;
        let rt: Vec<f64> = (0..n)
            .map(|_| {
                t += 0.01 + 0.004 * r();
                t
            })
            .collect();
        for shape in 0..4 {
            let y: Vec<f64> = (0..n)
                .map(|k| {
                    let k = k as f64;
                    match shape {
                        0 => {
                            1e5 * (-(k - 20.0).powi(2) / 8.0).exp()
                                + 6e4 * (-(k - 26.0).powi(2) / 8.0).exp()
                                + 300.0 * r()
                        }
                        1 => 5.0e4 + 100.0 * r(),
                        2 => (k * 10.0).floor() * 7.0,
                        _ => 1e3 * r(),
                    }
                })
                .collect();
            let st = TraceStats::new(&rt, &y);
            for i in 0..n {
                assert_eq!(
                    bits(parabola_apex(&rt, &y, i, &st)),
                    bits(legacy_apex(&rt, &y, i)),
                    "n {n} shape {shape} i {i}"
                );
                let h = legacy_apex(&rt, &y, i).2;
                // the height alone is the same as the apex's only where the fit closes downwards; elsewhere both return y[i]
                assert_eq!(
                    parabola_height(&rt, &y, i).to_bits(),
                    h.to_bits(),
                    "height n {n} shape {shape} i {i}"
                );
            }
        }
    }
}

/// `extras` of every detected peak of a trace with two close peaks equals the one built from the per-call apex.
#[test]
fn extras_of_a_trace_equal_the_old_ones() {
    let rt: Vec<f64> = (0..400)
        .map(|k| 0.02 * f64::from(k) + 1e-4 * f64::from(k % 3))
        .collect();
    let y: Vec<f64> = (0..400)
        .map(|k| {
            let k = f64::from(k);
            2e6 * (-(k - 120.0).powi(2) / 18.0).exp()
                + 1.2e6 * (-(k - 131.0).powi(2) / 18.0).exp()
                + 8e5 * (-(k - 300.0).powi(2) / 30.0).exp()
                + 2e3 * (k * 1.7).sin()
        })
        .collect();
    let d = detect_full(&rt, &y, Params::default());
    assert!(d.peaks.len() >= 2);
    let st = TraceStats::new(&rt, &d.ys);
    for pk in &d.peaks {
        let e = extras(&rt, &d.ys, pk, &st);
        let w = fwhm(&rt, &d.ys, pk.apex_i, pk.lo, pk.hi);
        let dt = median_dt(&rt);
        let (apex_rt, apex_sigma, _) = legacy_apex(&rt, &d.ys, pk.apex_i);
        assert_eq!(e.apex_rt.to_bits(), apex_rt.to_bits());
        assert_eq!(e.apex_sigma.to_bits(), apex_sigma.to_bits());
        assert_eq!(e.fwhm_min.to_bits(), w.to_bits());
        assert_eq!(
            e.fwhm_scans.to_bits(),
            (if dt > 0.0 { w / dt } else { 0.0 }).to_bits()
        );
    }
}
