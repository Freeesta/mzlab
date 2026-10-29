//! The pressure of an analog trace: only a declared pressure unit is converted to bar, any other unit is shown as the file gives it.

use mzlab_core::analog::{analog_json, pressure_to_bar};

#[test]
fn only_a_declared_pressure_unit_is_converted() {
    let close = |a: &[f64], b: &[f64]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3);
    assert!(close(
        &pressure_to_bar(&[2.78e7], "Pascal").unwrap(),
        &[278.0]
    ));
    assert!(close(
        &pressure_to_bar(&[250_000.0], "mbar").unwrap(),
        &[250.0]
    ));
    assert!(close(
        &pressure_to_bar(&[4000.0], "psi").unwrap(),
        &[275.7904]
    ));
    // no magic factor: 2780 bar is shown as 2780 bar
    assert!(close(
        &pressure_to_bar(&[2.78e8], "pascal").unwrap(),
        &[2780.0]
    ));
    for u in ["number of detector counts", "", "unknown", "paper"] {
        assert!(pressure_to_bar(&[2700.0, 6000.0], u).is_none(), "{u}");
    }
}

#[test]
fn a_pressure_in_counts_is_shown_as_declared() {
    let (t, y) = ([0.5, 5.6, 13.0, 15.0], [2700.0, 2100.0, 1800.0, 6000.0]);
    let j = analog_json(
        3,
        "pressure",
        "Pump Pressure 1",
        "number of detector counts",
        &t,
        &y,
        100,
    );
    assert!(
        j.contains("\"unit\":\"number of detector counts\"")
            && j.contains("\"y\":[2700,2100,1800,6000]")
            && !j.contains("unit_note"),
        "{j}"
    );
    let j = analog_json(
        3,
        "pressure",
        "Pump Pressure 1",
        "pascal",
        &t,
        &[2.7e7, 2.1e7, 1.8e7, 6e7],
        100,
    );
    assert!(
        j.contains("\"unit\":\"bar\"") && j.contains("\"y\":[270,210,180,600]"),
        "{j}"
    );
}

/// A pump pressure like the one Xcalibur shows for a 35 min run (about 138 bar at 0.5 min, a dip to about 65 at 29.3) with a 3 bar pump-stroke ripple.
fn xcalibur_like(k: f64) -> (Vec<f64>, Vec<f64>) {
    let knots = [
        (0.0, 138.0),
        (4.0, 140.0),
        (16.0, 148.0),
        (21.0, 148.0),
        (22.0, 146.0),
        (29.3, 65.0),
        (32.0, 140.0),
        (35.0, 138.0),
    ];
    let t: Vec<f64> = (0..60000).map(|i| 35.0 * f64::from(i) / 59999.0).collect();
    let y = t
        .iter()
        .map(|&x| {
            let w = knots
                .windows(2)
                .find(|w| x <= w[1].0)
                .unwrap_or(&knots[6..8]);
            let base = w[0].1 + (w[1].1 - w[0].1) * (x - w[0].0) / (w[1].0 - w[0].0);
            (base + if (x / 0.05).fract() < 0.5 { 1.5 } else { -1.5 }) * k
        })
        .collect();
    (t, y)
}

#[test]
fn the_displayed_pressure_keeps_its_magnitude_and_its_ripple() {
    for (unit, k) in [("pascal", 1e5), ("psi", 1.0 / 0.0689476), ("bar", 1.0)] {
        let (t, y) = xcalibur_like(k);
        let j: serde_json::Value = serde_json::from_str(&analog_json(
            1,
            "pressure",
            "LoadingPump_Pressure",
            unit,
            &t,
            &y,
            2000,
        ))
        .unwrap();
        assert_eq!(j["unit"], "bar", "{unit}");
        let at = |lo: f64, hi: f64| -> Vec<f64> {
            j["t"]
                .as_array()
                .unwrap()
                .iter()
                .zip(j["y"].as_array().unwrap())
                .filter(|(x, _)| (lo..hi).contains(&x.as_f64().unwrap()))
                .map(|(_, v)| v.as_f64().unwrap())
                .collect()
        };
        let near_start = at(0.4, 0.6);
        assert!(
            near_start.iter().all(|v| (v - 138.0).abs() < 3.0),
            "{unit}: {near_start:?}"
        );
        let dip = at(28.5, 30.5).into_iter().fold(f64::INFINITY, f64::min);
        assert!((dip - 63.5).abs() < 1.0, "{unit}: {dip}");
        let mid = at(10.0, 10.5);
        let (lo, hi) = (
            mid.iter().copied().fold(f64::INFINITY, f64::min),
            mid.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        assert!(hi - lo > 2.9, "{unit}: ripple {lo}..{hi}");
    }
}
