use super::*;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// A chromatogram-like trace: a sloped background, a few Gaussian peaks and noise.
fn trace(rng: &mut Rng, n: usize, scale: f64) -> Vec<f64> {
    let peaks: Vec<(f64, f64, f64)> = (0..1 + (rng.next() * 4.0) as usize)
        .map(|_| {
            (
                rng.next() * n as f64,
                rng.next() * 1e4 * scale,
                1.0 + rng.next() * 6.0,
            )
        })
        .collect();
    (0..n)
        .map(|i| {
            let g: f64 = peaks
                .iter()
                .map(|&(c, h, w)| h * (-((i as f64 - c) / w).powi(2) / 2.0).exp())
                .sum();
            scale * (50.0 + 0.2 * i as f64 + 20.0 * rng.next()) + g
        })
        .collect()
}

/// `asls_baseline` as it was: the dense n x n penalty matrix cloned at every iteration and solved by the dense `solve`.
fn old_asls(y: &[f64], lam: f64, p: f64, iters: usize, max_n: usize) -> Vec<f64> {
    let n = y.len();
    if n < 5 {
        let m = y.iter().cloned().fold(f64::INFINITY, f64::min);
        return vec![if n == 0 { 0.0 } else { m }; n];
    }
    if n > 20000 {
        return snip_baseline(y, 20);
    }
    let k = n.div_ceil(max_n).max(1);
    if k > 1 {
        let m = n / k;
        let yd: Vec<f64> = (0..m)
            .map(|b| y[b * k..(b + 1) * k].iter().sum::<f64>() / k as f64)
            .collect();
        let xd: Vec<f64> = (0..m).map(|b| (b as f64 + 0.5) * k as f64 - 0.5).collect();
        let zd = old_asls(&yd, lam / (k as f64).powi(4), p, iters, usize::MAX / 4);
        return (0..n)
            .map(|i| interp(i as f64, &xd, &zd).min(y[i]))
            .collect();
    }
    let mut pm = vec![0.0; n * n];
    for r in 0..n - 2 {
        let c = [(r, 1.0), (r + 1, -2.0), (r + 2, 1.0)];
        for &(i, vi) in &c {
            for &(j, vj) in &c {
                pm[i * n + j] += lam * vi * vj;
            }
        }
    }
    let mut w = vec![1.0; n];
    let mut z = y.to_vec();
    for _ in 0..iters {
        let mut a = pm.clone();
        for i in 0..n {
            a[i * n + i] += w[i];
        }
        let b: Vec<f64> = (0..n).map(|i| w[i] * y[i]).collect();
        z = match solve(a, b, n) {
            Some(s) => s,
            None => break,
        };
        let w_new: Vec<f64> = (0..n)
            .map(|i| if y[i] > z[i] { p } else { 1.0 - p })
            .collect();
        if w_new == w {
            break;
        }
        w = w_new;
    }
    (0..n).map(|i| z[i].min(y[i])).collect()
}

#[test]
fn band_penalty_is_the_dense_penalty_bit_for_bit() {
    for n in [5usize, 6, 7, 20, 160] {
        for lam in [1e-3, 1.0, 1e5, 3.7e11] {
            let mut pm = vec![0.0; n * n];
            for r in 0..n - 2 {
                let c = [(r, 1.0), (r + 1, -2.0), (r + 2, 1.0)];
                for &(i, vi) in &c {
                    for &(j, vj) in &c {
                        pm[i * n + j] += lam * vi * vj;
                    }
                }
            }
            assert_eq!(bits(&dense_of_band(&penalty_band(n, lam), n)), bits(&pm));
        }
    }
}

#[test]
fn banded_asls_gives_the_same_bits_as_the_dense_one() {
    let mut rng = Rng(2024);
    for case in 0..300 {
        let n = 5 + (rng.next() * 150.0) as usize;
        let y = trace(&mut rng, n, [1e-6, 1.0, 1e5][case % 3]);
        let lam = 10f64.powf(rng.next() * 9.0);
        let p = 0.0005 + rng.next() * 0.2;
        let iters = 1 + (rng.next() * 12.0) as usize;
        let a = asls_baseline(&y, lam, p, iters, 160);
        let b = old_asls(&y, lam, p, iters, 160);
        assert_eq!(bits(&a), bits(&b), "case {case}: n {n} lam {lam} p {p}");
    }
}

