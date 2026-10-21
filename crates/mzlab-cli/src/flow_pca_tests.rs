//! `table.pca@1` in a plan: the areas of the invented `.mzfeat` of `flow_stats_tests` (a blank and two conditions of three files) give the answer of the PCA
//! engine asked by hand as the page asks (matrix, roles, groups), a parameter reaches the engine, and a series with too few samples fails with its key.

use std::path::Path;

use mzlab_core::campioni;
use mzlab_feat::stat::pca_json;

use crate::flow_stats_tests::{campioni_2, rows, table, NAMES};

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_pca_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table()).unwrap();
    std::fs::write(d.join("samples.json"), campioni::to_json(&campioni_2())).unwrap();
    d
}

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"table.mzfeat","type":"table"},{"name":"samples.json","type":"samples"}],"steps":[
{"id":"p","step":"table.pca@1","in":{"table":"file:table.mzfeat","samples":"file:samples.json"},"params":PARAMS}]}"#;

fn run(d: &Path, params: &str) -> (String, String) {
    let plan = PLAN.replace("PARAMS", params);
    let (_, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None).unwrap();
    let out = std::fs::read_to_string(d.join("work/p.json")).unwrap_or_default();
    (out, prov)
}

/// The engine asked by hand, the way the page asks.
fn engine(extra: &str) -> String {
    let matrix: Vec<String> = rows().iter().map(|r| format!("{r:?}")).collect();
    let roles: Vec<String> = (0..NAMES.len())
        .map(|i| format!("\"{}\"", if i == 0 { "blank" } else { "sample" }))
        .collect();
    let groups: Vec<String> = NAMES.iter().map(|n| format!("\"#{n}\"")).collect();
    let body = format!(
        "{{\"matrix\":[{}],\"roles\":[{}],\"groups\":[{}]{extra}}}",
        matrix.join(","),
        roles.join(","),
        groups.join(",")
    );
    pca_json(body.as_bytes()).unwrap()
}

#[test]
fn the_plan_gives_the_pca_of_the_engine() {
    let d = dir("engine");
    let (got, prov) = run(&d, "{}");
    assert!(!prov.contains("\"error\""), "{prov}");
    assert_eq!(got, engine(""));
    assert_eq!(got, run(&d, "{}").0);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_parameter_reaches_the_engine() {
    let d = dir("param");
    let (got, _) = run(&d, r#"{"transform":"none","scaling":"auto","ncomp":2}"#);
    assert_eq!(
        got,
        engine(r#","transform":"none","scaling":"auto","ncomp":2"#)
    );
    assert_ne!(got, engine(""));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn too_few_samples_fail_with_the_key_of_the_engine() {
    let d = dir("few");
    let mut few = campioni_2();
    few.iter_mut().for_each(|c| c.kind = "blank".into());
    std::fs::write(d.join("samples.json"), campioni::to_json(&few)).unwrap();
    let (out, prov) = run(&d, "{}");
    assert!(
        out.is_empty() && prov.contains("err.stat.pca.few"),
        "{prov}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
