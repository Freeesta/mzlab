//! Isotope finder (one charge): candidates and score on synthetic spectra with an exact binomial 13C envelope (`tools/isofinder_riferimento.py`).

use mzlab_feat::untargeted::isofinder::{Config, Engine, Tol};
use serde_json::Value;

fn cases() -> Vec<Value> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/untargeted/isofinder/casi.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn vec_of(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn case(name: &str) -> (Vec<f64>, Vec<f64>, f64, i32) {
    let c = cases().into_iter().find(|c| c["name"] == name).unwrap();
    (
        vec_of(&c["mz"]),
        vec_of(&c["y"]),
        c["mz0"].as_f64().unwrap(),
        c["z"].as_i64().unwrap() as i32,
    )
}

fn engine(require_c13: bool) -> Engine {
    Engine::new(Config {
        require_c13,
        ..Config::default()
    })
    .unwrap()
}

#[test]
fn singly_charged_envelopes_are_found_and_fit_the_reference() {
    for name in ["c10_z1", "c20_z1"] {
        let (mz, y, mz0, z) = case(name);
        let e = engine(true);
        let cands = e.candidates(z, &mz, &y, mz0);
        assert_eq!(cands.len(), mz.len(), "{name}: every peak is a candidate");
        let ev = e
            .score_charge(z, mz0, &cands, 1)
            .unwrap_or_else(|| panic!("{name}: no score"));
        assert!(ev.raw > 0.0 && ev.coverage > 0.99, "{name}: {ev:?}");
        assert!(
            ev.carbon_fit > 0.98 && ev.intensity_agreement > 0.9 && ev.spacing_consistency > 0.99,
            "{name}: {ev:?}"
        );
        let reference = cases().into_iter().find(|c| c["name"] == name).unwrap()["cos_poisson"]
            .as_f64()
            .unwrap();
        assert!(
            (ev.carbon_fit - reference).abs() < 0.03,
            "{name}: {} vs {reference}",
            ev.carbon_fit
        );
        assert_eq!(ev.kept.len(), mz.len(), "{name}");
    }
}

#[test]
fn a_doubly_charged_envelope_scores_for_charge_two_and_not_for_one() {
    let (mz, y, mz0, _) = case("c40_z2");
    let e = engine(true);
    let c2 = e.candidates(2, &mz, &y, mz0);
    let ev2 = e.score_charge(2, mz0, &c2, 1).expect("z=2");
    assert!(ev2.raw > 0.0 && ev2.self_consistency > 0.0, "{ev2:?}");
    // as a singly charged series the half-step peaks are not on the 13C grid, so the charge does not count better
    let c1 = e.candidates(1, &mz, &y, mz0);
    let ev1 = e.score_charge(1, mz0, &c1, 1);
    assert!(
        ev1.as_ref().is_none_or(|v| v.raw < ev2.raw),
        "{ev1:?} vs {ev2:?}"
    );
}

#[test]
fn require_c13_rejects_a_series_without_the_m_plus_1_peak() {
    let (mz, y, mz0, z) = case("no_m1");
    let strict = engine(true);
    let cands = strict.candidates(z, &mz, &y, mz0);
    assert!(strict.score_charge(z, mz0, &cands, 1).is_none());
    let loose = engine(false);
    let cands = loose.candidates(z, &mz, &y, mz0);
    assert!(loose.score_charge(z, mz0, &cands, 1).is_some());
}

#[test]
fn a_lone_peak_and_unrelated_noise_give_no_score() {
    let e = engine(true);
    let (mz, y) = (vec![100.0, 117.3, 150.77], vec![1e5, 5e4, 3e4]);
    let cands = e.candidates(1, &mz, &y, 100.0);
    assert_eq!(cands.len(), 1);
    assert!(e.score_charge(1, 100.0, &cands, 1).is_none());
    assert!(e.candidates(1, &[], &[], 100.0).is_empty());
}

#[test]
fn the_tolerance_is_the_larger_of_the_absolute_and_the_ppm_part() {
    let t = Tol::default();
    assert_eq!(t.for_mass(100.0), 0.005);
    assert!((t.for_mass(1000.0) - 0.015).abs() < 1e-12);
    assert!(t.within(500.0, 500.0074) && !t.within(500.0, 500.0076));
}

#[test]
fn unknown_elements_and_bad_charges_are_refused() {
    assert!(Engine::new(Config {
        elements: vec!["Xx".into()],
        ..Config::default()
    })
    .is_err());
    assert!(Engine::new(Config {
        max_charge: 0,
        ..Config::default()
    })
    .is_err());
}
