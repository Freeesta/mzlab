//! `tp.review@1` in a plan over the `.mzfeat` of the position tests: the decisions are matched to the rows by m/z and RT, checked with the rules of the
//! levels and written as `review/1`; the same bytes on every run, and a file that is not `tpdec/1` is refused.

use std::path::Path;

use mzlab_feat::recipe::Recipe;

use crate::flow_position_tests::tp_file;

fn run(d: &Path, decisions: &str) -> Result<String, String> {
    std::fs::write(d.join("decisions.json"), decisions).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"tp.mzfeat","type":"tp"},{"name":"decisions.json","type":"transform"}],"steps":[
        {"id":"rev","step":"tp.review@1","in":{"tp":"file:tp.mzfeat","decisions":"file:decisions.json"},"params":{}}]}"#;
    let (done, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)
        .map_err(|e| e.key.to_string())?;
    if !done.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read_to_string(d.join("work/rev.json")).unwrap())
}

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_review_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("tp.mzfeat"), tp_file(&Recipe::default())).unwrap();
    d
}

#[test]
fn decisions_are_matched_checked_and_counted() {
    let d = dir("rules");
    let decisions = r#"{"schema":"tpdec/1","rows":{
        "100.0000@1.00":{"l":"3a","n":"  looks like a hydroxylation  "},
        "101.0000@1.00":{"l":"1","n":""},
        "999.0000@9.00":{"l":"nottp","n":"gone"},
        "100.0001@1.00":{"l":"9","n":""}}}"#;
    let got = run(&d, decisions).unwrap();
    assert!(got.starts_with("{\"schema\":\"review/1\""), "{got}");
    // a decision of the standard needs a marked standard, an unknown level and a row that is not there are listed and not counted
    assert!(
        got.contains("\"key\":\"101.0000@1.00\",\"id\":1")
            && got.contains("1 needs a standard marked"),
        "{got}"
    );
    assert!(
        got.contains("\"key\":\"999.0000@9.00\",\"id\":null")
            && got.contains("\"status\":\"unmatched\""),
        "{got}"
    );
    assert!(
        got.contains("\"note\":\"looks like a hydroxylation\""),
        "{got}"
    );
    assert!(
        got.contains("\"summary\":{\"decided\":1,\"not_tp\":0,\"refused\":1,\"unmatched\":2}"),
        "{got}"
    );
    assert_eq!(got, run(&d, decisions).unwrap());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn not_a_tp_is_counted_and_an_unknown_level_refused() {
    let d = dir("nottp");
    let got = run(
        &d,
        r#"{"schema":"tpdec/1","rows":{"100.0000@1.00":{"l":"nottp","n":""},"101.0000@1.00":{"l":"9","n":""}}}"#,
    )
    .unwrap();
    assert!(got.contains("unknown level"), "{got}");
    assert!(
        got.contains("\"summary\":{\"decided\":0,\"not_tp\":1,\"refused\":1,\"unmatched\":0}"),
        "{got}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_level_two_b_needs_a_proposed_level() {
    let d = dir("twob");
    let got = run(
        &d,
        r#"{"schema":"tpdec/1","rows":{"100.0000@1.00":{"l":"2b","n":""}}}"#,
    )
    .unwrap();
    let proposed = got
        .split("\"proposed\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap();
    let allowed = ["2a", "3a", "3b", "4"].contains(&proposed);
    assert_eq!(got.contains("\"status\":\"ok\""), allowed, "{got}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_file_that_is_not_tpdec_is_refused() {
    let d = dir("schema");
    assert!(run(&d, r#"{"schema":"other/1","rows":{}}"#).is_err());
    let _ = std::fs::remove_dir_all(&d);
}
