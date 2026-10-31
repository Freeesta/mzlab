//! `imported_lists` of `screen.suspects@1` and `annot.regulatory@1`: a list the user imported (a `.mzsus` of its own) is screened whole next to the package, never merged
//! into it. On the invented package of `reg_tests` plus one invented imported list (`tests/data/normative_imported`: Isomerone A, which a package list names too, and Impallide).

use std::path::{Path, PathBuf};

use super::reg_tests::package;
use super::tests::table_bytes;
use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{self, Value};
use mzlab_feat::flow_regulatory::{regulatory_text, Params as RegParams};
use mzlab_feat::flow_screen_suspects::{screen_text, Params};

/// The `.mzsus` of the invented imported list.
pub(super) fn imported() -> Vec<u8> {
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/normative_imported");
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let out: PathBuf =
        std::env::temp_dir().join(format!("mzlab_imported_{}_{k}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    crate::normative::pack::pack(&input, &out).unwrap();
    let raw = std::fs::read(out.join("normative.mzsus")).unwrap();
    let _ = std::fs::remove_dir_all(&out);
    raw
}

const ANNOT: &str = r#"{"format":"annot/1","step":"annot.level@1","algorithm":"cascade","profile":"mzlab","rows":[{"id":"1","level":"2a","why":"library","name":"Isomerone A","entry":"ISOMERONEAXXAA-UHFFFAOYSA-N","gates":[]},{"id":"2","level":"3","why":"library","name":"Impallide","entry":"IMPALLIDEXXXAA-UHFFFAOYSA-N","gates":[]}]}"#;

fn screen(imp: &[(&str, &[u8])]) -> Value {
    let pk = package();
    let text = screen_text(
        &Params::from_params(&[]),
        &table_bytes(1),
        (&[("normative.mzsus", &pk)], imp),
        None,
        &[],
    )
    .unwrap();
    json::parse(text.as_bytes()).unwrap()
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::str).unwrap_or("")
}

fn hit(v: &Value, feature: f64) -> Option<&Value> {
    v.get("hits")?
        .arr()
        .iter()
        .find(|h| h.get("feature").and_then(Value::f64) == Some(feature))
}

fn lists(c: &Value) -> Vec<&str> {
    c.get("memberships")
        .unwrap()
        .arr()
        .iter()
        .map(|m| s(m, "list"))
        .collect()
}

#[test]
fn an_imported_list_adds_memberships_and_never_changes_the_package_ones_or_a_level() {
    let imp = imported();
    let (without, with) = (screen(&[]), screen(&[("imp.mzsus", &imp)]));
    let cand = |v: &Value| hit(v, 4.0).unwrap().get("candidates").unwrap().arr()[0].clone();
    assert_eq!(lists(&cand(&without)), ["T_LIST_A"]);
    assert_eq!(lists(&cand(&with)), ["T_LIST_A", "T_IMP"]);
    assert_eq!(s(&cand(&with), "name"), s(&cand(&without), "name"));
    // the imported list names a candidate the package does not screen by default: Level 5 as any mass candidate
    let new = hit(&with, 3.0).unwrap();
    assert_eq!(lists(&new.get("candidates").unwrap().arr()[0]), ["T_IMP"]);
    assert_eq!(new.get("level").and_then(Value::f64), Some(5.0));
    // every hit of the package-only run keeps its level
    for h in without.get("hits").unwrap().arr() {
        let f = h.get("feature").and_then(Value::f64).unwrap();
        let same = with
            .get("hits")
            .unwrap()
            .arr()
            .iter()
            .find(|x| {
                x.get("feature").and_then(Value::f64) == Some(f) && s(x, "adduct") == s(h, "adduct")
            })
            .unwrap();
        assert_eq!(same.get("level"), h.get("level"));
    }
}

#[test]
fn the_screen_header_records_the_sha256_of_the_package_and_of_the_imported_file() {
    let imp = imported();
    let v = screen(&[("imp.mzsus", &imp)]);
    let files = v.get("suspect_files").unwrap().arr();
    assert_eq!(files.len(), 2);
    assert!(files[0].get("imported").is_none());
    assert_eq!(s(&files[0], "sha256"), sha256_hex(&package()));
    assert_eq!(s(&files[1], "name"), "imp.mzsus");
    assert_eq!(s(&files[1], "sha256"), sha256_hex(&imp));
    assert_eq!(files[1].get("imported"), Some(&Value::Bool(true)));
    assert_eq!(files[1].get("lists").unwrap().arr().len(), 1);
}

#[test]
fn a_file_without_listings_is_not_an_imported_list() {
    let pk = package();
    let e = screen_text(
        &Params::from_params(&[]),
        &table_bytes(1),
        (&[("n.mzsus", &pk)], &[("l.csv", b"name,mass\nx,100\n")]),
        None,
        &[],
    )
    .unwrap_err();
    assert_eq!(e.key, "err.flow.regulatory.imported");
    let e = regulatory_text(
        &RegParams::from_params(&[]),
        ANNOT.as_bytes(),
        &pk,
        &[("l.csv", b"MZSU")],
    )
    .unwrap_err();
    assert_eq!(e.key, "err.flow.regulatory.imported");
}

fn regulatory(imp: &[(&str, &[u8])]) -> Value {
    let text = regulatory_text(
        &RegParams::from_params(&[]),
        ANNOT.as_bytes(),
        &package(),
        imp,
    )
    .unwrap();
    json::parse(text.as_bytes()).unwrap()
}

fn found(v: &Value, row: usize) -> Vec<String> {
    v.get("rows").unwrap().arr()[row]
        .get("regulatory")
        .unwrap()
        .arr()
        .iter()
        .map(|r| {
            format!(
                "{}|{}",
                s(r, "match"),
                s(r.get("membership").unwrap(), "list")
            )
        })
        .collect()
}

#[test]
fn the_structure_path_matches_the_imported_list_by_key_and_records_its_file() {
    let imp = imported();
    let (without, with) = (regulatory(&[]), regulatory(&[("imp.mzsus", &imp)]));
    assert_eq!(found(&without, 0), ["InChIKey|T_LIST_A"]);
    assert_eq!(found(&with, 0), ["InChIKey|T_LIST_A", "InChIKey|T_IMP"]);
    assert!(found(&without, 1).is_empty());
    assert_eq!(found(&with, 1), ["InChIKey|T_IMP"]);
    assert!(without.get("regulatory").unwrap().get("imported").is_none());
    let rec = with
        .get("regulatory")
        .unwrap()
        .get("imported")
        .unwrap()
        .at(0)
        .unwrap();
    assert_eq!(s(rec, "sha256"), sha256_hex(&imp));
    // the row levels are the ones the input gave
    for (a, b) in without
        .get("rows")
        .unwrap()
        .arr()
        .iter()
        .zip(with.get("rows").unwrap().arr())
    {
        assert_eq!(a.get("level"), b.get("level"));
    }
}
