//! `draw_json`: spec kinds and their detection, the molecules database for names and keys, kept or new layout, the scene and its choices.

use super::*;
use mzlab_core::json::{self, Value};

fn req(spec: &str, extra: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"spec\":").str(spec);
    o.raw(extra).raw("}");
    o.0
}

fn catalog() -> String {
    let p = format!(
        "{}/../../mzlab/web/molecole/catalogo.csv",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

fn scene_of(spec: &str, extra: &str) -> Value {
    let text = draw_json(&req(spec, &format!(",\"format\":\"scene\"{extra}"))).unwrap();
    json::parse(text.as_bytes()).unwrap()
}

fn with_db(spec: &str) -> Result<String> {
    let mut o = Out::default();
    o.raw(",\"db\":").str(&catalog());
    draw_json(&req(spec, &o.0))
}

fn err_key(r: Result<String>) -> &'static str {
    r.expect_err("an error").key
}

#[test]
fn draw_detects_the_kind_of_a_spec() {
    assert_eq!(classify("CCO").0, "smiles");
    assert_eq!(classify("c1ccccc1Cl").0, "smiles");
    assert_eq!(classify("smiles:CCO").0, "smiles");
    assert_eq!(classify("caffeine").0, "name");
    assert_eq!(classify("acetylsalicylic acid").0, "name");
    assert_eq!(classify("name:CCO").0, "name");
    assert_eq!(classify("BSYNRYMUTXBXSQ-UHFFFAOYSA-N").0, "inchikey");
    assert_eq!(classify("InChI=1S/CH4/h1H4").0, "inchi");
    assert_eq!(classify("\n  mzLab\n\nM  END\n").0, "molfile");
    assert_eq!(
        classify("InChI=1S/CH4/h1H4 BSYNRYMUTXBXSQ-UHFFFAOYSA-N"),
        ("inchikey", "BSYNRYMUTXBXSQ-UHFFFAOYSA-N")
    );
}

#[test]
fn draw_a_smiles_gives_a_molfile_and_a_scene_with_its_choices() {
    let mf = draw_json(&req("smiles:C/C=C/C(=O)O", "")).unwrap();
    assert!(mf.contains("M  END") && mf.contains("V2000"));
    let s = scene_of("C/C=C/C(=O)O", "");
    assert_eq!(s.get("schema").and_then(Value::str), Some("draw/1"));
    assert_eq!(s.get("atoms").unwrap().arr().len(), 6);
    assert_eq!(s.get("bonds").unwrap().arr().len(), 5);
    assert_eq!(s.get("fragments").unwrap().arr().len(), 1);
    let c = s.get("choices").unwrap().arr();
    assert_eq!(c[0].get("value").and_then(Value::str), Some("smiles"));
    assert_eq!(c[1].get("value").and_then(Value::str), Some("new"));
}

#[test]
fn draw_the_scene_comes_first_with_its_schema() {
    let t = draw_json(&req("CCO", ",\"format\":\"scene\"")).unwrap();
    assert!(t.starts_with("{\"schema\":\"draw/1\""), "{t}");
}

#[test]
fn draw_a_name_and_a_key_need_the_database() {
    assert_eq!(
        err_key(draw_json(&req("caffeine", ""))),
        "err.mol.draw.needDb"
    );
    assert_eq!(
        err_key(draw_json(&req("BSYNRYMUTXBXSQ-UHFFFAOYSA-N", ""))),
        "err.mol.draw.needDb"
    );
    let by_name = with_db("name:caffeine").unwrap();
    assert!(by_name.starts_with("caffeine\n") && by_name.contains("M  END"));
    let by_key = with_db("BSYNRYMUTXBXSQ-UHFFFAOYSA-N").unwrap();
    assert!(by_key.starts_with("acetylsalicylic acid\n"));
}

#[test]
fn draw_a_name_that_is_not_there_names_the_nearest() {
    let e = with_db("caffeinee").err().unwrap();
    assert_eq!(e.key, "err.mol.draw.not_found");
    assert!(
        e.params
            .iter()
            .any(|(k, v)| *k == "nearest" && v.contains("caffeine")),
        "{:?}",
        e.params
    );
    // nothing near it: the message without the "nearest names" clause
    assert_eq!(
        err_key(with_db("ZZZZZZZZZZZZZZ-ZZZZZZZZZZ-Z")),
        "err.mol.draw.not_found_bare"
    );
    assert_eq!(
        err_key(with_db("qqqqqqqqqqqq")),
        "err.mol.draw.not_found_bare"
    );
}

#[test]
fn draw_an_inchi_alone_is_refused_not_parsed() {
    assert_eq!(err_key(with_db("InChI=1S/CH4/h1H4")), "err.mol.draw.inchi");
}

#[test]
fn draw_a_molfile_keeps_its_coordinates_unless_told_otherwise() {
    let mut m = Mol::from_smiles("CCO").unwrap();
    m.coords = Some(m.layout_2d());
    let mf = m.to_molfile("ethanol");
    let kept = draw_json(&req(&mf, "")).unwrap();
    assert_eq!(kept, m.to_molfile("ethanol"));
    let s = scene_of(&mf, "");
    let c = s.get("choices").unwrap().arr();
    assert!(c.iter().any(|n| n
        .get("value")
        .and_then(Value::str)
        .is_some_and(|v| v.starts_with("kept"))));
    let relaid = scene_of(&mf, ",\"layout\":\"new\"");
    assert!(relaid.get("choices").unwrap().arr().iter().any(|n| n
        .get("value")
        .and_then(Value::str)
        .is_some_and(|v| v.starts_with("new"))));
    let flat = Mol::from_smiles("CCO").unwrap().to_molfile("");
    let s = scene_of(&flat, "");
    assert!(s.get("choices").unwrap().arr().iter().any(|n| n
        .get("value")
        .and_then(Value::str)
        .is_some_and(|v| v.contains("origin"))));
}

#[test]
fn draw_bad_requests_and_settings_are_told_apart() {
    assert_eq!(err_key(draw_json("{")), "err.mol.draw.request");
    assert_eq!(
        err_key(draw_json("{\"spec\":\"\"}")),
        "err.mol.draw.request"
    );
    assert_eq!(
        err_key(draw_json(&req("CCO", ",\"format\":\"png\""))),
        "err.mol.draw.binary"
    );
    assert_eq!(
        err_key(draw_json(&req("smiles:C(C", ""))),
        "err.mol.draw.smiles"
    );
    assert_eq!(
        err_key(draw_json(&req("molfile:not a molfile", ""))),
        "err.mol.draw.molfile"
    );
}

#[test]
fn draw_the_name_option_sets_the_title_line() {
    let t = draw_json(&req("CCO", ",\"name\":\"ethanol\"")).unwrap();
    assert!(t.starts_with("ethanol\n"));
}

#[test]
fn draw_pieces_are_listed_in_the_scene() {
    let s = scene_of("CC(=O)[O-].[Na+]", "");
    assert_eq!(s.get("fragments").unwrap().arr().len(), 2);
}

#[test]
fn draw_refuses_a_too_big_molecule_before_any_layout_work() {
    // 2001 atoms: the limit is checked first, so the refusal costs the parse only
    let t = std::time::Instant::now();
    let smiles = "C".repeat(2001);
    assert_eq!(err_key(draw_json(&req(&smiles, ""))), "err.mol.draw.tooBig");
    let mol = Mol::from_smiles(&smiles).unwrap().to_molfile("big");
    assert_eq!(err_key(draw_json(&req(&mol, ""))), "err.mol.draw.tooBig");
    assert!(t.elapsed().as_secs_f64() < 5.0);
}

#[test]
fn draw_a_large_molecule_finishes_with_every_atom_placed() {
    // 1950 atoms of polyphenylene: the layout stops at its work budget instead of cleaning for minutes
    let t = std::time::Instant::now();
    let smiles = vec!["c1ccc(cc1)"; 325].join("-");
    let s = scene_of(&smiles, "");
    assert!(t.elapsed().as_secs_f64() < 60.0);
    let atoms = s.get("atoms").unwrap().arr();
    assert_eq!(atoms.len(), 1950);
}

#[test]
fn an_inchikey_with_two_rows_warns_under_its_own_key() {
    let key = "BSYNRYMUTXBXSQ-UHFFFAOYSA-N";
    let smiles = "CC(=O)Oc1ccccc1C(=O)O";
    let db = format!(
        "id;name;synonyms;smiles;inchikey\n1;aspirin;;{smiles};{key}\n2;acetylsalicylic acid;;{smiles};{key}\n"
    );
    let mut o = Out::default();
    o.raw(",\"format\":\"scene\",\"db\":").str(&db);
    let text = draw_json(&req(&format!("inchikey:{key}"), &o.0)).unwrap();
    assert!(text.contains("key_ambiguous"), "{text}");
    assert!(!text.contains("name_ambiguous"), "{text}");
}
