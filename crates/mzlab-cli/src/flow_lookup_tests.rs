//! `spectra.lookup` on the synthetic three-compound library of `flow_ident_tests`: by name, whole name, formula and precursor m/z, and the found entry as the query of `spectra.match`.

use crate::flow_ident_tests::fixture;

fn run(tag: &str, params: &str, tail: &str) -> (std::path::PathBuf, String) {
    let dir = fixture(tag);
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"lib.msp","type":"library"}}],"steps":[
        {{"id":"l","step":"spectra.lookup@1","in":{{"library":"file:lib.msp"}},"params":{params}}}{tail}]}}"#
    );
    let (_, prov) =
        crate::flow::run_plan(plan.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    (dir, prov)
}

fn found(dir: &std::path::Path) -> String {
    std::fs::read_to_string(dir.join("work/l.mgf")).unwrap()
}

#[test]
fn the_entries_are_found_by_name_formula_and_precursor() {
    let (dir, prov) = run("lookup_name", r#"{"by":"name","value":"ALP"}"#, "");
    assert!(!prov.contains("\"error\""), "{prov}");
    let mgf = found(&dir);
    assert_eq!(mgf.matches("BEGIN IONS").count(), 1, "{mgf}");
    assert!(
        mgf.contains("NAME=Alpha") && mgf.contains("100.05000 400"),
        "{mgf}"
    );
    // whole name: a part of it finds nothing
    let (d, _) = run(
        "lookup_exact",
        r#"{"by":"name","value":"alp","exact":true}"#,
        "",
    );
    assert_eq!(found(&d).matches("BEGIN IONS").count(), 0);
    // the three entries share the formula, and top_n cuts them in the order of the library
    let (d, _) = run(
        "lookup_formula",
        r#"{"by":"formula","value":"C10H10O","top_n":2}"#,
        "",
    );
    let mgf = found(&d);
    assert!(
        mgf.contains("NAME=Alpha") && mgf.contains("NAME=Beta") && !mgf.contains("NAME=Gamma"),
        "{mgf}"
    );
    // Alpha and Beta are 2 ppm apart: a 1 ppm window keeps one, a 10 ppm window both
    let (d, _) = run(
        "lookup_mz1",
        r#"{"by":"precursor_mz","value":"300.1234","ppm":0.5}"#,
        "",
    );
    assert_eq!(found(&d).matches("BEGIN IONS").count(), 1);
    let (d, _) = run(
        "lookup_mz10",
        r#"{"by":"precursor_mz","value":"300.1234","ppm":10}"#,
        "",
    );
    assert_eq!(found(&d).matches("BEGIN IONS").count(), 2);
    for d in [dir, d] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn the_found_entry_is_a_query_for_the_match() {
    let tail = r#",{"id":"m","step":"spectra.match@1","in":{"query":"l.spectra","library":"file:lib.msp"},"params":{"min_shared":3}}"#;
    let (dir, prov) = run("lookup_chain", r#"{"by":"name","value":"alpha"}"#, tail);
    assert!(!prov.contains("\"error\""), "{prov}");
    let hits = std::fs::read_to_string(dir.join("work/m.json")).unwrap();
    assert!(hits.contains("\"name\":\"Alpha\""), "{hits}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_value_is_an_error() {
    let dir = fixture("lookup_missing");
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"lib.msp","type":"library"}],"steps":[
        {"id":"l","step":"spectra.lookup@1","in":{"library":"file:lib.msp"},"params":{"by":"name"}}]}"#;
    let out = crate::flow::run_plan(plan.as_bytes(), &dir, &dir.join("work"), 1, None);
    let err = match out {
        Ok((_, prov)) => prov,
        Err(e) => format!("{e:?}"),
    };
    assert!(err.contains("err.flow.lookup.value"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}
