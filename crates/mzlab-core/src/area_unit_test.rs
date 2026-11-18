use super::*;
use crate::baseline::{peak_area_json, Baseline};
use crate::peakq_lrauto::{lr_auto_integrate, LrAutoParams};

fn gauss() -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..400).map(|i| 1.0 + i as f64 * 0.005).collect();
    let y = rt
        .iter()
        .map(|t| 1.0e4 * (-((t - 2.0) / 0.05).powi(2) / 2.0).exp())
        .collect();
    (rt, y)
}

fn field(j: &str, key: &str) -> f64 {
    let s = j.split(&format!("\"{key}\":")).nth(1).unwrap();
    s.split([',', '}']).next().unwrap().parse().unwrap()
}

#[test]
fn the_default_unit_is_counts_seconds_like_peak_area() {
    let (rt, y) = gauss();
    let r = lr_auto_integrate(&rt, &y, 1.0, 3.0, LrAutoParams::default());
    assert_eq!(r.peaks.len(), 1);
    let manual = peak_area_json(&rt, &y, 1.5, 2.5, Baseline::Zero).unwrap();
    let (auto_s, auto_min) = (
        field(&lr_auto_json_in(&rt, &r, AreaUnit::default()), "area_zero"),
        field(&lr_auto_json_in(&rt, &r, AreaUnit::Minutes), "area_zero"),
    );
    // Gaussian: 1e4 * 0.05 * sqrt(2 pi) = 1253.3 counts x min = 75198.8 counts x s
    assert!(
        (auto_s - 60.0 * auto_min).abs() < 1e-6 * auto_s,
        "{auto_s} {auto_min}"
    );
    assert!(
        (auto_s - field(&manual, "area")).abs() < 1e-3 * auto_s,
        "{auto_s} vs {manual}"
    );
}

#[test]
fn minutes_is_the_old_output_byte_for_byte() {
    let (rt, y) = gauss();
    let r = lr_auto_integrate(&rt, &y, 1.0, 3.0, LrAutoParams::default());
    assert_eq!(
        lr_auto_json_in(&rt, &r, AreaUnit::Minutes),
        crate::peakq_lrauto::lr_auto_json(&rt, &r)
    );
}

#[test]
fn the_unit_names_and_numbers() {
    assert_eq!(AreaUnit::parse("s"), Ok(AreaUnit::Seconds));
    assert_eq!(AreaUnit::parse("min"), Ok(AreaUnit::Minutes));
    assert!(AreaUnit::parse("h").is_err());
    assert_eq!(AreaUnit::from_number(None), AreaUnit::Seconds);
    assert_eq!(AreaUnit::from_number(Some(1.0)), AreaUnit::Minutes);
}