#[test]
fn banded_asls_is_identical_through_the_downsampling_path_and_on_odd_inputs() {
    let mut rng = Rng(77);
    for case in 0..60 {
        let n = 200 + (rng.next() * 1500.0) as usize;
        let y = trace(&mut rng, n, 1.0);
        let max_n = 40 + (rng.next() * 120.0) as usize;
        let a = asls_baseline(&y, 1e6, 0.01, 8, max_n);
        let b = old_asls(&y, 1e6, 0.01, 8, max_n);
        assert_eq!(bits(&a), bits(&b), "case {case}: n {n} max_n {max_n}");
    }
    // no downsampling at all (the recursion passes usize::MAX / 4), a size that still solves quickly with the dense reference
    let y = trace(&mut rng, 300, 1.0);
    assert_eq!(
        bits(&asls_baseline(&y, 1e5, 0.01, 10, usize::MAX / 4)),
        bits(&old_asls(&y, 1e5, 0.01, 10, usize::MAX / 4))
    );
    let flat = vec![5.0; 100];
    let steps: Vec<f64> = (0..100).map(|i| if i < 50 { 0.0 } else { 1e9 }).collect();
    let zeros = vec![0.0; 100];
    for y in [&flat, &steps, &zeros] {
        for lam in [1e-3, 1e2, 1e12] {
            let a = asls_baseline(y, lam, 0.01, 10, 160);
            let b = old_asls(y, lam, 0.01, 10, 160);
            assert_eq!(bits(&a), bits(&b), "lam {lam}");
        }
    }
    let mut bad = trace(&mut rng, 80, 1.0);
    bad[10] = f64::NAN;
    let a = asls_baseline(&bad, 1e5, 0.01, 5, 160);
    let b = old_asls(&bad, 1e5, 0.01, 5, 160);
    assert_eq!(bits(&a), bits(&b));
    let mut inf = trace(&mut rng, 80, 1.0);
    inf[40] = f64::INFINITY;
    assert_eq!(
        bits(&asls_baseline(&inf, 1e5, 0.01, 5, 160)),
        bits(&old_asls(&inf, 1e5, 0.01, 5, 160))
    );
}

/// `snip_baseline` as it was, with a new vector for every width.
fn old_snip(y: &[f64], iters: usize) -> Vec<f64> {
    let n = y.len();
    let mut v: Vec<f64> = y
        .iter()
        .map(|&x| (((x.max(0.0) + 1.0).sqrt() + 1.0).ln() + 1.0).ln())
        .collect();
    let top = iters.min((n / 2).saturating_sub(1).max(1));
    for w in 1..=top {
        let a: Vec<f64> = (0..n)
            .map(|i| {
                let right = if i + w < n { v[i + w] } else { v[n - 1] };
                let left = if i >= w { v[i - w] } else { v[0] };
                (right + left) / 2.0
            })
            .collect();
        for i in 0..n {
            v[i] = v[i].min(a[i]);
        }
    }
    v.iter()
        .map(|&x| ((x.exp() - 1.0).exp() - 1.0).powi(2) - 1.0)
        .collect()
}

/// `savgol_d` as it was, reflecting every index.
fn old_savgol_d(y: &[f64], window: usize, poly: usize, deriv: usize) -> Vec<f64> {
    let n = y.len();
    let mut window = window | 1;
    if n < 3 {
        return y.to_vec();
    }
    window = window.min(if n % 2 == 1 { n } else { n - 1 });
    let poly = poly.min(window - 1);
    let half = window / 2;
    let c = savgol_coeffs(window, poly, deriv);
    let ni = n as i64;
    (0..n)
        .map(|i| {
            (0..window)
                .map(|k| {
                    let src = i as i64 + k as i64 - half as i64;
                    let v = if n > half {
                        y[reflect(src, ni)]
                    } else {
                        y[src.clamp(0, ni - 1) as usize]
                    };
                    v * c[window - 1 - k]
                })
                .sum()
        })
        .collect()
}

#[test]
fn snip_and_savgol_keep_their_bits() {
    let mut rng = Rng(9);
    for n in (0..30).chain([64, 257, 1000]) {
        let y = trace(&mut rng, n, 3.0);
        for iters in [1, 5, 20, 100] {
            assert_eq!(
                bits(&snip_baseline(&y, iters)),
                bits(&old_snip(&y, iters)),
                "snip n {n}"
            );
        }
        for (window, poly, deriv) in [(5, 2, 0), (7, 3, 1), (11, 2, 2), (21, 3, 0), (51, 4, 1)] {
            let (a, b) = (
                savgol_d(&y, window, poly, deriv),
                old_savgol_d(&y, window, poly, deriv),
            );
            assert_eq!(bits(&a), bits(&b), "savgol n {n} window {window}");
        }
    }
}

