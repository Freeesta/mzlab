//! `samples::import` (`sample-import/1`): a table of file -> sample name, role and group laid over the samples of a set of files; findings are reported
//! with their evidence, every automatic choice is logged and nothing is refused for a finding.

use serde_json::{json, Value};

use mzlab_core::samples;

pub(super) fn run(files: &[&str], text: &str) -> Value {
    let files: Vec<Value> = files.iter().map(|n| json!({"name": n})).collect();
    let out =
        samples::route_all("import", &json!({"files": files, "text": text}).to_string()).unwrap();
    serde_json::from_str(&out).unwrap()
}

pub(super) fn sample<'a>(doc: &'a Value, file: &str) -> &'a Value {
    doc["samples"]["samples"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["file"] == file)
        .unwrap()
}

pub(super) fn what<'a>(doc: &'a Value, w: &str) -> Vec<&'a Value> {
    doc["report"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["what"] == w)
        .collect()
}

#[test]
fn rows_set_name_role_and_group_and_every_change_is_logged() {
    let text = "file,sample name,role,group\nrun_a.mzML,S-001,dark,G1\nrun_b.mzML,S-002,wash,G1\nrun_c.mzML,S-003,QC,\nrun_d.mzML,S-004,std,G2\nrun_e.mzML,S-005,blank,G2\n";
    let d = run(
        &[
            "run_a.mzML",
            "run_b.mzML",
            "run_c.mzML",
            "run_d.mzML",
            "run_e.mzML",
        ],
        text,
    );
    assert_eq!(d["schema"], "sample-import/1");
    assert_eq!(d["report"]["status"], "complete");
    assert_eq!(
        (
            d["report"]["rows"].as_i64(),
            d["report"]["matched"].as_i64()
        ),
        (Some(5), Some(5))
    );
    let a = sample(&d, "run_a.mzML");
    assert_eq!(
        (
            a["kind"].as_str(),
            a["sample_id"].as_str(),
            a["group"].as_str()
        ),
        (Some("dark"), Some("S-001"), Some("G1"))
    );
    let b = sample(&d, "run_b.mzML");
    assert_eq!(
        (b["kind"].as_str(), b["blank_kind"].as_str()),
        (Some("blank"), Some("wash"))
    );
    assert_eq!(sample(&d, "run_c.mzML")["kind"], "qc");
    assert_eq!(sample(&d, "run_d.mzML")["kind"], "std");
    // the role the name proposed is the evidence of the change: `sample` -> `dark`
    let roles = what(&d, "role");
    let r = roles.iter().find(|c| c["file"] == "run_a.mzML").unwrap();
    assert_eq!(
        (r["from"].as_str(), r["to"].as_str(), r["row"].as_i64()),
        (Some("sample"), Some("dark"), Some(2))
    );
}

