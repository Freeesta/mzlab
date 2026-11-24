//! Steps of the parity plan: the statistics routes of the committed reference and the experimental design (`design/1`), which rides the `samples` route
//! (`design_*`); same bytes in the app and the .wasm.

use serde_json::{json, Value};
use std::path::Path;

/// The experimental design (`design/1`) rides the `samples` route: proposed from a table, checked against it, canonical, digest and a bad input.
pub fn design_steps() -> Vec<Value> {
    let files: Vec<Value> = [
        "blank",
        "ref_t005min_1",
        "ref_t005min_2",
        "trt_t005min_1",
        "trt_t005min_2",
    ]
    .iter()
    .map(|n| json!({"name": n}))
    .collect();
    let step =
        |cop: &str, body: Value| json!({"op": "lr_samples", "cop": cop, "body": body.to_string()});
    let table =
        mzlab_core::samples::route_all("propose", &json!({"files": files}).to_string()).unwrap();
    let design = mzlab_core::samples::route_all("design_propose", &table).unwrap();
    let table: Value = serde_json::from_str(&table).unwrap();
    let design: Value = serde_json::from_str(&design).unwrap();
    vec![
        step("design_propose", table.clone()),
        step(
            "design_validate",
            json!({"design": design, "samples": table}),
        ),
        step("design_canonical", design.clone()),
        step("design_digest", design),
        step("design_propose", json!({"files": 1})), // not a samples/1: the same error on both sides
    ]
}

/// The first case of each statistics route of the committed reference (`tests/golden/ref/statistica.json`, invented tables): profile, blank, reference, PCA.
pub fn stat_steps() -> Vec<Value> {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/statistica.json");
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for c in &cases {
        let path = c["path"].as_str().unwrap().to_string();
        if !seen.contains(&path) {
            out.push(json!({"op": "lr_stat", "path": path, "body": c["body"].to_string()}));
            seen.push(path);
        }
    }
    out
}

/// `raw info` on the index of slot 0 (`/api/rawinfo`, `docs/agenti/raw-access.md`): a short probe, a measured offset, a probe out of range.
pub fn raw_steps() -> Vec<Value> {
    ["?probe=8", "?probe=64&offset_mz=300", "?probe=0"]
        .map(|q| json!({"op": "route", "slot": 0, "path": "/api/rawinfo", "query": q}))
        .to_vec()
}

/// Where two text answers first differ: the position and the text around it (a report is thousands of characters and the CI log keeps a tail).
pub fn first_difference(a: &str, w: &str) -> String {
    let at = a
        .bytes()
        .zip(w.bytes())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(w.len()));
    let near = |t: &str| {
        t.get(at.saturating_sub(120)..(at + 120).min(t.len()))
            .unwrap_or("?")
            .to_string()
    };
    format!(
        "  byte {at} of {} / {}\n  app:  ...{}\n  wasm: ...{}",
        a.len(),
        w.len(),
        near(a),
        near(w)
    )
}

/// The table of the `import` steps: a stem match, a case match, an unmatched row, duplicate rows, an unknown role, a wash and a name that looks like a number.
const IMPORT_CSV: &str = "file,sample name,role,group\nMQ_blk,12,wash,G1\nmq_wash_1,B,blank,G1\nghost.mzML,C,std,G2\nTW_blk,D,QC,G2\nTW_blk,E,dark,G2\nMQ_std_BTZ_1ppm,F,maybe,\n";

/// The design steps and the sample table import (`sample-import/1`): matches by name, case and stem, an unmatched row, duplicates, an unknown role, a wash, a name
/// that looks like a number, other separators, a CSV without a file column, a samples base and a bad request.
pub fn sample_steps() -> Vec<Value> {
    let files = json!([{"name": "MQ_blk.mzML"}, {"name": "MQ_wash_1.mzML"}, {"name": "TW_blk.mzML"}, {"name": "MQ_std_BTZ_1ppm.mzML"}]);
    let step = |body: Value| json!({"op": "lr_samples", "cop": "import", "body": body.to_string()});
    let mut steps = design_steps();
    steps.extend([
        step(json!({"files": files, "text": IMPORT_CSV})),
        step(json!({"files": files, "text": IMPORT_CSV.replace(',', ";")})),
        step(json!({"files": files, "text": IMPORT_CSV.replace(',', "\t")})),
        step(json!({"files": [{"name": "a.mzML"}], "text": "role\nstd\n"})),
        step(json!({"text": "file\na\n"})),
        step(json!({"files": files})),
    ]);
    steps
}

/// The import steps are not vacuous: four of the six give a `sample-import/1` (the other two are the same refusal in the app and the .wasm).
pub fn check_import_answers(plan: &[Value], wasm: &[Value]) {
    let n = plan.iter().zip(wasm).filter(|(s, _)| s["cop"] == "import").filter(|(_, w)| {
        matches!(super::from_wasm(w), super::Got::Text(t) if t.contains("\"schema\":\"sample-import/1\""))
    });
    assert_eq!(n.count(), 4);
}
