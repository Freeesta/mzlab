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
