//! `screen.suspects@1` and `annot.regulatory@1` on an invented regulatory package (`tests/data/normative_screen`, packed by `normative pack`): two isomers
//! with one formula on two lists, an acid and its salt collapsed into one skeleton on two lists, one compound on three lists, one proposal-only compound.
//! The feature table is the one of `tests`. Candidates and memberships only: no verdict.

use std::path::{Path, PathBuf};

use super::tests::table_bytes;
use mzlab_core::json::{self, Value};
use mzlab_feat::flow_regulatory::{regulatory_text, Params as RegParams};
use mzlab_feat::flow_screen_suspects::{screen_text, Params};

/// A fresh folder of its own: the tests run in parallel in one process.
fn temp(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "mzlab_screen_reg_{name}_{}_{k}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The package bytes (`normative.mzsus`) of the invented lists.
pub(super) fn package() -> Vec<u8> {
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/normative_screen");
    let out = temp("pack");
    crate::normative::pack::pack(&input, &out).unwrap();
    let raw = std::fs::read(out.join("normative.mzsus")).unwrap();
    let _ = std::fs::remove_dir_all(&out);
    raw
}

fn obj(text: &str) -> Vec<(String, Value)> {
    match json::parse(text.as_bytes()).unwrap() {
        Value::Obj(o) => o,
        _ => unreachable!(),
    }
}

fn screen(p: &str) -> Value {
    let pk = package();
    let text = screen_text(
        &Params::from_params(&obj(p)),
        &table_bytes(1),
        &[("normative.mzsus", &pk)],
        None,
        &[],
    )
    .unwrap();
    json::parse(text.as_bytes()).unwrap()
}

/// The hits of one feature.
fn of(v: &Value, feature: f64) -> Vec<&Value> {
    v.get("hits")
        .unwrap()
        .arr()
        .iter()
        .filter(|h| h.get("feature").and_then(Value::f64) == Some(feature))
        .collect()
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::str).unwrap_or("")
}

fn lists_of(c: &Value) -> Vec<String> {
    c.get("memberships")
        .unwrap()
        .arr()
        .iter()
        .map(|m| format!("{}|{}|{}", s(m, "list"), s(m, "entry_ref"), s(m, "form")))
        .collect()
}

#[test]
fn the_isomer_group_is_complete_with_names_and_lists() {
    let v = screen("{}");
    let h = of(&v, 4.0);
    assert_eq!(h.len(), 1, "{h:?}");
    let c = h[0].get("candidates").unwrap().arr();
    let names: Vec<&str> = c.iter().map(|c| s(c, "name")).collect();
    assert_eq!(names, ["Isomerone A", "Isomerone B"]);
    assert_eq!(s(&c[0], "inchikey"), "ISOMERONEAXXAA-UHFFFAOYSA-N");
    assert_eq!(lists_of(&c[0]), ["T_LIST_A|Annex I, no. 1|parent"]);
    assert_eq!(lists_of(&c[1]), ["T_LIST_B|Table 1, no. 2|as_listed"]);
    assert_eq!(s(h[0], "formula"), "C14H20N2O4");
    assert_eq!(h[0].get("level").and_then(Value::f64), Some(5.0));
}

#[test]
fn a_salt_collapses_into_its_parent_and_a_compound_keeps_all_its_lists() {
    let v = screen("{}");
    let acid = of(&v, 1.0);
    let c = &acid[0].get("candidates").unwrap().arr()[0];
    assert_eq!(acid[0].get("candidates").unwrap().arr().len(), 1);
    assert_eq!(
        lists_of(c),
        [
            "T_LIST_A|Annex I, no. 2|parent",
            "T_LIST_C|Table 2, no. 7|salt"
        ]
    );
    let tri = of(&v, 2.0);
    let c = &tri[0].get("candidates").unwrap().arr()[0];
    assert_eq!(s(tri[0], "adduct"), "[M+Na]+");
    assert_eq!(lists_of(c).len(), 3, "{:?}", lists_of(c));
    let attr: Vec<&str> = acid[0].get("candidates").unwrap().arr()[0]
        .get("memberships")
        .unwrap()
        .arr()
        .iter()
        .map(|m| s(m, "attr"))
        .collect();
    assert_eq!(attr, ["", "MCL 4 ng/L"]);
}

