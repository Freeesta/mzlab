use super::*;
use crate::broad_blank::BlankBank;
use std::sync::Arc;

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
    // the bridge from the apex block to the end of the tail leaves the tail under the line: it has no half-height width either
    let mut off = on();
    off.front_s = -1.0;
    assert!(detect_broad(&rt, &vec![300.0; 1200], &y, &narrow, &off)
        .unwrap()
        .is_empty());
}

#[test]
fn front_width_is_zero_at_the_foot_of_a_tail_and_wide_on_a_band() {
    // blocks of 2 s: a step at block 3 (the apex block is the lower border) and a ramp of five blocks
    let rt_d = [1.0, 3.0, 5.0, 7.0, 9.0, 11.0];
    assert_eq!(
        hwhm_left(
            &rt_d,
            &[0.0, 0.0, 0.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0],
            2,
            3,
            3
        ),
        0.0
    );
    let ramp = [0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 5.0];
    assert_eq!(hwhm_left(&rt_d, &ramp, 2, 5, 0), 4.0);
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

fn bank_of(peaks: &[(f64, f64, f64)]) -> Arc<BlankBank> {
    let (rt, _, mut y) = trace(peaks);
    let mut rng = Lcg(99);
    for v in y.iter_mut() {
        *v += rng.next(10.0);
    }
    let mut b = BlankBank::default();
    b.add(300.0, &rt, &y, 8).unwrap();
    Arc::new(b.finish().unwrap())
}

fn with_blank(b: &Arc<BlankBank>, scale: bool) -> BroadParams {
    BroadParams {
        blank_pointwise: true,
        blank_scale: scale,
        blank: Some(b.clone()),
        ..on()
    }
}

#[test]
fn pointwise_blank_removes_the_hump_of_the_blank_and_keeps_the_sample_band() {
    let (rt, mz, y) = trace(&[(300.0, 75.0, 1000.0), (800.0, 75.0, 1000.0)]);
    let plain = detect_broad(&rt, &mz, &y, &[], &on()).unwrap();
    assert_eq!(plain.len(), 2, "{plain:?}");
    let bank = bank_of(&[(800.0, 75.0, 1000.0)]);
    for scale in [false, true] {
        let b = detect_broad(&rt, &mz, &y, &[], &with_blank(&bank, scale)).unwrap();
        assert_eq!(b.len(), 1, "{b:?}");
        assert!((b[0].rt_apex - 300.0).abs() <= 16.0, "{b:?}");
    }
}

#[test]
fn pointwise_blank_is_inert_without_the_option_or_the_bank_or_another_ion() {
    let (rt, mz, y) = trace(&[(300.0, 75.0, 1000.0), (800.0, 75.0, 1000.0)]);
    let plain = detect_broad(&rt, &mz, &y, &[], &on()).unwrap();
    let bank = bank_of(&[(800.0, 75.0, 1000.0)]);
    let off = BroadParams {
        blank: Some(bank.clone()),
        ..on()
    };
    assert_eq!(detect_broad(&rt, &mz, &y, &[], &off).unwrap(), plain);
    let no_bank = BroadParams {
        blank_pointwise: true,
        ..on()
    };
    assert_eq!(detect_broad(&rt, &mz, &y, &[], &no_bank).unwrap(), plain);
    let other = vec![450.0; y.len()];
    assert_eq!(
        detect_broad(&rt, &other, &y, &[], &with_blank(&bank, false)).unwrap(),
        detect_broad(&rt, &other, &y, &[], &on()).unwrap()
    );
}

/// The response as it was computed before the taps were tabulated: `ricker` evaluated at every sample.
fn response_direct(y: &[f64], t: usize, s: f64) -> f64 {
    let half = libm::ceil(4.0 * s) as isize;
    let (i0, i1) = (
        (t as isize - half).max(0) as usize,
        (t + half as usize + 1).min(y.len()),
    );
    let mut acc = 0.0;
    for (i, v) in y.iter().enumerate().take(i1).skip(i0) {
        acc += v * ricker((i as f64 - t as f64) / s);
    }
    acc / libm::sqrt(s)
}

#[test]
fn tabulated_taps_give_the_same_bits_as_the_direct_wavelet() {
    let mut rng = Lcg(11);
    let mut checked = 0usize;
    for round in 0..10_000 {
        let n = 3 + (round * 7) % 90;
        let y: Vec<f64> = (0..n).map(|_| rng.next(1000.0)).collect();
        // all the scales of the pass and the ridge scales, from sub-sample to wide
        let s = [0.8, 1.7, 3.3, 6.1, 11.9, 24.7][round % 6] * (1.0 + (round % 13) as f64 / 17.0);
        let k = Kernel::new(s).unwrap();
        let t = (round * 31) % n;
        assert_eq!(
            k.at(&y, t).to_bits(),
            response_direct(&y, t, s).to_bits(),
            "round {round} s {s} t {t}"
        );
        checked += 1;
    }
    assert_eq!(checked, 10_000);
}

#[test]
fn candidates_match_a_direct_scan_of_every_scale() {
    let (_, _, y) = trace(&[(600.0, 75.0, 1000.0), (580.0, 4.0, 8000.0)]);
    let sig: Vec<f64> = scale_widths(&on()).iter().map(|&w| sigma(w, 8.0)).collect();
    for &s in &sig {
        let k = Kernel::new(s).unwrap();
        for t in 0..y.len() {
            assert_eq!(k.at(&y, t).to_bits(), response_direct(&y, t, s).to_bits());
        }
    }
    assert!(!candidates(&y, &sig).unwrap().is_empty());
}

/// The inpainting of the first version: region by region, in the order given.
fn inpaint_sequential(y: &mut [f64], regions: &[(usize, usize)]) {
    let n = y.len();
    for &(s, e) in regions {
        if s > e || e >= n {
            continue;
        }
        match (s.checked_sub(1), (e + 1 < n).then_some(e + 1)) {
            (Some(l), Some(r)) => {
                let (yl, yr) = (y[l], y[r]);
                for (j, v) in y[l + 1..r].iter_mut().enumerate() {
                    *v = yl + (yr - yl) * (j + 1) as f64 / (r - l) as f64;
                }
            }
            (None, Some(r)) => {
                let v = y[r];
                y[..r].fill(v);
            }
            (Some(l), None) => {
                let v = y[l];
                y[l + 1..].fill(v);
            }
            (None, None) => {}
        }
    }
}

#[test]
fn inpaint_of_disjoint_regions_is_the_sequential_one() {
    let mut rng = Lcg(5);
    for round in 0..500 {
        let n = 30 + round % 40;
        let y: Vec<f64> = (0..n).map(|_| rng.next(100.0)).collect();
        // separated by at least one sample, some at the borders, some invalid
        let mut regions = Vec::new();
        let mut at = round % 3;
        while at + 2 < n {
            let e = (at + 1 + round % 4).min(n - 1);
            regions.push((at, e));
            at = e + 2 + round % 3;
        }
        regions.push((7, 3));
        regions.push((n, n + 2));
        let (mut a, mut b) = (y.clone(), y);
        inpaint_linear(&mut a, &regions);
        inpaint_sequential(&mut b, &regions);
        assert_eq!(a, b, "round {round} {regions:?}");
    }
}

#[test]
fn inpaint_merges_contiguous_and_overlapping_regions() {
    let base: Vec<f64> = (0..30)
        .map(|i| if i == 9 || i == 21 { 0.0 } else { 50.0 })
        .collect();
    let line = |a: usize, z: usize, y: &[f64]| -> Vec<f64> {
        (a..=z)
            .map(|i| y[a - 1] + (y[z + 1] - y[a - 1]) * (i - a + 1) as f64 / (z - a + 2) as f64)
            .collect()
    };
    for regions in [
        vec![(10, 15), (16, 20)],
        vec![(16, 20), (10, 15)],
        vec![(10, 17), (14, 20)],
        vec![(10, 20), (12, 14)],
    ] {
        let mut y = base.clone();
        inpaint_linear(&mut y, &regions);
        assert_eq!(&y[10..=20], &line(10, 20, &base)[..], "{regions:?}");
        assert_eq!((y[9], y[21]), (0.0, 0.0));
        assert!(y[..9].iter().chain(&y[22..]).all(|&v| v == 50.0));
    }
    // borders: the merged region is flat when it has one anchor only
    let mut y = vec![9.0, 9.0, 9.0, 9.0, 1.0, 2.0];
    inpaint_linear(&mut y, &[(0, 1), (2, 3)]);
    assert_eq!(y, [1.0, 1.0, 1.0, 1.0, 1.0, 2.0]);
    let mut y = vec![1.0, 2.0, 9.0, 9.0, 9.0];
    inpaint_linear(&mut y, &[(2, 3), (4, 4)]);
    assert_eq!(y, [1.0, 2.0, 2.0, 2.0, 2.0]);
    let mut y = vec![5.0, 6.0, 7.0];
    inpaint_linear(&mut y, &[(0, 2)]);
    assert_eq!(y, [5.0, 6.0, 7.0]);
}