/// `cwt_ricker` as it was: every tap clamped to the trace.
fn old_cwt(y: &[f64], widths: &[f64]) -> Vec<Vec<f64>> {
    let n = y.len();
    widths
        .iter()
        .map(|&w| {
            let k = ((10.0 * w).min(n as f64 - 1.0) as usize) | 1;
            let wv = ricker(w, k);
            let h = (k / 2) as i64;
            (0..n)
                .map(|i| {
                    (0..k)
                        .map(|j| {
                            let src = (i as i64 + j as i64 - h).clamp(0, n as i64 - 1) as usize;
                            y[src] * wv[k - 1 - j]
                        })
                        .sum::<f64>()
                })
                .collect()
        })
        .collect()
}

#[test]
fn cwt_keeps_its_bits_in_the_middle_and_at_the_ends() {
    let mut rng = Rng(5);
    for case in 0..200 {
        let n = 7 + (rng.next() * 120.0) as usize;
        let mut y = trace(&mut rng, n, 1.0);
        if case % 9 == 0 {
            y[n / 2] = 0.0;
            y[0] = -0.0;
        }
        let widths = [1.0, 1.7, 3.0, 5.4, 9.0, 12.0];
        let (a, b) = (cwt_ricker(&y, &widths), old_cwt(&y, &widths));
        for (x, z) in a.iter().zip(&b) {
            assert_eq!(bits(x), bits(z), "case {case} n {n}");
        }
    }
}

/// `noise_level` as it was: a vector for the differences, one for each median's copy, one for the deviations.
fn old_noise_level(y: &[f64]) -> f64 {
    if y.len() < 4 {
        return 0.0;
    }
    let d: Vec<f64> = y.windows(2).map(|w| w[1] - w[0]).collect();
    let m = median(&d);
    let dev: Vec<f64> = d.iter().map(|v| (v - m).abs()).collect();
    1.4826 * median(&dev) / 2f64.sqrt()
}

#[test]
fn noise_level_keeps_its_bits_also_with_nan_and_infinity() {
    let mut rng = Rng(8);
    for case in 0..600 {
        let n = (rng.next() * 40.0) as usize;
        let mut y = trace(&mut rng, n, [1.0, 1e-3, 1e6][case % 3]);
        if n > 6 {
            match case % 7 {
                0 => y[3] = f64::NAN,
                1 => y[2] = f64::INFINITY,
                2 => {
                    y[1] = f64::INFINITY;
                    y[4] = f64::NEG_INFINITY;
                }
                3 => y.iter_mut().for_each(|v| *v = 4.0),
                _ => {}
            }
        }
        assert_eq!(
            noise_level(&y).to_bits(),
            old_noise_level(&y).to_bits(),
            "case {case}"
        );
    }
}

#[test]
fn median_or_covers_empty_odd_even_and_nan() {
    assert!(median_or(&[], f64::NAN).is_nan());
    assert_eq!(median_or(&[], 0.0), 0.0);
    assert_eq!(median_or(&[3.0, 1.0, 2.0], 0.0), 2.0);
    assert_eq!(median_or(&[4.0, 1.0, 2.0, 3.0], 0.0), 2.5);
    // NaN sorts last under total_cmp and never panics
    assert_eq!(median_or(&[f64::NAN, 1.0, 2.0], 0.0), 2.0);
}

#[test]
fn interp_clamps_and_is_exact_at_nodes() {
    let (xp, fp) = ([0.0, 1.0, 3.0], [0.0, 10.0, 30.0]);
    assert_eq!(interp(-1.0, &xp, &fp), 0.0);
    assert_eq!(interp(5.0, &xp, &fp), 30.0);
    assert_eq!(interp(1.0, &xp, &fp), 10.0);
    assert_eq!(interp(2.0, &xp, &fp), 20.0);
}

#[test]
fn fwhm_walk_stops_at_the_first_point_not_above_half() {
    let x = [0.0, 1.0, 2.0, 3.0, 4.0];
    assert_eq!(fwhm_walk(&x, &[0.0, 6.0, 10.0, 6.0, 0.0], 2), 2.0);
    assert_eq!(fwhm_walk(&x, &[10.0, 10.0, 10.0, 10.0, 10.0], 2), 4.0);
}
