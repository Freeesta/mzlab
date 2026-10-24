//! Labels of the main peaks: a synthetic TIC with three peaks (RT and apex), a UV spectrum with two maxima at different wavelengths, the base peak of an MS scan.

use mzlab_core::etichette_picchi::{main_peak, select};

fn gauss(t: f64, c: f64, w: f64, h: f64) -> f64 {
    h * (-((t - c) / w).powi(2) / 2.0).exp()
}

fn trace(centers: &[(f64, f64)]) -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..600).map(|i| i as f64 * 0.02).collect();
    let y = rt
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            20.0 + (i % 7) as f64
                + centers
                    .iter()
                    .map(|&(c, h)| gauss(t, c, 0.12, h))
                    .sum::<f64>()
        })
        .collect();
    (rt, y)
}

#[test]
fn tic_with_three_peaks_labels_each_apex() {
    let (rt, y) = trace(&[(3.0, 5000.0), (6.0, 20000.0), (9.0, 8000.0)]);
    let l = select(&rt, &y, 8, 0.05, 5.0, false);
    assert_eq!(l.len(), 3);
    for (lab, c) in l.iter().zip([3.0, 6.0, 9.0]) {
        assert!((lab.rt - c).abs() < 0.05, "{} vs {}", lab.rt, c);
    }
    assert!(l[0].apex_i < l[1].apex_i && l[1].apex_i < l[2].apex_i);
}

#[test]
fn the_count_and_the_relative_threshold_limit_the_labels() {
    let (rt, y) = trace(&[(3.0, 500.0), (6.0, 20000.0), (9.0, 8000.0)]);
    assert_eq!(
        select(&rt, &y, 8, 0.05, 5.0, false).len(),
        2,
        "the 2.5 % peak is under the relative threshold"
    );
    let one = select(&rt, &y, 1, 0.05, 5.0, false);
    assert_eq!(one.len(), 1);
    assert!((one[0].rt - 6.0).abs() < 0.05, "the tallest is kept");
    assert!(select(&rt, &y, 0, 0.05, 5.0, false).is_empty());
}

#[test]
fn a_flat_noise_trace_has_no_label() {
    let rt: Vec<f64> = (0..300).map(|i| i as f64 * 0.02).collect();
    let mut seed = 12345u64;
    let y: Vec<f64> = (0..300)
        .map(|_| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            100.0 + (seed >> 33) as f64 / (1u64 << 31) as f64 * 4.0
        })
        .collect();
    assert!(select(&rt, &y, 8, 0.05, 5.0, false).is_empty());
}

#[test]
fn uv_spectrum_maximum_is_the_tallest_wavelength() {
    let nm: Vec<f64> = (0..200).map(|i| 200.0 + i as f64 * 2.0).collect();
    let a: Vec<f64> = nm
        .iter()
        .map(|&w| gauss(w, 254.0, 12.0, 80.0) + gauss(w, 330.0, 15.0, 30.0))
        .collect();
    let (w, v) = main_peak(&nm, &a, true).unwrap();
    assert!((w - 254.0).abs() < 1.0 && (v - 80.0).abs() < 1.0);
    let b: Vec<f64> = nm
        .iter()
        .map(|&w| gauss(w, 254.0, 12.0, 30.0) + gauss(w, 330.0, 15.0, 80.0))
        .collect();
    let (w2, _) = main_peak(&nm, &b, true).unwrap();
    assert!(
        (w2 - 330.0).abs() < 1.0,
        "a second trace has its maximum elsewhere: {w2}"
    );
}

#[test]
fn ms_base_peak_is_not_refined() {
    let (mz, i) = main_peak(&[100.1, 301.14, 450.2], &[10.0, 900.0, 40.0], false).unwrap();
    assert_eq!((mz, i), (301.14, 900.0));
    assert!(main_peak(&[1.0, 2.0], &[0.0, -1.0], false).is_none());
    assert!(main_peak(&[], &[], true).is_none());
}

fn run(minutes: f64, centers: &[(f64, f64)]) -> (Vec<f64>, Vec<f64>) {
    let n = (minutes / 0.02) as usize;
    let rt: Vec<f64> = (0..n).map(|i| i as f64 * 0.02).collect();
    let y = rt
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            20.0 + (i % 7) as f64
                + centers
                    .iter()
                    .map(|&(c, h)| gauss(t, c, 0.12, h))
                    .sum::<f64>()
        })
        .collect();
    (rt, y)
}

#[test]
fn solvent_front_and_end_of_a_long_run_are_not_labelled() {
    let (rt, y) = run(20.0, &[(2.4, 9000.0), (14.3, 6000.0), (18.4, 9000.0)]);
    let l = select(&rt, &y, 8, 0.05, 5.0, true);
    assert_eq!(l.len(), 1, "{l:?}");
    assert!((l[0].rt - 14.3).abs() < 0.05);
    assert_eq!(
        select(&rt, &y, 8, 0.05, 5.0, false).len(),
        3,
        "a spectrum keeps every peak"
    );
}

#[test]
fn a_short_run_keeps_a_useful_zone() {
    // 4 min: skip 0.6 min at the start and 1 min at the end, not 3 and 5
    let (rt, y) = run(4.0, &[(0.3, 9000.0), (2.0, 5000.0), (3.6, 9000.0)]);
    let l = select(&rt, &y, 8, 0.05, 5.0, true);
    assert_eq!(l.len(), 1, "{l:?}");
    assert!((l[0].rt - 2.0).abs() < 0.05);
}

#[test]
fn a_huge_solvent_peak_does_not_hide_the_real_ones_by_rel_min() {
    let (rt, y) = run(20.0, &[(1.0, 100_000.0), (8.0, 3000.0), (12.0, 2000.0)]);
    let l = select(&rt, &y, 8, 0.05, 5.0, true);
    assert!(l.iter().all(|x| x.rt >= 3.0), "{l:?}");
    for c in [8.0, 12.0] {
        assert!(
            l.iter().any(|x| (x.rt - c).abs() < 0.05),
            "{c} min is labelled: {l:?}"
        );
    }
}

#[test]
fn everything_out_of_the_zones_gives_no_label() {
    let (rt, y) = run(20.0, &[(1.0, 9000.0), (19.0, 9000.0)]);
    assert!(select(&rt, &y, 8, 0.05, 5.0, true).is_empty());
}
