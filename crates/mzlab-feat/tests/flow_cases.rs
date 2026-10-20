//! The cases of `tests/data/flow_check_cases.json` against the real checker: the same file is read by `tests/test_flow_grafico.py`, which runs the JavaScript
//! checker of the Flow page (`flow/lazy/flow-check.js`) on it, so the two cannot drift apart. `MZLAB_UPDATE_FLOW_CASES=1` writes the answers of this checker
//! into the file (review the diff: the answer is the contract).

use mzlab_core::flow::{check, parse, Platform};
use mzlab_feat::steps::{attributio, registry, tabui};

const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/data/flow_check_cases.json"
);

/// The answer of the checker for one case: `instance|port|key|k=v,k=v` lines, or the key of the error of the reader.
fn answer(plan: &str) -> Result<Vec<String>, String> {
    let plan = parse(plan.as_bytes()).map_err(|e| e.key.to_string())?;
    let mut steps = registry();
    steps.extend(attributio());
    steps.extend(tabui());
    Ok(check(&plan, &steps, Platform::Native)
        .iter()
        .map(|p| {
            let params: Vec<String> = p.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
            format!(
                "{}|{}|{}|{}",
                p.instance,
                p.port,
                p.error_key,
                params.join(",")
            )
        })
        .collect())
}

#[test]
fn the_shared_cases_give_the_written_answers() {
    let text = std::fs::read_to_string(PATH).unwrap();
    let mut cases: serde_json::Value = serde_json::from_str(&text).unwrap();
    let update = std::env::var("MZLAB_UPDATE_FLOW_CASES").is_ok();
    for case in cases.as_array_mut().unwrap() {
        let got = answer(&case["plan"].to_string());
        let name = case["name"].as_str().unwrap().to_string();
        if update {
            match &got {
                Ok(lines) => case["expect"] = serde_json::json!(lines),
                Err(key) => case["parse_error"] = serde_json::json!(key),
            }
            continue;
        }
        match (&got, case["parse_error"].as_str()) {
            (Err(key), Some(want)) => assert_eq!(key, want, "{name}"),
            (Ok(lines), None) => assert_eq!(&serde_json::json!(lines), &case["expect"], "{name}"),
            (got, want) => panic!("{name}: got {got:?}, wanted a parse error {want:?}"),
        }
    }
    if update {
        std::fs::write(PATH, serde_json::to_string_pretty(&cases).unwrap() + "\n").unwrap();
    }
}
