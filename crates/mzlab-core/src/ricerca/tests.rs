use super::*;

const TABLES: &str = r#""losses":[{"id":"H2O","f":"H2O"},{"id":"NH3","f":"NH3"},{"id":"CO","f":"CO"}],
    "adducts":[{"id":"[M+H]+","k":1,"add":"H","z":1},{"id":"[M+NH4]+","k":1,"add":"NH4","z":1},{"id":"[M+Na]+","k":1,"add":"Na","z":1},
    {"id":"[M-H]-","k":1,"sub":"H","z":-1},{"id":"[2M+Na]+","k":2,"add":"Na","z":1},{"id":"[M]+","k":1,"add":"","z":1}],
    "contam":[{"id":"peg1","mz":63.04406,"pol":1},{"id":"x","mz":18.2,"pol":-1}]"#;

fn ids(x: &str, extra: &str) -> Vec<(String, String)> {
    let out = match_number(&format!(r#"{{"x":{x},{extra}{TABLES}}}"#)).unwrap();
    let v = parse(out.as_bytes()).unwrap();
    v.get("hits")
        .unwrap()
        .arr()
        .iter()
        .map(|h| {
            (
                h.get("kind").unwrap().str().unwrap().into(),
                h.get("id").unwrap().str().unwrap().into(),
            )
        })
        .collect()
}
fn has(h: &[(String, String)], k: &str, id: &str) -> bool {
    h.iter().any(|(a, b)| a == k && b == id)
}

#[test]
fn exact_masses_hr() {
    let hr = r#""hr":true,"tol_mda":3,"tol_ppm":5,"#;
    let h = ids("18.0106", hr);
    assert!(has(&h, "loss", "H2O") && !has(&h, "loss", "NH3"));
    assert!(!has(&h, "adduct", "[M+NH4]+"));
    assert!(has(&ids("22.9892", hr), "adduct", "[M+Na]+"));
    assert!(has(&ids("17.0265", hr), "loss", "NH3"));
    assert!(has(&ids("1.0073", hr), "adduct", "[M+H]+"));
}

#[test]
fn unit_resolution_is_half_a_dalton_without_decimals() {
    let h = ids("18", "");
    assert!(has(&h, "loss", "H2O") && has(&h, "adduct", "[M+NH4]+"));
    assert!(!has(&h, "loss", "NH3") && !has(&h, "loss", "CO"));
    let out = match_number(&format!(r#"{{"x":18,{TABLES}}}"#)).unwrap();
    assert!(out.contains("\"delta_da\":null") && out.contains("\"ppm\":null"));
}

#[test]
fn polarity_and_only_single_adducts() {
    let h = ids("22.99", r#""pol":"-","#);
    assert!(!has(&h, "adduct", "[M+Na]+"));
    assert!(
        has(&ids("22.99", ""), "adduct", "[M+Na]+")
            && !has(&ids("22.99", ""), "adduct", "[2M+Na]+")
    );
    assert!(!ids("0", "").iter().any(|(k, _)| k == "adduct"));
}

#[test]
fn contaminant_and_order() {
    let hr = r#""hr":true,"tol_ppm":5,"#;
    assert!(has(&ids("63.0441", hr), "contaminant", "peg1"));
    assert!(!has(&ids("63.0500", hr), "contaminant", "peg1"));
    let h = ids("63.2", "");
    assert!(has(&h, "contaminant", "peg1"));
}

#[test]
fn bad_request_is_an_error_with_a_key() {
    assert_eq!(
        match_number(r#"{"losses":[]}"#).unwrap_err().key,
        "err.search.request"
    );
}
