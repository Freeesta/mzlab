//! Signed shift of an ion text (`288.2+16`, `C9H8O4-18`): a literal addition to the m/z after the adduct, `+16` is 16.000 and not the mass of an oxygen.
use mzlab_core::elements::{formula_mz, split_offset};
use serde_json::Value;

fn get(f: &str, a: &str) -> Value {
    serde_json::from_str(&formula_mz(f, if a.is_empty() { None } else { Some(a) }).unwrap())
        .unwrap()
}

#[test]
fn a_bare_mz_takes_the_shift() {
    let j = get("288.2+16", "");
    assert_eq!(j["mz"].as_f64().unwrap(), 304.2);
    assert_eq!(j["offset"].as_f64().unwrap(), 16.0);
    assert_eq!(get("288,2 - 18", "")["mz"].as_f64().unwrap(), 270.2);
}

#[test]
fn a_formula_takes_the_shift_after_the_adduct() {
    let base = get("C9H8O4", "[M+H]+")["mz5"].as_f64().unwrap();
    let j = get("C9H8O4+16", "[M+H]+");
    assert!((j["mz5"].as_f64().unwrap() - (base + 16.0)).abs() < 1e-9);
    assert_eq!(j["adduct"], "[M+H]+");
    let na = get("C9H8O4+Na", "none")["mz5"].as_f64().unwrap();
    assert!((get("C9H8O4+Na-18", "none")["mz5"].as_f64().unwrap() - (na - 18.0)).abs() < 1e-9);
    assert!(get("C9H8O4", "[M+H]+").get("offset").is_none());
}

#[test]
fn a_malformed_shift_is_refused_and_an_adduct_is_not_a_shift() {
    for t in ["C9H8O4+1.2.3", "288.2+16x", "C9H8O4+2H"] {
        assert_eq!(
            split_offset(t).unwrap_err().key,
            "err.formula.offset",
            "{t}"
        );
    }
    assert_eq!(split_offset("C9H8O4+Na").unwrap().1, None);
    assert_eq!(split_offset("C9H8O4 [M+H]+").unwrap().1, None);
    assert!(formula_mz("288.2+", None).is_err());
}

#[test]
fn an_unreadable_head_keeps_its_text_and_shift_in_the_error() {
    let e = formula_mz("imidacloprid+16", Some("[M+H]+")).unwrap_err();
    let get = |k: &str| {
        e.params
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(get("head"), Some("imidacloprid"));
    assert_eq!(get("offset"), Some("16"));
}
