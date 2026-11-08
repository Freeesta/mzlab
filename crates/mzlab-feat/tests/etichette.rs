//! `etich/1` against reference values of RDKit 2026.09 (monoisotopic masses, formulas with isotopes): [13CH4], CD3OH, cisplatin, a dianion, the
//! arrow benzene -> [13C]benzene (an isotopologue), and the errors of an R group and of an unknown isotope.
use mzlab_feat::chem::etichette::labels_json;

fn one(atoms: &str) -> String {
    labels_json(&format!(r#"{{"structures":[{atoms}]}}"#)).unwrap()
}
fn num(s: &str, key: &str) -> f64 {
    let i = s.find(&format!("\"{key}\":")).unwrap() + key.len() + 3;
    s[i..].split([',', '}']).next().unwrap().parse().unwrap()
}

#[test]
fn labelled_methane_counts_its_isotope() {
    let s = one(r#"[{"el":"C","iso":13,"h":4}]"#);
    assert!(s.contains(r#""formula":"[13C]H4""#), "{s}");
    assert!((num(&s, "mass") - 17.0347).abs() < 5e-5, "{s}");
    assert_eq!(num(&s, "nominal"), 17.0);
}

#[test]
fn deuterated_methanol() {
    let s = one(r#"[{"el":"D"},{"el":"C"},{"el":"D"},{"el":"D"},{"el":"O","h":1}]"#);
    assert!(s.contains(r#""formula":"CHD3O""#), "{s}");
    assert!((num(&s, "mass") - 35.0450).abs() < 5e-5, "{s}");
}

#[test]
fn cisplatin_has_a_formula_and_a_mass() {
    let s = one(r#"[{"el":"Cl"},{"el":"Pt"},{"el":"Cl"},{"el":"N","h":2},{"el":"N","h":2}]"#);
    assert!(s.contains(r#""formula":"H4Cl2N2Pt""#), "{s}");
    assert!((num(&s, "mass") - 296.9399).abs() < 5e-5, "{s}");
}

#[test]
fn a_dianion_brackets_its_formula() {
    let s = one(
        r#"[{"el":"O","q":-1},{"el":"C"},{"el":"O"},{"el":"C","h":2},{"el":"C"},{"el":"O"},{"el":"O","q":-1}]"#,
    );
    assert!(
        s.contains(r#""text":"[C3H2O4]2-""#) && s.contains(r#""charge":-2"#),
        "{s}"
    );
    let mz = (num(&s, "mass") + 2.0 * 0.000548579909) / 2.0;
    assert!((num(&s, "mz") - mz).abs() < 1e-6, "{s}");
}

#[test]
fn an_arrow_between_isotopologues() {
    let c = r#"{"el":"C","h":1}"#;
    let benz = format!("[{c},{c},{c},{c},{c},{c}]");
    let lab = format!(r#"[{{"el":"C","iso":13,"h":1}},{c},{c},{c},{c},{c}]"#);
    let s = labels_json(&format!(
        r#"{{"structures":[{benz},{lab}],"arrows":[[0,1]]}}"#
    ))
    .unwrap();
    assert!(
        s.contains(r#""gain":"[13C]","loss":"C","isotopologue":true"#),
        "{s}"
    );
    assert_eq!(num(&s, "dnominal"), 1.0);
    assert!((num(&s, "dm") - 1.003355).abs() < 1e-6, "{s}");
}

#[test]
fn errors_name_the_label() {
    let s = labels_json(r#"{"structures":[[{"el":"R#"}],[{"el":"C","iso":99}],[{"el":"O","h":2}]],"arrows":[[0,2]]}"#).unwrap();
    assert!(
        s.contains(r#"{"error":"error.etich.element","label":"R#"}"#),
        "{s}"
    );
    assert!(
        s.contains(r#"{"error":"error.etich.isotope","label":"C"}"#),
        "{s}"
    );
    assert!(
        s.contains(r#""formula":"H2O""#) && s.ends_with(r#""arrows":[null]}"#),
        "{s}"
    );
    assert_eq!(labels_json("nope").unwrap_err().key, "error.etich.request");
}
