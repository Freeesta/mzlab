//! `peakq_lrauto::lr_auto_integrate` on synthetic jagged 3200 QTRAP-like traces (1.5 s cycle, 10-20 points per peak, counting noise, spray jitter,
//! spikes): the count of peaks, where touching peaks split, raw areas and the clamped baseline.
use super::*;

const DT: f64 = 1.5 / 60.0;

/// Deterministic noise, approximately normal (sum of four uniforms, xorshift), so the test never flakes.
fn gauss(seed: &mut u64) -> f64 {
    let mut u = || {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        (*seed >> 11) as f64 / (1u64 << 53) as f64
    };
    (u() + u() + u() + u() - 2.0) * 3f64.sqrt()
}

/// 7 min of trace: 200 counts of background, Gaussians `(centre, sigma, height)` (min, min, counts), 4 % jitter, counting noise, a spike every 70 points.
fn trace(peaks: &[(f64, f64, f64)], seed: u64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut sd = seed;
    let rt: Vec<f64> = (0..280).map(|i| i as f64 * DT).collect();
    let clean: Vec<f64> = rt
        .iter()
        .map(|&t| {
            200.0
                + peaks
                    .iter()
                    .map(|&(c, w, h)| h * (-0.5 * ((t - c) / w).powi(2)).exp())
                    .sum::<f64>()
        })
        .collect();
    let y = clean
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let v = c * (1.0 + 0.04 * gauss(&mut sd)) + 1.5 * c.sqrt() * gauss(&mut sd);
            (v + if i % 70 == 33 { 10.0 * c.sqrt() } else { 0.0 }).max(0.0)
        })
        .collect();
    (rt, y, clean)
}

fn run(rt: &[f64], y: &[f64]) -> LrAuto {
    lr_auto_integrate(rt, y, rt[0], rt[rt.len() - 1], LrAutoParams::default())
}

fn contains(k: &LrAutoPeak, rt: &[f64], t: f64) -> bool {
    rt[k.lo] <= t && t <= rt[k.hi]
}

#[test]
fn isolated_jagged_peaks_are_one_each_with_their_raw_area() {
    let pk = [
        (1.2, 0.06, 2.0e4),
        (2.6, 0.07, 6.0e3),
        (4.0, 0.05, 4.0e4),
        (5.6, 0.08, 1.5e3),
    ];
    for seed in 1..=10 {
        let (rt, y, _) = trace(&pk, seed);
        let r = run(&rt, &y);
        assert_eq!(r.peaks.len(), 4, "seed {seed}: {:?}", r.peaks);
        for (k, &(c, w, h)) in r.peaks.iter().zip(&pk) {
            assert!(contains(k, &rt, c), "seed {seed}: {k:?}");
            let truth = h * w * (2.0 * std::f64::consts::PI).sqrt();
            assert!(
                (k.area / truth - 1.0).abs() < 0.12,
                "seed {seed}: area {} truth {truth}",
                k.area
            );
            assert!(
                k.base_lo >= 0.0
                    && k.base_lo <= y[k.lo]
                    && k.base_hi >= 0.0
                    && k.base_hi <= y[k.hi],
                "{k:?}"
            );
        }
        assert!(r.half_auto && r.half >= 2, "{}", r.half);
    }
}

