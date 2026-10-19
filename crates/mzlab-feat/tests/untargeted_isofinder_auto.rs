//! Automatic detection of heavy elements in the isotope finder: synthetic [M+H]+ patterns with exact fine structure (`tools/isofinder_riferimento.py`).

use mzlab_feat::untargeted::isofinder::auto::{detect, detection_window, DEFAULT_CANDIDATES};
use mzlab_feat::untargeted::isofinder::select;
use mzlab_feat::untargeted::isofinder::{Config, ElementMode, Engine, Tol};
use serde_json::Value;

fn cases() -> Vec<Value> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/untargeted/isofinder/elementi.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn spectrum(c: &Value) -> (Vec<f64>, Vec<f64>) {
    let v = |k: &str| {
        c[k].as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect::<Vec<f64>>()
    };
    (v("mz"), v("y"))
}

fn found(c: &Value) -> Vec<String> {
    let (mz, y) = spectrum(c);
    let sig: Vec<(f64, f64)> = mz.iter().copied().zip(y.iter().copied()).collect();
    detect(&sig, 1, &Tol::default(), &DEFAULT_CANDIDATES)
        .into_iter()
        .map(|f| f.symbol)
        .collect()
}

fn case(name: &str) -> Value {
    cases().into_iter().find(|c| c["name"] == name).unwrap()
}

#[test]
fn the_true_element_ranks_first() {
    for (name, first, atoms) in [
        ("cl2", "Cl", 2),
        ("br1", "Br", 1),
        ("s2", "S", 2),
        ("si1", "Si", 2),
    ] {
        let (mz, y) = spectrum(&case(name));
        let sig: Vec<(f64, f64)> = mz.iter().copied().zip(y.iter().copied()).collect();
        let out = detect(&sig, 1, &Tol::default(), &DEFAULT_CANDIDATES);
        assert_eq!(out[0].symbol, first, "{name}: {out:?}");
        assert_eq!(out[0].atoms, atoms, "{name}: {out:?}");
        assert!(
            out.windows(2).all(|w| w[0].confidence >= w[1].confidence),
            "{name}: {out:?}"
        );
        assert!(
            out.iter()
                .all(|f| f.confidence >= 0.2 && f.confidence <= 1.0 && f.atoms >= 1),
            "{name}: {out:?}"
        );
    }
}

#[test]
fn a_plain_chno_pattern_finds_nothing() {
    assert!(found(&case("chno")).is_empty());
}

#[test]
fn a_doubly_charged_pattern_is_read_with_its_charge() {
    let (mz, y) = spectrum(&case("cl2"));
    let sig: Vec<(f64, f64)> = mz
        .iter()
        .map(|m| (m - 1.007276466 + 2.0 * 1.007276466) / 2.0)
        .zip(y.iter().copied())
        .collect();
    // the same neutral series as a doubly charged ion: half the m/z spacings, read as charge 2 they are the same neutral spacings
    let out = detect(&sig, 2, &Tol::default(), &DEFAULT_CANDIDATES);
    assert_eq!(
        out.first().map(|f| f.symbol.as_str()),
        Some("Cl"),
        "{out:?}"
    );
}

#[test]
fn too_few_signals_or_no_candidates_find_nothing() {
    assert!(detect(&[(100.0, 1e6)], 1, &Tol::default(), &DEFAULT_CANDIDATES).is_empty());
    assert!(detect(&[], 1, &Tol::default(), &DEFAULT_CANDIDATES).is_empty());
    let (mz, y) = spectrum(&case("cl2"));
    let sig: Vec<(f64, f64)> = mz.iter().copied().zip(y.iter().copied()).collect();
    assert!(detect(&sig, 1, &Tol::default(), &[]).is_empty());
    assert!(detect(&sig, 1, &Tol::default(), &["Xx"]).is_empty());
}

#[test]
fn the_detection_window_pads_by_two_and_a_half_over_the_charge() {
    let (mz, y) = (vec![97.0, 99.0, 100.0, 102.0, 103.0], vec![1.0; 5]);
    assert_eq!(detection_window(&mz, &y, 100.0, 100.0, 1).len(), 3);
    assert_eq!(detection_window(&mz, &y, 100.0, 100.0, 2).len(), 2);
}

#[test]
fn the_finder_reports_the_elements_and_rescoring_keeps_the_winner() {
    let c = case("cl2");
    let (mz, y) = spectrum(&c);
    let mz0 = mz[0];
    let auto = Engine::new(Config::default()).unwrap();
    let d = select::detect(&auto, &mz, &y, mz0, 1).expect("detected");
    assert_eq!(d.best_charge, 1);
    assert_eq!(
        d.composition.first().map(|f| f.symbol.as_str()),
        Some("Cl"),
        "{:?}",
        d.composition
    );
    let user = Engine::new(Config {
        mode: ElementMode::UserDefined,
        ..Config::default()
    })
    .unwrap();
    let u = select::detect(&user, &mz, &y, mz0, 1).expect("detected");
    assert!(u.composition.is_empty());
    assert_eq!(u.best_charge, d.best_charge);
    // the wider upper bound can only make the intensities agree better with the envelope
    assert!(
        d.patterns[0].eval.intensity_agreement >= u.patterns[0].eval.intensity_agreement - 1e-12
    );
}
