use super::*;

/// Small deterministic generator (xorshift64*), so the tests need no crate and give the same scans every time.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn poisson(&mut self, lam: f64) -> u32 {
        if lam > 30.0 {
            let (a, b) = (self.next().max(1e-12), self.next());
            let g = (-2.0 * a.ln()).sqrt() * (2.0 * std::f64::consts::PI * b).cos();
            return (lam + lam.sqrt() * g).round().max(0.0) as u32;
        }
        let (l, mut k, mut p) = ((-lam).exp(), 0, 1.0);
        loop {
            p *= self.next();
            if p <= l {
                return k;
            }
            k += 1;
        }
    }
}

const STEP: f64 = 0.06;
const W: f64 = 0.7;

fn params(unit: f64) -> Params {
    Params { mode: Mode::Counts, unit, step: STEP, w: W, k: 3.0, vmax: 0.0, area: true }
}

/// A scan on the 0.06 grid (only the points above zero) with Gaussian peaks `(centre, height in counts, fwhm)` and Poisson counts; `bg` counts per bin.
fn scan(rng: &mut Rng, start: f64, bins: usize, peaks: &[(f64, f64, f64)], bg: f64, unit: f64) -> (Vec<f64>, Vec<f64>) {
    let (mut mz, mut y) = (vec![], vec![]);
    for b in 0..bins {
        let x = start + b as f64 * STEP;
        let lam = bg + peaks.iter().map(|&(c, h, w)| h * (-(x - c).powi(2) / (2.0 * (w / 2.355).powi(2))).exp()).sum::<f64>();
        let n = rng.poisson(lam);
        if n > 0 {
            mz.push(x);
            y.push(n as f64 * unit);
        }
    }
    (mz, y)
}

fn run(mz: &[f64], y: &[f64], p: &Params) -> Vec<Centroid> {
    centroids(mz, y, p, &mut Buf::default())
}

#[test]
fn an_isolated_peak_gives_one_centroid_near_its_centre() {
    let mut rng = Rng(11);
    for &h in &[20.0, 100.0, 1000.0] {
        let mut worst: f64 = 0.0;
        for t in 0..60 {
            let c = 300.0 + 0.02 * t as f64;
            let (mz, y) = scan(&mut rng, 297.0, 100, &[(c, h, W)], 0.0, 1.0);
            let out = run(&mz, &y, &params(1.0));
            assert_eq!(out.len(), 1, "h={h} t={t}: {out:?}");
            worst = worst.max((out[0].mz - c).abs());
        }
        let sigma_m = W / 2.355;
        let n = h * W / STEP * 1.064;
        assert!(worst <= 6.0 * 0.6 * sigma_m / n.sqrt() * 3.0 + 0.02, "h={h}: worst {worst}");
    }
}

#[test]
fn a_noiseless_peak_is_found_within_half_a_milli_dalton() {
    let (mut mz, mut y) = (vec![], vec![]);
    for b in 0..100 {
        let x = 297.0 + b as f64 * STEP;
        let v = (1000.0 * (-(x - 300.013).powi(2) / (2.0 * (W / 2.355).powi(2))).exp()).round();
        if v > 0.0 {
            mz.push(x);
            y.push(v);
        }
    }
    let out = run(&mz, &y, &params(1.0));
    assert_eq!(out.len(), 1);
    assert!((out[0].mz - 300.013).abs() <= 5e-4, "{}", out[0].mz);
}

#[test]
fn the_scale_of_the_intensities_changes_the_areas_and_not_the_masses() {
    let mut rng = Rng(5);
    let (mz, y) = scan(&mut rng, 297.0, 100, &[(300.0, 50.0, W), (302.0, 200.0, W)], 0.3, 1.0);
    let a = run(&mz, &y, &params(1.0));
    let y7: Vec<f64> = y.iter().map(|v| v * 7.3).collect();
    let b = run(&mz, &y7, &params(7.3));
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(&b) {
        assert!((p.mz - q.mz).abs() < 1e-9, "{} {}", p.mz, q.mz);
        assert!((q.area / p.area - 7.3).abs() < 1e-9);
    }
    let shifted: Vec<f64> = mz.iter().map(|m| m + 0.03).collect();
    let c = run(&shifted, &y, &params(1.0));
    for (p, q) in a.iter().zip(&c) {
        assert!((q.mz - p.mz - 0.03).abs() < 1e-9);
    }
}

#[test]
fn pure_noise_gives_almost_no_peaks() {
    let mut rng = Rng(3);
    for &bg in &[0.2, 1.0, 5.0] {
        let total: usize = (0..1000)
            .map(|_| {
                let (mz, y) = scan(&mut rng, 100.0, 200, &[], bg, 1.0);
                run(&mz, &y, &params(1.0)).len()
            })
            .sum();
        assert!(total as f64 / 1000.0 <= 0.05, "bg {bg}: {total} false peaks in 1000 scans");
    }
}

#[test]
fn doublets_follow_the_resolution_rules() {
    let mut rng = Rng(21);
    for &(ratio, label) in &[(1.0, "1:1"), (3.0, "1:3"), (10.0, "1:10")] {
        for &d in &[2.0f64, 0.5, 0.3] {
            let (mz, y) = scan(&mut rng, 297.0, 100, &[(300.0, 400.0, W), (300.0 + d * W, 400.0 / ratio, W)], 0.0, 1.0);
            let out = run(&mz, &y, &params(1.0));
            assert!(out.len() <= 2, "{label} d={d}: {out:?}");
            if d >= 2.0 {
                assert_eq!(out.iter().filter(|c| c.flags & SHOULDER == 0).count(), 2, "{label}: {out:?}");
            }
            if d <= 0.5 {
                assert_eq!(out.len(), 1, "{label} d={d}: {out:?}");
            }
        }
    }
}

