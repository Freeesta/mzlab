//! The defaults of every Flow step (`step-params/1`, op `lr_flow_defaults`) in the app, in the CLI and in the site: the `step/1` descriptor of every step of the
//! registry goes to the app's rpc, to the core function `mzlab-cli flow defaults` uses and, when `MZLAB_WASM_PKG` names the built `.wasm` (see `parity_routes.rs`),
//! to the module of the site; the answers must be the same bytes.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

/// Every step of the registry as the page receives it, then two bodies that are not a step.
fn bodies() -> Vec<String> {
    let mut all: Vec<String> = mzlab_cli::flow::steps()
        .iter()
        .map(mzlab_core::step::json::step_json)
        .collect();
    all.extend(["{}".to_string(), "not json".to_string()]);
    all
}

fn on_app(st: &State, body: &str) -> Result<String, String> {
    match st.rpc("lr_flow_defaults", &json!({"body": body}), &[]) {
        Ok(Answer::Text(t)) => Ok(t),
        Ok(Answer::Bytes(_)) => panic!("bytes"),
        Err(e) => Err(e.key.to_string()),
    }
}

#[test]
fn the_app_gives_the_defaults_of_the_core_and_the_same_error_keys() {
    let st = State::new();
    let mut ok = 0;
    for body in bodies() {
        let core = mzlab_core::step::defaults::route(&body).map_err(|e| e.key.to_string());
        assert_eq!(on_app(&st, &body), core, "{}", &body[..body.len().min(60)]);
        ok += core.is_ok() as usize;
    }
    assert_eq!(
        ok,
        bodies().len() - 2,
        "every step answers and the two other bodies are errors"
    );
}

#[test]
fn the_app_gives_the_same_bytes_as_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let plan: Vec<Value> = bodies()
        .iter()
        .map(|b| json!({"op": "flow_defaults", "body": b}))
        .collect();
    let file = dir.path().join("plan.json");
    std::fs::write(&file, serde_json::to_string(&plan).unwrap()).unwrap();
    let empty = dir.path().join("empty");
    std::fs::write(&empty, b"").unwrap();
    let out = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/rotte_wasm_node.js"))
        .arg(&pkg)
        .arg(&empty)
        .arg(&empty)
        .arg(&file)
        .output()
        .expect("node runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wasm: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    let st = State::new();
    for (body, w) in bodies().iter().zip(&wasm) {
        let app = on_app(&st, body);
        let site = match w["s"].as_str() {
            Some(s) => Ok(s.to_string()),
            None => {
                let e = w["e"].as_str().unwrap_or("");
                Err(serde_json::from_str::<Value>(e)
                    .ok()
                    .and_then(|j| j["error_key"].as_str().map(String::from))
                    .unwrap_or_else(|| e.to_string()))
            }
        };
        assert_eq!(app, site, "{}", &body[..body.len().min(60)]);
    }
    assert!(wasm[0]["s"]
        .as_str()
        .unwrap()
        .contains("\"schema\":\"step-params/1\""));
}