#[test]
fn unmatched_rows_files_and_ambiguous_keys_are_reported_not_refused() {
    let text = "file,role\nS1,blank\nghost.mzML,std\nS1.mzML,qc\nA.mzML,dark\n";
    let d = run(&["S1.mzML", "S1.raw", "a.mzML", "free.mzML"], text);
    let r = &d["report"];
    assert_eq!(r["status"], "partial");
    let rows = r["unmatched_rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    // `S1` could be S1.mzML or S1.raw: nothing is guessed, both are named
    assert_eq!(
        (rows[0]["row"].as_i64(), rows[0]["why"].as_str()),
        (Some(2), Some("ambiguous"))
    );
    assert_eq!(rows[0]["candidates"], json!(["S1.mzML", "S1.raw"]));
    assert_eq!(
        (rows[1]["file"].as_str(), rows[1]["why"].as_str()),
        (Some("ghost.mzML"), Some("no file"))
    );
    assert_eq!(r["unmatched_files"], json!(["S1.raw", "free.mzML"]));
    // an exact name matches silently; a different case is matched and logged
    assert_eq!(sample(&d, "S1.mzML")["kind"], "qc");
    assert_eq!(sample(&d, "a.mzML")["kind"], "dark");
    assert_eq!(what(&d, "match:case").len(), 1);
}

#[test]
fn a_name_without_its_extension_matches_and_is_logged() {
    let d = run(&["x_1.mzML"], "file,role\nx_1,blank\n");
    assert_eq!(sample(&d, "x_1.mzML")["kind"], "blank");
    let m = what(&d, "match:stem");
    assert_eq!(
        (m[0]["from"].as_str(), m[0]["to"].as_str()),
        (Some("x_1"), Some("x_1.mzML"))
    );
}

#[test]
fn duplicate_rows_keep_the_first_and_show_whether_they_disagree() {
    let text = "file,role\na.mzML,blank\nb.mzML,std\na.mzML,dark\nb.mzML,std\n";
    let d = run(&["a.mzML", "b.mzML"], text);
    let dup = d["report"]["duplicates"].as_array().unwrap();
    assert_eq!(dup.len(), 2);
    assert_eq!(
        (
            dup[0]["file"].as_str(),
            dup[0]["rows"].clone(),
            dup[0]["conflict"].as_bool(),
            dup[0]["kept"].as_i64()
        ),
        (Some("a.mzML"), json!([2, 4]), Some(true), Some(2))
    );
    assert_eq!(dup[1]["conflict"], false);
    assert_eq!(sample(&d, "a.mzML")["kind"], "blank");
    assert_eq!(what(&d, "duplicate_first").len(), 2);
    assert_eq!(d["report"]["status"], "partial");
}

#[test]
fn an_unknown_role_keeps_the_role_of_the_name_and_is_reported() {
    let d = run(&["BLK_1.mzML"], "file,role\nBLK_1.mzML,maybe\n");
    assert_eq!(
        d["report"]["unknown_roles"],
        json!([{"file": "BLK_1.mzML", "row": 2, "value": "maybe"}])
    );
    assert_eq!(sample(&d, "BLK_1.mzML")["kind"], "blank");
}

#[test]
fn a_sample_name_that_looks_like_a_number_stays_text_and_a_shared_name_is_evidence() {
    let d = run(&["p.mzML", "n.mzML"], "file,sample\np.mzML,12\nn.mzML,12\n");
    assert_eq!(sample(&d, "p.mzML")["sample_id"], "12");
    assert_eq!(
        d["report"]["shared_names"],
        json!([{"name": "12", "files": ["p.mzML", "n.mzML"]}])
    );
    assert_eq!(d["report"]["status"], "complete");
}

#[test]
fn other_columns_are_ignored_and_logged_and_a_table_without_files_is_an_error() {
    let d = run(&["a.mzML"], "file,role,vial\na.mzML,std,A1\n");
    assert_eq!(what(&d, "ignored_column")[0]["from"], "vial");
    assert_eq!(sample(&d, "a.mzML")["kind"], "std");
    let e = samples::route_all(
        "import",
        &json!({"files": [], "text": "role\nstd\n"}).to_string(),
    )
    .unwrap_err();
    assert_eq!(e.key, "error.badCsv");
    assert!(samples::route_all("import", "{}").is_err());
}

#[test]
fn a_samples_document_is_the_base_and_semicolons_and_tabs_are_read() {
    let base = run(&["a.mzML", "b.mzML"], "file\na.mzML\n");
    let doc = base["samples"].to_string();
    let input = json!({"samples": serde_json::from_str::<Value>(&doc).unwrap(), "text": "file\trole\nb.mzML\tqc\n"});
    let d: Value =
        serde_json::from_str(&samples::route_all("import", &input.to_string()).unwrap()).unwrap();
    assert_eq!(sample(&d, "b.mzML")["kind"], "qc");
    assert_eq!(d["report"]["unmatched_files"], json!(["a.mzML"]));
    let semi = run(&["a.mzML"], "file;role\na.mzML;dark\n");
    assert_eq!(sample(&semi, "a.mzML")["kind"], "dark");
}

#[test]
fn the_answer_is_the_same_bytes_every_time() {
    let text = "file,role,group\nb.mzML,std,G\na.mzML,blank,G\n";
    let files =
        json!({"files": [{"name": "a.mzML"}, {"name": "b.mzML"}], "text": text}).to_string();
    let first = samples::route_all("import", &files).unwrap();
    assert_eq!(first, samples::route_all("import", &files).unwrap());
    assert!(first.starts_with("{\"report\":{\"choices\":"));
}