#[test]
fn the_proposal_is_screened_only_when_included() {
    assert!(of(&screen("{}"), 3.0).is_empty());
    let v = screen(r#"{"include":"proposal"}"#);
    let h = of(&v, 3.0);
    assert_eq!(h.len(), 1);
    let c = &h[0].get("candidates").unwrap().arr()[0];
    assert_eq!(lists_of(c), ["T_LIST_P|Proposal annex, no. 1|as_listed"]);
    let used = v
        .get("suspect_files")
        .unwrap()
        .at(0)
        .unwrap()
        .get("lists")
        .unwrap()
        .arr();
    assert_eq!(used.len(), 4);
    assert_eq!(
        screen("{}")
            .get("suspect_files")
            .unwrap()
            .at(0)
            .unwrap()
            .get("lists")
            .unwrap()
            .arr()
            .len(),
        3
    );
}

#[test]
fn the_mask_keeps_the_named_lists_only_and_refuses_an_unknown_one() {
    let v = screen(r#"{"lists":"T_LIST_B"}"#);
    assert!(of(&v, 1.0).is_empty());
    let c = of(&v, 2.0)[0].get("candidates").unwrap().arr()[0].clone();
    assert_eq!(lists_of(&c), ["T_LIST_B|Table 1, no. 5|as_listed"]);
    let names: Vec<String> = of(&v, 4.0)[0]
        .get("candidates")
        .unwrap()
        .arr()
        .iter()
        .map(|c| s(c, "name").to_string())
        .collect();
    assert_eq!(names, ["Isomerone B"]);
    let pk = package();
    for (p, key) in [
        (r#"{"lists":"T_NOPE"}"#, "err.flow.regulatory.list"),
        (r#"{"include":"rumour"}"#, "err.flow.regulatory.include"),
    ] {
        let e = screen_text(
            &Params::from_params(&obj(p)),
            &table_bytes(1),
            &[("n.mzsus", &pk)],
            None,
            &[],
        )
        .unwrap_err();
        assert_eq!(e.key, key);
    }
}

#[test]
fn delta_ppm_is_that_of_the_table() {
    let v = screen("{}");
    for (feature, ppm) in [(1.0, 1.0), (2.0, 2.0), (4.0, -3.0)] {
        let d = of(&v, feature)[0]
            .get("delta_ppm")
            .and_then(Value::f64)
            .unwrap();
        assert!((d - ppm).abs() < 0.01, "feature {feature}: {d}");
    }
}

/// `params_sha256` of the step in the `prov/1` of a plan run with `params`.
fn params_hash(d: &Path, params: &str) -> String {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}},{{"name":"normative.mzsus","type":"regulatory"}}],"steps":[
        {{"id":"s","step":"screen.suspects@1","in":{{"table":"file:table.mzfeat","suspects":["file:normative.mzsus"]}},"params":{params}}}]}}"#
    );
    let (_, prov) =
        crate::flow::run_plan_cached(plan.as_bytes(), d, &d.join("work"), 1, None, Some(false))
            .unwrap();
    assert!(!prov.contains("\"error\""), "{prov}");
    let v = json::parse(prov.as_bytes()).unwrap();
    s(
        v.get("activity").unwrap().get("s").unwrap(),
        "params_sha256",
    )
    .to_string()
}

