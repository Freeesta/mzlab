//! `untargeted::formula_id`: candidate compositions of known molecules ([M+H]+ and a doubly charged ion) whose spectra come from the natural isotope
//! abundances (`tools/formula_id_riferimento.py`), the choice of the program, the confidence and the JSON `formula_id/1`.
#![cfg(feature = "untargeted")]

use mzlab_feat::untargeted::formula_id::{identify, to_json, ChargeSource, Opts};
use serde_json::Value;

fn cases() -> Vec<Value> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/untargeted/formula_id/casi.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn case(name: &str) -> (Vec<f64>, Vec<f64>, Value) {
    let c = cases().into_iter().find(|c| c["name"] == name).unwrap();
    let v = |k: &str| {
        c[k].as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect::<Vec<f64>>()
    };
    (v("mz"), v("y"), c)
}

#[test]
fn the_true_composition_is_found_and_the_confidences_add_up() {
    for name in [
        "glucose",
        "caffeine",
        "paracetamol",
        "atrazine",
        "bromazepam",
    ] {
        let (mz, y, c) = case(name);
        let r = identify(&mz, &y, mz[0], &Opts::default()).unwrap();
        let want = c["neutral"].as_str().unwrap();
        let at = r
            .candidates
            .iter()
            .position(|k| k.neutral.as_deref() == Some(want));
        assert!(
            at.is_some_and(|i| i < 3),
            "{name}: {want} not in the first 3: {:?}",
            r.candidates.iter().map(|k| &k.formula).collect::<Vec<_>>()
        );
        assert_eq!(r.charge, 1, "{name}");
        assert_eq!(r.charge_source, ChargeSource::Finder, "{name}");
        let sum: f64 = r.candidates.iter().map(|k| k.confidence).sum();
        assert!((sum - 1.0).abs() < 1e-9, "{name}: {sum}");
        assert!(
            r.candidates.windows(2).all(|w| w[0].score >= w[1].score),
            "{name}"
        );
        assert_eq!(r.chosen, Some(0));
    }
}

#[test]
fn the_heavy_elements_of_the_spectrum_open_their_ranges() {
    for (name, el) in [("atrazine", "Cl"), ("bromazepam", "Br")] {
        let (mz, y, _) = case(name);
        let r = identify(&mz, &y, mz[0], &Opts::default()).unwrap();
        assert!(
            r.elements_found.iter().any(|f| f.symbol == el),
            "{name}: {:?}",
            r.elements_found
        );
        // the element is in the first formula: its isotope peaks are in the pattern
        assert!(
            r.candidates[0].formula.contains(el),
            "{name}: {}",
            r.candidates[0].formula
        );
    }
    // a CHNO molecule finds none
    let (mz, y, _) = case("glucose");
    assert!(identify(&mz, &y, mz[0], &Opts::default())
        .unwrap()
        .elements_found
        .is_empty());
}

#[test]
fn the_finder_decides_the_charge_and_the_formulas_are_those_of_the_ion() {
    let (mz, y, _) = case("peptide_2plus");
    let r = identify(&mz, &y, mz[0], &Opts::default()).unwrap();
    assert_eq!((r.charge, r.charge_source), (2, ChargeSource::Finder));
    assert!(r.charge_probability.is_some_and(|p| p > 0.5));
    let at = r.candidates.iter().position(|k| k.formula == "C34H52N8O9");
    assert!(
        at.is_some_and(|i| i < 3),
        "{:?}",
        r.candidates.iter().map(|k| &k.formula).collect::<Vec<_>>()
    );
}

#[test]
fn the_given_charge_wins_and_without_a_pattern_only_the_mass_ranks() {
    let (mz, y, _) = case("caffeine");
    let given = identify(
        &mz,
        &y,
        mz[0],
        &Opts {
            z: Some(1),
            ..Opts::default()
        },
    )
    .unwrap();
    assert_eq!(given.charge_source, ChargeSource::Given);
    // a lone peak: no pattern, the charge is that of the ion type and the candidates are ranked by their mass error
    let lone = identify(&[195.0877], &[1e6], 195.0877, &Opts::default()).unwrap();
    assert_eq!(
        (lone.charge, lone.charge_source),
        (1, ChargeSource::Assumed)
    );
    assert!(lone.observed.is_empty() && !lone.candidates.is_empty());
    assert!(lone
        .candidates
        .iter()
        .all(|k| k.why.iter().any(|w| w.code == "massOnly")));
    let sum: f64 = lone.candidates.iter().map(|k| k.confidence).sum();
    assert!((sum - 1.0).abs() < 1e-9);
}

#[test]
fn the_explanation_and_the_json_say_why() {
    let (mz, y, _) = case("caffeine");
    let r = identify(&mz, &y, mz[0], &Opts::default()).unwrap();
    let best = &r.candidates[r.chosen.unwrap()];
    for code in ["massError", "rdb", "isotopeFit", "rulesOk"] {
        assert!(best.why.iter().any(|w| w.code == code), "{code}");
    }
    let j: Value = serde_json::from_str(&to_json(&r)).unwrap();
    assert_eq!(j["v"], "formula_id/1");
    assert_eq!(j["charge"]["source"], "finder");
    assert_eq!(j["chosen"], 0);
    assert_eq!(
        j["candidates"].as_array().unwrap().len(),
        r.candidates.len()
    );
    assert!(j["candidates"][0]["why"].as_array().unwrap().len() >= 4);
    assert!(j["observed"].as_array().unwrap().len() >= 2);
    // the alternatives are indexes of the list, never the chosen one
    assert!(j["alternatives"]
        .as_array()
        .unwrap()
        .iter()
        .all(|a| a.as_u64().unwrap() > 0));
}

#[test]
fn bad_input_is_an_error_not_a_panic() {
    let (mz, y, _) = case("caffeine");
    assert!(identify(&mz, &y, -1.0, &Opts::default()).is_err());
    assert!(identify(
        &mz,
        &y,
        mz[0],
        &Opts {
            ion: "[M+X]+".into(),
            ..Opts::default()
        }
    )
    .is_err());
    assert!(identify(
        &mz,
        &y,
        mz[0],
        &Opts {
            elements: Some("Q".into()),
            ..Opts::default()
        }
    )
    .is_err());
    assert!(identify(&[], &[], 195.0877, &Opts::default()).is_ok());
}

#[test]
fn the_options_come_from_json() {
    let o = Opts::from_json(r#"{"ion":"[M-H]-","z":null,"tol_ppm":3,"elements":"C:0-10,H:0-20","nitrogen":"odd","rdb_min":0,"max_results":5}"#).unwrap();
    assert_eq!(
        (o.ion.as_str(), o.z, o.tol_ppm, o.max_results),
        ("[M-H]-", None, 3.0, 5)
    );
    assert_eq!(o.elements.as_deref(), Some("C:0-10,H:0-20"));
    assert_eq!(Opts::from_json("").unwrap().ion, "[M+H]+");
    for bad in [
        r#"{"ion":3}"#,
        r#"{"tol_ppm":0}"#,
        r#"{"max_results":0}"#,
        r#"{"elements":4}"#,
        "{",
    ] {
        assert!(Opts::from_json(bad).is_err(), "{bad}");
    }
}
