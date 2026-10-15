#![cfg(feature = "attributio")]
//! Formulas, adducts and structure fits of `attributio::formula` and `attributio::checks::norm_name` against a table generated once by Attributio 1.12.0
//! (`tools/attributio_riferimento.py`, stage `formule`), and the measured distance of the 36-element mass table from the 14-element one of `mzlab-core`.

mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::checks::norm_name;
use mzlab_feat::attributio::formula::{
    best_common_adduct, formula_charge, formula_mass, ion_mz, neutral_equivalent, parse_adduct,
    structure_fit,
};

fn close(a: f64, b: f64, at: &str) {
    assert!(
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0) * 1e-3 + 1e-12 || a == b,
        "{at}: {a} != {b}"
    );
}

#[test]
fn twenty_formulas_and_adducts_equal_the_reference_tool() {
    let g = golden("sintetica", "formule.json");
    let items = g["items"].as_array().expect("items");
    assert_eq!(items.len(), 20);
    for w in items {
        let (f, a) = (
            w["formula"].as_str().expect("f"),
            w["adduct"].as_str().expect("a"),
        );
        let at = format!("{f} {a}");
        let m = formula_mass(f);
        assert_eq!(m.is_some(), !w["mass"].is_null(), "{at}: mass");
        let Some(m) = m else { continue };
        close(m, w["mass"].as_f64().expect("mass"), &format!("{at} mass"));
        close(
            neutral_equivalent(m, f),
            w["neutral"].as_f64().expect("n"),
            &format!("{at} neutral"),
        );
        assert_eq!(
            formula_charge(f) as i64,
            w["charge"].as_i64().expect("q"),
            "{at}: charge"
        );
        match (parse_adduct(a), w["parsed"].as_array()) {
            (Some((n, d, z)), Some(p)) => {
                assert_eq!(n as i64, p[0].as_i64().expect("n"), "{at}: n");
                close(d, p[1].as_f64().expect("d"), &format!("{at} delta"));
                assert_eq!(z as i64, p[2].as_i64().expect("z"), "{at}: z");
            }
            (None, None) => continue,
            other => panic!("{at}: adduct {other:?}"),
        }
        close(
            ion_mz(m, a).expect("mz"),
            w["mz"].as_f64().expect("mz"),
            &format!("{at} mz"),
        );
        if let (Some(obs), Some(c)) = (w["observed"].as_f64(), w["closest"].as_array()) {
            let (ba, be) = best_common_adduct(
                obs,
                neutral_equivalent(m, f),
                w["pol"].as_str().expect("pol"),
            );
            assert_eq!(ba, c[0].as_str().expect("adduct"), "{at}: closest adduct");
            close(
                be,
                c[1].as_f64().expect("ppm"),
                &format!("{at} closest ppm"),
            );
        }
    }
}

#[test]
fn the_structure_fit_equals_the_reference_tool() {
    let g = golden("sintetica", "formule.json");
    for w in g["fits"].as_array().expect("fits") {
        let (e, ok) = structure_fit(
            w["formula"].as_str(),
            w["adduct"].as_str(),
            w["precursor"].as_f64(),
            w["pol"].as_str().expect("pol"),
            20.0,
        );
        let at = format!("{} {}", w["formula"], w["adduct"]);
        match w["ppm"].as_f64() {
            Some(p) => close(e, p, &at),
            None => assert!(e.is_nan(), "{at}"),
        }
        assert_eq!(ok, w["ok"].as_bool(), "{at}: ok");
    }
}

#[test]
fn norm_name_equals_the_reference_tool() {
    let g = golden("sintetica", "formule.json");
    for w in g["names"].as_array().expect("names") {
        assert_eq!(
            norm_name(w["in"].as_str().expect("in")),
            w["out"].as_str().expect("out"),
            "{}",
            w["in"]
        );
    }
}

#[test]
fn the_36_element_table_differs_from_the_core_table_by_less_than_50_ppb_where_both_have_the_element(
) {
    let mut shared = 0;
    for el in [
        "H", "C", "N", "O", "F", "Na", "Si", "P", "S", "Cl", "K", "Br", "I", "Se", "Mg", "Ca",
        "Al", "Fe", "Zn", "Cu",
    ] {
        let (Some(core), Some(ours)) = (
            mzlab_core::elements::mass_of(el),
            mzlab_feat::attributio::formula::mass_of(el),
        ) else {
            continue;
        };
        assert!(
            ((ours - core) / ours).abs() < 5e-8,
            "{el}: {ours} vs {core}"
        );
        shared += 1;
    }
    assert!(shared >= 10, "{shared} shared elements");
    assert!(formula_mass("FeCl2").is_some());
}
