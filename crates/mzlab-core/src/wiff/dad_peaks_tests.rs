#![allow(
    clippy::cast_possible_truncation,
    clippy::unwrap_used,
    clippy::indexing_slicing
)]
use super::*;

/// A run of 10 minutes at 0.02 min, 200..=400 nm: a solvent background that peaks at 210 nm at every time, plus a peak at each of `(rt, nm, height)`.
fn run(peaks: &[(f64, f64, f64)]) -> DadData {
    let time_min: Vec<f64> = (0..501).map(|i| f64::from(i) * 0.02).collect();
    let wavelengths_nm: Vec<f64> = (0..101).map(|j| 200.0 + 2.0 * f64::from(j)).collect();
    let matrix_mau: Vec<f32> = time_min
        .iter()
        .flat_map(|&t| {
            wavelengths_nm.iter().map(move |&w| {
                let solvent = 40.0 * (-((w - 210.0) / 15.0).powi(2)).exp();
                let analytes: f64 = peaks
                    .iter()
                    .map(|&(rt, nm, h)| {
                        h * (-((t - rt) / 0.08).powi(2)).exp() * (-((w - nm) / 20.0).powi(2)).exp()
                    })
                    .sum();
                (solvent + analytes + 0.01 * ((t * 977.0 + w * 13.0).sin())) as f32
            })
        })
        .collect();
    let twc_mau = time_min
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let n = wavelengths_nm.len();
            matrix_mau[i * n..(i + 1) * n]
                .iter()
                .map(|v| f64::from(*v))
                .sum::<f64>()
                / n as f64
        })
        .collect();
    DadData {
        duration_min: 10.0,
        twc_mau,
        wl_start_nm: 200.0,
        wl_stop_nm: 400.0,
        wl_step_nm: 2.0,
        time_min,
        wavelengths_nm,
        matrix_mau,
    }
}

#[test]
fn each_peak_gets_its_own_lambda_max_not_the_solvents() {
    let d = run(&[(2.0, 254.0, 120.0), (5.0, 280.0, 300.0)]);
    let p = uv_peaks(&d, &PeakOpts::default()).unwrap();
    assert_eq!(p.len(), 2, "{p:?}");
    assert!((p[0].rt_min - 2.0).abs() < 0.03 && (p[1].rt_min - 5.0).abs() < 0.03);
    // the solvent peaks at 210 nm at every time: the edge rows are taken off, so it does not win
    assert!((p[0].lambda_max_nm - 254.0).abs() <= 2.0, "{p:?}");
    assert!((p[1].lambda_max_nm - 280.0).abs() <= 2.0, "{p:?}");
    assert!(p[1].height > p[0].height && p[0].abs_max > 100.0);
}

#[test]
fn a_band_trace_and_the_peak_cap_are_honoured() {
    let d = run(&[(2.0, 254.0, 120.0), (5.0, 400.0, 300.0)]);
    // at 254 nm the second analyte is far below the first: with the cap of one, only the highest of the band comes out
    let o = PeakOpts {
        nm: Some((252.0, 256.0)),
        max: 1,
        snr_min: 3.0,
    };
    let p = uv_peaks(&d, &o).unwrap();
    assert_eq!(p.len(), 1);
    assert!((p[0].rt_min - 2.0).abs() < 0.03);
}

#[test]
fn a_flat_run_has_no_peak_and_an_empty_matrix_is_none() {
    let flat = run(&[]);
    assert_eq!(uv_peaks(&flat, &PeakOpts::default()).unwrap().len(), 0);
    let mut empty = run(&[]);
    empty.time_min.clear();
    assert!(uv_peaks(&empty, &PeakOpts::default()).is_none());
}
