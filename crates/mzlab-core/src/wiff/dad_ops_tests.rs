#![allow(clippy::cast_possible_truncation, clippy::unwrap_used)]
use super::*;

fn grid(
    t0: f64,
    dt: f64,
    n: usize,
    w0: f64,
    dw: f64,
    m: usize,
    f: impl Fn(f64, f64) -> f64,
) -> DadData {
    let time_min: Vec<f64> = (0..n).map(|i| t0 + dt * i as f64).collect();
    let wavelengths_nm: Vec<f64> = (0..m).map(|j| w0 + dw * j as f64).collect();
    let f = &f;
    let matrix_mau = time_min
        .iter()
        .flat_map(|&t| wavelengths_nm.iter().map(move |&w| f(t, w) as f32))
        .collect();
    DadData {
        duration_min: dt * (n - 1) as f64,
        twc_mau: vec![0.0; n],
        wl_start_nm: w0,
        wl_stop_nm: w0 + dw * (m - 1) as f64,
        wl_step_nm: dw,
        time_min,
        wavelengths_nm,
        matrix_mau,
    }
}

#[test]
fn equal_grids_subtract_exactly_and_report_no_warning() {
    let a = grid(0.0, 0.1, 10, 200.0, 2.0, 5, |t, w| t + w);
    let (d, r) = dad_subtract(&a, &a, &SubOpts::default()).unwrap();
    assert!(d.matrix_mau.iter().all(|&v| v == 0.0));
    assert!(r.warnings.is_empty());
    assert!(!r.interp_rt && !r.interp_nm);
    assert_eq!(r.rows, (10, 10));
}

#[test]
fn a_finer_grid_interpolates_b_and_says_so() {
    let a = grid(0.0, 0.05, 21, 200.0, 1.0, 9, |t, w| 2.0 * t + 0.1 * w);
    let b = grid(0.0, 0.1, 11, 200.0, 2.0, 5, |t, w| 2.0 * t + 0.1 * w);
    let (d, r) = dad_subtract(&a, &b, &SubOpts::default()).unwrap();
    assert!(d.matrix_mau.iter().all(|v| v.abs() < 1e-4));
    assert!(r.interp_rt && r.interp_nm);
    assert!(r.warnings.contains(&"rt_step_differs") && r.warnings.contains(&"nm_step_differs"));
}

#[test]
fn the_part_b_does_not_cover_is_dropped() {
    let a = grid(0.0, 0.1, 20, 200.0, 2.0, 10, |_, _| 1.0);
    let b = grid(0.5, 0.1, 10, 204.0, 2.0, 5, |_, _| 1.0);
    let (d, r) = dad_subtract(&a, &b, &SubOpts::default()).unwrap();
    assert_eq!(r.rows, (10, 20));
    assert_eq!(r.cols, (5, 10));
    assert_eq!(d.matrix_mau.len(), 50);
    assert!(r.warnings.contains(&"rt_range_partial") && r.warnings.contains(&"nm_range_partial"));
}

#[test]
fn no_overlap_is_an_error() {
    let a = grid(0.0, 0.1, 5, 200.0, 2.0, 5, |_, _| 1.0);
    let b = grid(50.0, 0.1, 5, 200.0, 2.0, 5, |_, _| 1.0);
    assert!(dad_subtract(&a, &b, &SubOpts::default()).is_err());
}

#[test]
fn shift_and_scale_are_applied_and_reported() {
    let a = grid(0.0, 0.1, 20, 200.0, 2.0, 4, |t, _| (t - 1.0).abs());
    let b = grid(0.0, 0.1, 20, 200.0, 2.0, 4, |t, _| 0.5 * (t - 0.5).abs());
    let o = SubOpts {
        scale: Scale::Factor(2.0),
        shift_min: 0.5,
        ..SubOpts::default()
    };
    let (d, r) = dad_subtract(&a, &b, &o).unwrap();
    assert!(d.matrix_mau.iter().all(|v| v.abs() < 1e-4));
    assert!(r.warnings.contains(&"shift_applied") && r.warnings.contains(&"scale_applied"));
    let fit = SubOpts {
        scale: Scale::Fit,
        shift_min: 0.5,
        ..SubOpts::default()
    };
    assert!((dad_subtract(&a, &b, &fit).unwrap().1.factor - 2.0).abs() < 1e-3);
}

#[test]
fn the_report_json_has_stable_keys() {
    let a = grid(0.0, 0.1, 5, 200.0, 2.0, 5, |_, _| 1.0);
    let j = dad_subtract(&a, &a, &SubOpts::default())
        .unwrap()
        .1
        .to_json();
    for k in [
        "a_rt",
        "b_nm",
        "rows",
        "cols",
        "shift_min",
        "scale",
        "factor",
        "interp_rt",
        "warnings",
    ] {
        assert!(j.contains(&format!("\"{k}\":")), "{k}");
    }
}

#[test]
fn every_view_carries_the_report() {
    use crate::itemroutes::{pda_subtract_data, PdaView};
    let a = grid(0.0, 0.1, 10, 200.0, 2.0, 5, |t, w| t + w);
    let o = SubOpts::default();
    for v in [
        PdaView::Map,
        PdaView::Spectrum(0.2, 0.4),
        PdaView::Wavelength(204.0, 206.0),
    ] {
        let j = pda_subtract_data(&a, &a, &o, v).unwrap();
        assert!(
            j.contains("\"sub\":{\"a_rt\":") && j.ends_with("}}"),
            "{v:?}"
        );
    }
}

fn peaks(shift: f64) -> DadData {
    grid(0.0, 0.02, 501, 220.0, 10.0, 3, |t, _| {
        [2.0, 5.0, 8.0]
            .iter()
            .map(|c| 50.0 * (-((t - c - shift) / 0.05).powi(2) / 2.0).exp())
            .sum()
    })
}

#[test]
fn an_automatic_shift_removes_the_false_peak_and_reports_its_evidence() {
    let (a, b) = (peaks(0.0), peaks(0.1));
    let (hand, _) = dad_subtract(&a, &b, &SubOpts::default()).unwrap();
    let auto = SubOpts {
        auto_shift: true,
        ..SubOpts::default()
    };
    let (d, r) = dad_subtract(&a, &b, &auto).unwrap();
    let top = |m: &DadData| m.matrix_mau.iter().fold(0.0f32, |t, v| t.max(v.abs()));
    assert!(
        top(&hand) > 20.0 && top(&d) < 5.0,
        "{} {}",
        top(&hand),
        top(&d)
    );
    let found = r.align.as_ref().unwrap();
    assert!((r.shift_min + 0.1).abs() < 0.005 && found.shift_min == r.shift_min);
    assert!(r.warnings.contains(&"shift_applied"));
    assert!(r.to_json().contains("\"align\":{\"method\":\"twc_xcorr\""));
}

#[test]
fn a_shift_by_hand_has_no_align_member() {
    let a = peaks(0.0);
    let (_, r) = dad_subtract(
        &a,
        &a,
        &SubOpts {
            shift_min: 0.02,
            ..SubOpts::default()
        },
    )
    .unwrap();
    assert!(r.align.is_none() && !r.to_json().contains("align"));
}