#[test]
fn the_provenance_hash_changes_with_the_mask() {
    let d = temp("plan");
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table_bytes(1)).unwrap();
    std::fs::write(d.join("normative.mzsus"), package()).unwrap();
    let (a, b, c) = (
        params_hash(&d, "{}"),
        params_hash(&d, r#"{"lists":"T_LIST_A"}"#),
        params_hash(&d, r#"{"include":"proposal"}"#),
    );
    assert!(!a.is_empty() && a != b && a != c && b != c, "{a} {b} {c}");
    let _ = std::fs::remove_dir_all(&d);
}

/// An `annot/1` text: a full-key identity, a first-block-only identity, a proposal-only one, a row without a key and one that names a formula.
pub(super) const ANNOT: &str = r#"{"format":"annot/1","step":"annot.level@1","algorithm":"cascade","profile":"mzlab","rows":[{"id":"1","level":"2a","why":"library","name":"Quillic acid","entry":"QUILLICACIDXAA-UHFFFAOYSA-N","gates":[]},{"id":"2","level":"2a","why":"library","name":"Trilistol","entry":"TRILISTOLXXXAA-ZZZZZZZZSA-N","gates":[]},{"id":"3","level":"3","why":"library","name":"Proposalide","entry":"PROPOSALIDEXAA-UHFFFAOYSA-N","gates":[]},{"id":"4","level":"5","why":"mass","gates":[]},{"id":"5","level":"4","why":"formula","name":"C14H20N2O4","entry":"C14H20N2O4","gates":[]}]}"#;

fn regulatory(p: &str) -> Value {
    let text = regulatory_text(
        &RegParams::from_params(&obj(p)),
        ANNOT.as_bytes(),
        &package(),
    )
    .unwrap();
    json::parse(text.as_bytes()).unwrap()
}

fn matches(v: &Value, row: usize) -> Vec<String> {
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
fn the_structure_path_matches_by_key_and_flags_a_first_block_match() {
    let v = regulatory("{}");
    assert_eq!(matches(&v, 0), ["InChIKey|T_LIST_A", "InChIKey|T_LIST_C"]);
    assert_eq!(
        matches(&v, 1),
        [
            "first block|T_LIST_A",
            "first block|T_LIST_B",
            "first block|T_LIST_C"
        ]
    );
    assert!(matches(&v, 2).is_empty() && matches(&v, 3).is_empty() && matches(&v, 4).is_empty());
    assert_eq!(
        matches(&regulatory(r#"{"include":"proposal"}"#), 2),
        ["InChIKey|T_LIST_P"]
    );
    let row = &v.get("rows").unwrap().arr()[0];
    assert_eq!(
        (s(row, "level"), s(row, "entry")),
        ("2a", "QUILLICACIDXAA-UHFFFAOYSA-N")
    );
    assert_eq!(
        v.get("regulatory")
            .unwrap()
            .get("lists")
            .unwrap()
            .arr()
            .len(),
        3
    );
    assert!(matches(&regulatory(r#"{"window":[0,100]}"#), 0).is_empty());
}

#[test]
fn the_structure_path_runs_in_a_plan_and_refuses_a_package_without_lists() {
    let d = temp("annot");
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("annot.json"), ANNOT).unwrap();
    std::fs::write(d.join("normative.mzsus"), package()).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"annot.json","type":"annot"},{"name":"normative.mzsus","type":"regulatory"}],"steps":[
        {"id":"r","step":"annot.regulatory@1","in":{"annot":"file:annot.json","lists":"file:normative.mzsus"}}]}"#;
    let (_, prov) =
        crate::flow::run_plan_cached(plan.as_bytes(), &d, &d.join("work"), 1, None, Some(false))
            .unwrap();
    assert!(!prov.contains("\"error\""), "{prov}");
    let made = std::fs::read(d.join("work/r.json")).unwrap();
    let want = regulatory_text(&RegParams::from_params(&[]), ANNOT.as_bytes(), &package()).unwrap();
    assert_eq!(String::from_utf8(made).unwrap(), want);
    let e = regulatory_text(&RegParams::from_params(&[]), ANNOT.as_bytes(), b"MZSU").unwrap_err();
    assert_eq!(e.key, "err.flow.regulatory.package");
    let _ = std::fs::remove_dir_all(&d);
}

/// With `MZLAB_PARITA_NORMATIVE=<dir>`, writes the inputs, the `prov/1` and the outputs of a native plan run of both steps there, for the byte parity with
/// the `.wasm` (`tests/test_normative_screen_wasm.py`); without it, the run alone.
#[test]
fn both_paths_run_in_one_plan_for_the_wasm_parity() {
    let d = temp("parity");
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table_bytes(1)).unwrap();
    std::fs::write(d.join("normative.mzsus"), package()).unwrap();
    std::fs::write(d.join("annot.json"), ANNOT).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"table.mzfeat","type":"table"},{"name":"annot.json","type":"annot"},{"name":"normative.mzsus","type":"regulatory"}],"steps":[
        {"id":"s","step":"screen.suspects@1","in":{"table":"file:table.mzfeat","suspects":["file:normative.mzsus"]},"params":{"include":"proposal"}},
        {"id":"r","step":"annot.regulatory@1","in":{"annot":"file:annot.json","lists":"file:normative.mzsus"},"params":{"lists":"T_LIST_A,T_LIST_C"}}]}"#;
    let (_, prov) =
        crate::flow::run_plan_cached(plan.as_bytes(), &d, &d.join("work"), 1, None, Some(false))
            .unwrap();
    assert!(!prov.contains("\"error\""), "{prov}");
    if let Some(out) = std::env::var_os("MZLAB_PARITA_NORMATIVE") {
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        for f in [
            "table.mzfeat",
            "normative.mzsus",
            "annot.json",
            "work/s.json",
            "work/r.json",
        ] {
            std::fs::copy(d.join(f), out.join(f.trim_start_matches("work/"))).unwrap();
        }
        std::fs::write(out.join("prov.json"), &prov).unwrap();
    }
    let _ = std::fs::remove_dir_all(&d);
}