#[test]
fn touching_peaks_split_at_the_valley_and_a_shoulder_stays_one() {
    for seed in 1..=10 {
        // a pair 4 sigma apart: the clean valley is at 2.125 min
        let (rt, y, clean) = trace(&[(2.0, 0.06, 2.0e4), (2.24, 0.06, 1.2e4)], seed);
        let r = run(&rt, &y);
        assert_eq!(r.peaks.len(), 2, "seed {seed}: {:?}", r.peaks);
        let (a, b) = (&r.peaks[0], &r.peaks[1]);
        assert_eq!(a.hi, b.lo, "the two share the valley point");
        assert_eq!((a.end_hi, b.end_lo), (End::Valley, End::Valley));
        let v = (80..90)
            .min_by(|&i, &j| clean[i].total_cmp(&clean[j]))
            .unwrap();
        assert!(
            a.hi.abs_diff(v) <= 2,
            "seed {seed}: cut at {} valley at {v}",
            a.hi
        );
        // drop line: the cut takes the line under the whole group, so the two baselines are one straight line
        let slope = |k: &LrAutoPeak| (k.base_hi - k.base_lo) / (rt[k.hi] - rt[k.lo]);
        assert!(
            (slope(a) - slope(b)).abs() <= 1e-9 * slope(a).abs().max(1.0)
                || a.base_hi == y[a.hi].max(0.0),
            "{a:?} {b:?}"
        );
        // a shoulder (no minimum in the clean trace) is one peak
        let (rt, y, _) = trace(&[(2.5, 0.06, 3.0e4), (2.596, 0.06, 0.8e4)], seed);
        assert_eq!(run(&rt, &y).peaks.len(), 1, "seed {seed}");
    }
}

#[test]
fn a_small_neighbour_of_a_jagged_peak_is_kept_and_valley_mode_ends_on_the_trace() {
    for seed in 1..=10 {
        let (rt, y, _) = trace(&[(2.0, 0.06, 4.0e4), (2.24, 0.06, 6.0e3)], seed);
        let r = run(&rt, &y);
        assert_eq!(r.peaks.len(), 2, "seed {seed}: {:?}", r.peaks);
        let p = LrAutoParams {
            split: Split::Valley,
            ..Default::default()
        };
        let v = lr_auto_integrate(&rt, &y, 0.0, 7.0, p);
        let (a, b) = (&v.peaks[0], &v.peaks[1]);
        assert_eq!(a.base_hi, b.base_lo);
        assert!(
            a.base_hi <= y[a.hi] && a.base_hi > 1000.0,
            "the valley baseline is on the trace: {a:?}"
        );
        // the parts of a valley-to-valley split take less than the drop line: the area under the valley goes to neither
        assert!(v.peaks[1].area < r.peaks[1].area);
    }
}

#[test]
fn spikes_and_noise_alone_give_nothing_and_the_stretch_limits_the_apexes() {
    for seed in 1..=20 {
        let (rt, y, _) = trace(&[], seed);
        assert!(run(&rt, &y).peaks.is_empty(), "seed {seed}");
    }
    let (rt, y, _) = trace(&[(1.2, 0.06, 2.0e4), (4.0, 0.05, 4.0e4)], 3);
    let r = lr_auto_integrate(&rt, &y, 3.0, 7.0, LrAutoParams::default());
    assert_eq!(r.peaks.len(), 1);
    assert!(contains(&r.peaks[0], &rt, 4.0));
}

#[test]
fn a_tailing_peak_keeps_its_tail_and_the_json_names_every_choice() {
    let mut sd = 5u64;
    let rt: Vec<f64> = (0..280).map(|i| i as f64 * DT).collect();
    // exponentially modified Gaussian, tail 0.12 min
    let g: Vec<f64> = rt
        .iter()
        .map(|&t| (-0.5 * ((t - 2.0) / 0.05).powi(2)).exp())
        .collect();
    let mut e = vec![0.0; rt.len()];
    for (i, ei) in e.iter_mut().enumerate() {
        *ei = (0..=i)
            .map(|j| g[j] * (-((i - j) as f64) * DT / 0.12).exp())
            .sum();
    }
    let top = e.iter().copied().fold(0.0, f64::max);
    let y: Vec<f64> = e
        .iter()
        .map(|v| 200.0 + 2.5e4 * v / top)
        .map(|c| c + 1.5 * c.sqrt() * gauss(&mut sd))
        .collect();
    let r = run(&rt, &y);
    assert_eq!(r.peaks.len(), 1, "{:?}", r.peaks);
    let k = &r.peaks[0];
    assert!(
        rt[k.hi] - rt[k.apex] > 2.5 * (rt[k.apex] - rt[k.lo]),
        "{k:?}"
    );
    let j = lr_auto_json(&rt, &r);
    assert!(
        j.starts_with("{\"params\":{\"half\":")
            && j.contains("\"half_auto\":true")
            && j.contains("\"split\":\"drop\"")
            && j.contains("\"end1\":\"background\""),
        "{j}"
    );
}

