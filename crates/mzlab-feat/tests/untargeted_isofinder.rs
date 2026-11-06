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

mod selection {
    use super::*;
    use mzlab_feat::untargeted::isofinder::select::{detect, ALT_MARGIN, MIN_ALT_QUALITY};

    const C13: f64 = 1.003354831;

    fn spectrum(m0: f64, step: f64, heights: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let mz = (0..heights.len()).map(|i| m0 + i as f64 * step).collect();
        (mz, heights.iter().map(|h| h * 1e6).collect())
    }

    #[test]
    fn one_valid_charge_gives_one_pattern_with_probability_one() {
        let (mz, y, mz0, _) = case("c10_z1");
        let d = detect(&engine(true), &mz, &y, mz0, 1).expect("detected");
        assert_eq!(d.best_charge, 1);
        assert_eq!(d.patterns.len(), 1);
        assert_eq!(d.probabilities.len(), 1);
        assert!((d.patterns[0].probability - 1.0).abs() < 1e-12);
        assert_eq!(d.patterns[0].peaks.len(), mz.len());
        let p = &d.patterns[0];
        assert!((p.score - p.eval.quality * p.eval.intensity_agreement).abs() < 1e-12);
    }

    #[test]
    fn a_doubly_charged_envelope_wins_as_charge_two() {
        let (mz, y, mz0, _) = case("c40_z2");
        let d = detect(&engine(true), &mz, &y, mz0, 1).expect("detected");
        assert_eq!(d.best_charge, 2);
        assert_eq!(d.patterns[0].charge, 2);
    }

    #[test]
    fn close_charges_are_ranked_and_the_probabilities_sum_to_one() {
        // half-step peaks with a full 13C ladder on top: charges 1 and 2 both find support; the winner goes by `raw` (quality and the peak-count reward), so its probability, which is quality only, need not be the largest
        let (mz, y) = spectrum(400.0, C13 / 2.0, &[1.0, 0.45, 0.4, 0.2, 0.1, 0.04, 0.02]);
        let d = detect(&engine(false), &mz, &y, 400.0, 1).expect("detected");
        let sum: f64 = d.probabilities.iter().map(|p| p.1).sum();
        assert_eq!(d.probabilities.len(), 2, "{:?}", d.probabilities);
        assert_eq!(d.patterns.len(), 2);
        assert!((sum - 1.0).abs() < 1e-12, "{:?}", d.probabilities);
        assert_eq!(d.patterns[0].charge, d.best_charge);
        let best = d.patterns[0].eval.quality;
        for p in &d.patterns[1..] {
            assert!(
                p.eval.quality >= best - ALT_MARGIN && p.eval.quality >= MIN_ALT_QUALITY,
                "{p:?}"
            );
            assert!(p.eval.raw <= d.patterns[0].eval.raw);
        }
        assert!(d.patterns.len() <= d.probabilities.len());
    }

    #[test]
    fn no_charge_scoring_gives_none() {
        let (mz, y, mz0, z) = case("no_m1");
        assert!(detect(&engine(true), &mz, &y, mz0, 1).is_none(), "z={z}");
        assert!(detect(&engine(true), &[100.0], &[1e5], 100.0, 1).is_none());
    }
}