#[test]
fn an_isotope_pattern_gives_three_centroids() {
    let mut rng = Rng(8);
    let (mz, y) = scan(&mut rng, 297.0, 150, &[(300.0, 800.0, W), (301.0, 300.0, W), (302.0, 500.0, W)], 0.0, 1.0);
    let out = run(&mz, &y, &params(1.0));
    assert_eq!(out.len(), 3, "{out:?}");
    for (c, &t) in out.iter().zip(&[300.0, 301.0, 302.0]) {
        assert!((c.mz - t).abs() < 0.05, "{} {t}", c.mz);
    }
    let want = [800.0, 300.0, 500.0].map(|h| h * W / STEP * 1.064);
    for (c, w) in out.iter().zip(want) {
        assert!((c.area - w).abs() <= 2.0 * w.sqrt() + 0.25 * w, "{} {w}", c.area);
    }
}

#[test]
fn a_flat_top_is_flagged_as_saturated() {
    let (mut mz, mut y) = (vec![], vec![]);
    for b in 0..60 {
        let x = 100.0 + b as f64 * STEP;
        let v = (1000.0 * (-(x - 101.8).powi(2) / (2.0 * (W / 2.355).powi(2))).exp()).round().min(600.0);
        if v > 0.0 {
            mz.push(x);
            y.push(v);
        }
    }
    let mut p = params(1.0);
    p.vmax = 600.0;
    let out = run(&mz, &y, &p);
    assert_eq!(out.len(), 1);
    assert!(out[0].flags & SATURATED != 0);
    assert!((out[0].mz - 101.8).abs() < 0.005, "{}", out[0].mz);
}

#[test]
fn edges_and_gaps_are_handled() {
    let mut rng = Rng(14);
    let (mz, y) = scan(&mut rng, 300.0, 40, &[(300.1, 300.0, W)], 0.0, 1.0);
    let out = run(&mz, &y, &params(1.0));
    assert!(out.iter().all(|c| c.flags & TRUNCATED != 0) && out.len() <= 1, "{out:?}");
    let (mut mz, mut y) = scan(&mut rng, 100.0, 60, &[(101.8, 300.0, W)], 0.0, 1.0);
    let (m2, y2) = scan(&mut rng, 200.0, 60, &[(201.8, 300.0, W)], 0.0, 1.0);
    mz.extend(m2);
    y.extend(y2);
    let out = run(&mz, &y, &params(1.0));
    assert_eq!(out.len(), 2, "{out:?}");
}

#[test]
fn very_low_counts_never_divide_by_zero() {
    for n in 0..6 {
        let mz: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * STEP).collect();
        let y = vec![1.0; n];
        let out = run(&mz, &y, &params(1.0));
        assert!(out.iter().all(|c| c.mz.is_finite() && c.value.is_finite()));
    }
    let out = run(&[100.0], &[1.0], &params(1.0));
    assert!(out.is_empty());
}

#[test]
fn a_high_resolution_peak_on_an_irregular_grid_is_within_half_a_ppm() {
    // Orbitrap-like: continuous intensities, points every 2 mDa around m/z 400 (R = 60000: FWHM 6.7 mDa, 3 points per FWHM is too few: use 1 mDa)
    let (c, fwhm): (f64, f64) = (400.1234, 400.0 / 60000.0);
    let mut rng = Rng(31);
    let (mut mz, mut y) = (vec![], vec![]);
    for i in 0..80 {
        let x = 400.08 + i as f64 * 0.001 + 0.0003 * ((i * 7 % 5) as f64 / 5.0);
        mz.push(x);
        y.push(1.0e6 * (-(x - c).powi(2) / (2.0 * (fwhm / 2.355).powi(2))).exp() + 50.0 * rng.next());
    }
    let p = Params { mode: Mode::Continuous, unit: 1.0, step: 0.001, w: fwhm, k: 3.0, vmax: 0.0, area: false };
    let out = run(&mz, &y, &p);
    assert_eq!(out.len(), 1, "{out:?}");
    assert!((out[0].mz - c).abs() / c * 1e6 <= 0.5, "{} ppm", (out[0].mz - c).abs() / c * 1e6);
}

#[test]
fn the_same_scan_gives_the_same_centroids() {
    let mut rng = Rng(77);
    let (mz, y) = scan(&mut rng, 297.0, 150, &[(300.0, 80.0, W), (301.0, 30.0, W)], 0.5, 1.0);
    let mut buf = Buf::default();
    let a = centroids(&mz, &y, &params(1.0), &mut buf);
    let b = centroids(&mz, &y, &params(1.0), &mut buf);
    assert_eq!(a, b);
}

#[test]
fn the_estimate_finds_counts_the_unit_the_step_and_the_width() {
    let mut rng = Rng(9);
    let scans: Vec<(Vec<f64>, Vec<f64>)> = (0..20)
        .map(|t| scan(&mut rng, 297.0, 120, &[(300.0 + 0.01 * t as f64, 900.0, 0.7), (304.0, 700.0, 0.7), (306.0, 500.0, 0.7)], 0.3, 2.5))
        .collect();
    let refs: Vec<(&[f64], &[f64])> = scans.iter().map(|(a, b)| (a.as_slice(), b.as_slice())).collect();
    let e = estimate(&refs, 0.7);
    assert_eq!(e.params.mode, Mode::Counts);
    assert!((e.params.unit - 2.5).abs() < 1e-9 && (e.params.step - STEP).abs() < 1e-9 && e.regular);
    assert!(e.peaks >= 8 && (e.params.w - 0.7).abs() < 0.12, "{e:?}");
}