#[test]
fn parameters_from_the_page_and_a_fixed_half_window() {
    let p = LrAutoParams::from_slice(&[3.0, 30.0, 4.0, 6.0, 0.0, 1.0]);
    assert_eq!(
        p,
        LrAutoParams {
            half: 3,
            valley: 0.3,
            snr: 4.0,
            min_pts: 6,
            despike: false,
            split: Split::Valley
        }
    );
    assert_eq!(LrAutoParams::from_slice(&[]), LrAutoParams::default());
    let (rt, y, _) = trace(&[(2.0, 0.06, 2.0e4)], 1);
    let r = lr_auto_integrate(
        &rt,
        &y,
        0.0,
        7.0,
        LrAutoParams {
            half: 4,
            ..Default::default()
        },
    );
    assert_eq!((r.half, r.half_auto, r.peaks.len()), (4, false, 1));
}

#[test]
fn a_trace_below_zero_never_gives_a_negative_area_and_reports_the_part_cut_off() {
    // a blank subtracted too much: the background sits at -400 counts, the peak still rises 2e4 above it
    for seed in 1..=10 {
        let (rt, y0, _) = trace(&[(3.0, 0.06, 2.0e4)], seed);
        let mut y: Vec<f64> = y0.iter().map(|v| v - 600.0).collect();
        // one point of the flank (2.9 min) over-subtracted far below the background: the median of 3 keeps it out of the detection, the raw area meets it
        y[116] = -1000.0;
        let r = run(&rt, &y);
        assert_eq!(r.peaks.len(), 1, "seed {seed}: {:?}", r.peaks);
        let k = &r.peaks[0];
        // the background is below zero, so each end was clamped (raised toward zero, or lowered onto the trace) and none is above the trace
        assert!(k.base_lo <= y[k.lo] && k.base_hi <= y[k.hi], "{k:?}");
        // at or above zero, unless the trace itself is below zero there: then on the trace
        for (b, i) in [(k.base_lo, k.lo), (k.base_hi, k.hi)] {
            assert!(b >= 0.0 || b == y[i], "{k:?}");
        }
        assert!(
            k.clamp_lo != Clamp::None && k.clamp_hi != Clamp::None,
            "{k:?}"
        );
        // the trace dips under the line between the ends: that part is cut off (`below`), not subtracted from the area
        assert!(
            k.lo < 116 && 116 < k.hi && k.area > 0.0 && k.below > 0.0,
            "{k:?}"
        );
        let line =
            |x: f64| k.base_lo + (k.base_hi - k.base_lo) * (x - rt[k.lo]) / (rt[k.hi] - rt[k.lo]);
        let signed = integrate(&rt, &y, k.lo, k.hi, line).area;
        assert!(
            ((k.area - k.below) - signed).abs() < 1e-9 * k.area,
            "{k:?} {signed}"
        );
        assert!(k.area_zero >= 0.0);
        let j = lr_auto_json(&rt, &r);
        assert!(
            j.contains("\"baseline\":\"engine\"") && !j.contains("\"clamp0\":\"none\""),
            "{j}"
        );
    }
    // noise alone below zero, at a low threshold that lets bumps through: every area is still >= 0
    for seed in 1..=10 {
        let (rt, y0, _) = trace(&[], seed);
        let y: Vec<f64> = y0.iter().map(|v| v - 600.0).collect();
        let p = LrAutoParams {
            snr: 1.0,
            ..Default::default()
        };
        for k in lr_auto_integrate(&rt, &y, 0.0, 7.0, p).peaks {
            assert!(
                k.area >= 0.0 && k.below >= 0.0 && k.area_zero >= 0.0,
                "seed {seed}: {k:?}"
            );
        }
    }
}
