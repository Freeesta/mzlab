//! `tray/1`: formula and mass of drawn structures, the suspect list written as CSV and read back to the same bytes, quoting, and the refusals.

use super::tray_text;
use mzlab_core::json::{self, Value};

const ETHANOL: &str = r#"{"name":"ethanol","smiles":"CCO","atoms":[{"el":"C","h":3},{"el":"C","h":2},{"el":"O","h":1}]}"#;
const D_ACID: &str = r#"{"name":"acetic acid, d1","smiles":"CC(=O)O[2H]","atoms":[{"el":"C","h":3},{"el":"C"},{"el":"O"},{"el":"O"},{"el":"H","iso":2}]}"#;
const CHOLINE: &str = r#"{"name":"choline","smiles":"C[N+](C)(C)CCO","atoms":[{"el":"C","h":3},{"el":"N","q":1},{"el":"C","h":3},{"el":"C","h":3},{"el":"C","h":2},{"el":"C","h":2},{"el":"O","h":1}]}"#;

fn entries(structures: &[&str]) -> Value {
    let r = format!(
        "{{\"op\":\"entries\",\"structures\":[{}]}}",
        structures.join(",")
    );
    json::parse(tray_text(&r).unwrap().as_bytes()).unwrap()
}

fn csv_of(entries: &str, adducts: &str) -> String {
    tray_text(&format!(
        "{{\"op\":\"csv\",\"entries\":{entries},\"adducts\":{adducts}}}"
    ))
    .unwrap()
}

fn read(csv: &str) -> Value {
    let r = format!("{{\"op\":\"read\",\"text\":{}}}", json_str(csv));
    json::parse(tray_text(&r).unwrap().as_bytes()).unwrap()
}

fn json_str(s: &str) -> String {
    let mut o = json::Out::default();
    o.str(s);
    o.0
}

fn entries_text(v: &Value) -> String {
    let mut o = json::Out::default();
    json::write_value(&mut o, v.get("entries").unwrap());
    o.0
}

#[test]
fn the_formula_and_mass_come_from_the_drawn_atoms() {
    let v = entries(&[ETHANOL, D_ACID, CHOLINE]);
    let e = v.get("entries").unwrap().arr();
    let f = |i: usize, k: &str| e[i].get(k).unwrap().clone();
    assert_eq!(f(0, "formula").str(), Some("C2H6O"));
    assert_eq!(f(0, "mass").f64(), Some(46.041865));
    assert_eq!(f(0, "name").str(), Some("ethanol"));
    assert_eq!(f(1, "smiles").str(), Some("CC(=O)O[2H]"));
    assert!(
        (f(1, "mass").f64().unwrap() - 61.027406).abs() < 2e-6,
        "{:?}",
        f(1, "mass")
    );
    assert_eq!(f(2, "charge").f64(), Some(1.0));
    assert_eq!(f(2, "formula").str(), Some("C5H14NO"));
}

#[test]
fn an_atom_that_is_no_element_is_an_error_row() {
    let v = entries(&[r#"{"name":"x","atoms":[{"el":"R1"}]}"#, ETHANOL]);
    let e = v.get("entries").unwrap().arr();
    assert!(e[0].get("error").is_some());
    assert_eq!(e[1].get("formula").unwrap().str(), Some("C2H6O"));
}

#[test]
fn the_list_has_the_columns_the_suspect_screen_reads() {
    let v = entries(&[ETHANOL, D_ACID]);
    let csv = csv_of(&entries_text(&v), r#"["[M+H]+","[M-H]-"]"#);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "name,smiles,formula,mass,charge,adducts");
    assert_eq!(lines[1], "ethanol,CCO,C2H6O,46.041865,0,[M+H]+;[M-H]-");
    assert!(
        lines[2].starts_with("\"acetic acid, d1\",CC(=O)O[2H],"),
        "{}",
        lines[2]
    );
    assert_eq!(lines.len(), 3);
}

#[test]
fn a_list_read_back_writes_the_same_bytes() {
    let v = entries(&[ETHANOL, D_ACID, CHOLINE]);
    let csv = csv_of(&entries_text(&v), r#"["[M+Na]+","[M+H]+","[M+HCOO]-"]"#);
    let back = read(&csv);
    assert_eq!(back.get("skipped").unwrap().f64(), Some(0.0));
    let ad: Vec<&str> = back
        .get("adducts")
        .unwrap()
        .arr()
        .iter()
        .filter_map(Value::str)
        .collect();
    assert_eq!(ad, ["[M+Na]+", "[M+H]+", "[M+HCOO]-"]);
    let again = csv_of(&entries_text(&back), r#"["[M+Na]+","[M+H]+","[M+HCOO]-"]"#);
    assert_eq!(again, csv);
}

#[test]
fn another_csv_is_read_by_column_name_and_rows_without_mass_are_counted() {
    let back = read(
        "SMILES,Name,Formula\nCCO,ethanol,C2H6O\nC,nothing,\n\"c1ccccc1\",\"benzene, ring\",C6H6\n",
    );
    let e = back.get("entries").unwrap().arr();
    assert_eq!(e.len(), 2);
    assert_eq!(e[1].get("name").unwrap().str(), Some("benzene, ring"));
    assert_eq!(e[1].get("mass").unwrap().f64(), Some(78.04695));
    assert_eq!(back.get("skipped").unwrap().f64(), Some(1.0));
    assert!(back.get("adducts").unwrap().arr().is_empty());
}

#[test]
fn an_unnamed_entry_gets_its_place_as_name() {
    let csv = csv_of(r#"[{"name":"","formula":"CH4O"}]"#, "[]");
    assert_eq!(csv.lines().nth(1), Some("structure 1,,CH4O,32.026215,0,"));
}

#[test]
fn the_adducts_are_the_catalogue_without_the_bare_ions() {
    let v = json::parse(tray_text(r#"{"op":"adducts"}"#).unwrap().as_bytes()).unwrap();
    let a = v.get("adducts").unwrap().arr();
    assert_eq!(a.len(), 18);
    let on: Vec<&str> = a
        .iter()
        .filter(|x| x.get("on").unwrap().bool() == Some(true))
        .filter_map(|x| x.get("name").unwrap().str())
        .collect();
    assert_eq!(on, ["[M+H]+", "[M-H]-"]);
    assert!(!a
        .iter()
        .any(|x| x.get("name").unwrap().str() == Some("[M]+")));
}

#[test]
fn bad_requests_are_refused_with_a_key() {
    let key = |r: &str| tray_text(r).unwrap_err().key;
    assert_eq!(key("{"), "error.tray.request");
    assert_eq!(key(r#"{"op":"nope"}"#), "error.tray.request");
    assert_eq!(
        key(r#"{"op":"csv","entries":[],"adducts":[]}"#),
        "error.tray.empty"
    );
    assert_eq!(
        key(r#"{"op":"csv","entries":[{"formula":"CH4"}],"adducts":["[M+Xx]+"]}"#),
        "error.tray.adduct"
    );
    assert_eq!(
        key(r#"{"op":"csv","entries":[{"formula":"Qq"}],"adducts":[]}"#),
        "error.tray.mass"
    );
    assert_eq!(
        key(r#"{"op":"read","text":"name,smiles\nx,C\n"}"#),
        "error.tray.csv"
    );
}
