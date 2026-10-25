use super::*;

/// Deterministic noise: sum of twelve uniforms of an LCG, mean 0, sd `sd`.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, sd: f64) -> f64 {
        let mut s = 0.0;
        for _ in 0..12 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            s += (self.0 >> 11) as f64 / (1u64 << 53) as f64;
        }
        (s - 6.0) * sd
    }
}

fn gauss(t: f64, t0: f64, sd: f64, h: f64) -> f64 {
    h * libm::exp(-0.5 * (t - t0) * (t - t0) / (sd * sd))
}

fn trace(peaks: &[(f64, f64, f64)]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut rng = Lcg(7);
    let rt: Vec<f64> = (0..1200).map(|i| i as f64).collect();
    let y = rt
        .iter()
        .map(|&t| {
            peaks
                .iter()
                .map(|&(t0, sd, h)| gauss(t, t0, sd, h))
                .sum::<f64>()
                + rng.next(40.0)
        })
        .collect();
    (rt, vec![300.0; 1200], y)
}

fn on() -> BroadParams {
    BroadParams {
        enabled: true,
        ..BroadParams::default()
    }
}

const BAND: f64 = 1000.0 * 75.0 * 2.506_628_274_631_000_5;

#[test]
fn narrow_plus_broad_separates_both() {
    let (rt, mz, y) = trace(&[(600.0, 75.0, 1000.0), (580.0, 4.0, 8000.0)]);
    let narrow = [(565, 595)];
    assert!(!is_broad_ridge(&y, 565, 595, 1.0).unwrap());
    let b = detect_broad(&rt, &mz, &y, &narrow, &on()).unwrap();
    assert_eq!(b.len(), 1, "{b:?}");
    assert!((b[0].rt_apex - 600.0).abs() <= 16.0, "{b:?}");
    assert!(
        (b[0].area_s - BAND).abs() <= 0.15 * BAND,
        "{} {BAND}",
        b[0].area_s
    );
}

#[test]
fn ridge_test_preserves_broad_crest() {
    let (rt, mz, y) = trace(&[(600.0, 75.0, 1000.0)]);
    assert!(is_broad_ridge(&y, 585, 615, 1.0).unwrap());
    let b = detect_broad(&rt, &mz, &y, &[(585, 615)], &on()).unwrap();
    assert_eq!(b.len(), 1, "{b:?}");
    assert!((b[0].area_s - BAND).abs() <= 0.15 * BAND, "{}", b[0].area_s);
}

#[test]
fn narrow_only_trace_emits_zero_broad_peaks() {
    let (rt, mz, y) = trace(&[
        (300.0, 4.0, 8000.0),
        (600.0, 5.0, 8000.0),
        (900.0, 6.0, 8000.0),
    ]);
    let narrow = [(280, 320), (575, 625), (870, 930)];
    let b = detect_broad(&rt, &mz, &y, &narrow, &on()).unwrap();
    assert!(b.is_empty(), "{b:?}");
}

#[test]
fn disabled_by_default_is_noop() {
    let (rt, mz, y) = trace(&[(600.0, 75.0, 1000.0)]);
    assert!(detect_broad(&rt, &mz, &y, &[], &BroadParams::default())
        .unwrap()
        .is_empty());
}

#[test]
fn inpaint_and_decimate_follow_the_spec() {
    let mut y = [1.0, 9.0, 9.0, 4.0, 5.0];
    inpaint_linear(&mut y, &[(1, 2)]);
    assert_eq!(y, [1.0, 2.0, 3.0, 4.0, 5.0]);
    let (r, v, z) = decimate(&[0.0, 1.0, 2.0], &[1.0, 3.0, 5.0], &[1.0, 3.0, 0.0], 2).unwrap();
    assert_eq!((r, v, z), (vec![0.5, 2.0], vec![2.0, 0.0], vec![2.5, 5.0]));
}

/// Gaussian convolved with an exponential tail (EMG), unit area scaled to height `h` at its own maximum approximately.
fn emg(t: f64, t0: f64, sd: f64, tau: f64, h: f64) -> f64 {
    let z = (sd / tau - (t - t0) / sd) / core::f64::consts::SQRT_2;
    let e = libm::exp(sd * sd / (2.0 * tau * tau) - (t - t0) / tau);
    h * e * 0.5 * libm::erfc(z)
}

#[test]
fn tail_of_a_narrow_peak_is_not_a_band() {
    let mut rng = Lcg(5);
    let rt: Vec<f64> = (0..1200).map(|i| i as f64).collect();
    let y: Vec<f64> = rt
        .iter()
        .map(|&t| emg(t, 600.0, 4.0, 40.0, 40000.0) + rng.next(40.0))
        .collect();
    // the detector of narrow peaks stops where the tail is still high; its shoulder reads as a band unless the steep front is checked
    let narrow = [(590, 610)];
    let b = detect_broad(&rt, &vec![300.0; 1200], &y, &narrow, &on()).unwrap();
    assert!(b.is_empty(), "{b:?}");
    let mut off = on();
    off.front_s = -1.0;
    assert!(!detect_broad(&rt, &vec![300.0; 1200], &y, &narrow, &off)
        .unwrap()
        .is_empty());
}

#[test]
fn wide_bands_keep_recall_and_area() {
    let (mut hit, mut total) = (0, 0);
    for fwhm in [60.0, 90.0, 120.0, 180.0, 240.0] {
        let sd = fwhm / 2.354_820_045;
        for emg_tau in [0.0, 25.0] {
            let mut rng = Lcg(fwhm as u64 + emg_tau as u64);
            let rt: Vec<f64> = (0..1500).map(|i| i as f64).collect();
            let f = |t: f64| {
                if emg_tau > 0.0 {
                    emg(t, 600.0, sd, emg_tau, 3000.0)
                } else {
                    gauss(t, 600.0, sd, 3000.0)
                }
            };
            let truth: f64 = rt.iter().map(|&t| f(t)).sum();
            let y: Vec<f64> = rt.iter().map(|&t| f(t) + rng.next(40.0)).collect();
            let b = detect_broad(&rt, &vec![300.0; 1500], &y, &[], &on()).unwrap();
            total += 1;
            if let Some(p) = b
                .iter()
                .find(|p| (p.rt_apex - 600.0).abs() <= 60.0 + emg_tau)
            {
                hit += 1;
                assert!(
                    (p.area_s - truth).abs() <= 0.2 * truth,
                    "{fwhm} {emg_tau}: {} vs {truth}",
                    p.area_s
                );
            }
        }
    }
    assert!(hit as f64 / total as f64 >= 0.8, "{hit}/{total}");
}
