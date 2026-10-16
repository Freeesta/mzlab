//! «No adduct» of the XIC window: the text typed is already the ion (`adduct=none` of `/api/formula`).
use mzlab_core::elements::formula_mz;
use serde_json::Value;

fn get(f: &str, a: &str) -> Value {
    serde_json::from_str(&formula_mz(f, Some(a)).unwrap()).unwrap()
}

#[test]
fn no_adduct_takes_the_formula_as_the_ion() {
    let neutral = get("C14H12F4N3O4S", "[M+H]+");
    let none = get("C14H12F4N3O4S", "none");
    assert_eq!(none["adduct"], "");
    assert_eq!(none["mz"], none["neutral"]);
    assert!(neutral["mz"].as_f64().unwrap() - none["mz"].as_f64().unwrap() > 1.0);
}

#[test]
fn an_adduct_written_in_the_text_is_the_one_used() {
    let want = get("C14H13F4N3O4S", "[M+H]+");
    for t in [
        "C14H13F4N3O4S+H",
        "C14H13F4N3O4S [M+H]+",
        "C14H13F4N3O4S[M+H]+",
    ] {
        let got = get(t, "none");
        assert_eq!(got["mz5"], want["mz5"], "{t}");
        assert_eq!(got["adduct"], "[M+H]+", "{t}");
        assert_eq!(got["formula"], want["formula"], "{t}");
    }
    let neg = get("C9H8O4-H", "none");
    assert_eq!(neg["mz5"], get("C9H8O4", "[M-H]-")["mz5"]);
    assert_eq!(
        get("C9H8O4+Na", "none")["mz5"],
        get("C9H8O4", "[M+Na]+")["mz5"]
    );
}

#[test]
fn the_other_adducts_are_unchanged_and_errors_stay() {
    assert_eq!(get("C9H8O4", "[M+H]+")["adduct"], "[M+H]+");
    assert!(formula_mz("C9H8O4+H", Some("[M+H]+")).is_err());
    assert!(formula_mz("zz", Some("none")).is_err());
}
