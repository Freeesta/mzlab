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

#[test]
fn banded_solver_gives_the_same_bits_as_the_dense_one() {
    let mut rng = Rng(2024);
    for case in 0..300 {
        let n = 5 + (rng.next() * 150.0) as usize;
        let y = trace(&mut rng, n, [1e-6, 1.0, 1e5][case % 3]);
        let lam = 10f64.powf(rng.next() * 9.0);
        let p = 0.0005 + rng.next() * 0.2;
        let iters = 1 + (rng.next() * 12.0) as usize;
        let a = asls_with(&y, lam, p, iters, 160, solve_banded);
        let b = asls_with(&y, lam, p, iters, 160, solve);
        assert_eq!(bits(&a), bits(&b), "case {case}: n {n} lam {lam} p {p}");
    }
}

#[test]
fn banded_solver_is_identical_through_the_downsampling_path_and_on_odd_inputs() {
    let mut rng = Rng(77);
    for case in 0..60 {
        let n = 200 + (rng.next() * 1500.0) as usize;
        let y = trace(&mut rng, n, 1.0);
        let max_n = 40 + (rng.next() * 120.0) as usize;
        let a = asls_with(&y, 1e6, 0.01, 8, max_n, solve_banded);
        let b = asls_with(&y, 1e6, 0.01, 8, max_n, solve);
        assert_eq!(bits(&a), bits(&b), "case {case}: n {n} max_n {max_n}");
    }
    let flat = vec![5.0; 100];
    let steps: Vec<f64> = (0..100).map(|i| if i < 50 { 0.0 } else { 1e9 }).collect();
    let zeros = vec![0.0; 100];
    for y in [&flat, &steps, &zeros] {
        for lam in [1e-3, 1e2, 1e12] {
            let a = asls_with(y, lam, 0.01, 10, 160, solve_banded);
            let b = asls_with(y, lam, 0.01, 10, 160, solve);
            assert_eq!(bits(&a), bits(&b), "lam {lam}");
        }
    }
    let mut bad = trace(&mut rng, 80, 1.0);
    bad[10] = f64::NAN;
    let a = asls_with(&bad, 1e5, 0.01, 5, 160, solve_banded);
    let b = asls_with(&bad, 1e5, 0.01, 5, 160, solve);
    assert_eq!(bits(&a), bits(&b));
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
