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
