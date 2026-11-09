//! The copyable command of a Flow plan (`api/flow/command`, op `lr_flow_command`) in the app and in the site: the same `mzflow/2` plans go to the app's rpc, to the
//! core function the CLI uses and, when `MZLAB_WASM_PKG` names the built `.wasm` (see `parita_rotte.rs`), to the module of the site; the answers must be the same bytes.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

/// Plans: a valid one with steps, a valid one with a name that needs care, a wrong format, a broken text.
fn plans() -> Vec<String> {
    vec![
        json!({"format": "mzflow/2", "inputs": [{"name": "a b.mzML"}, {"name": "x\ny.mzML"}], "steps": []}).to_string(),
        r#"{"format":"mzflow/2","inputs":[{"name":"MZFLOW"}],"steps":[]}"#.into(),
        r#"{"format":"mzflow/9","inputs":[],"steps":[]}"#.into(),
        "not json".into(),
    ]
}

fn on_app(st: &State, body: &str) -> Result<String, String> {
    match st.rpc("lr_flow_command", &json!({"body": body}), &[]) {
        Ok(Answer::Text(t)) => Ok(t),
        Ok(Answer::Bytes(_)) => panic!("bytes"),
        Err(e) => Err(e.key.to_string()),
    }
}

#[test]
fn the_app_gives_the_script_of_the_core_and_the_same_error_keys() {
    let st = State::new();
    let mut ok = 0;
    for body in plans() {
        let core = mzlab_core::flow::command_route(&body).map_err(|e| e.key.to_string());
        assert_eq!(on_app(&st, &body), core, "{body}");
        ok += core.is_ok() as usize;
    }
    assert_eq!(ok, 2, "two plans are valid and two are errors");
}

#[test]
fn the_app_gives_the_same_bytes_as_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let plan: Vec<Value> = plans()
        .iter()
        .map(|b| json!({"op": "flow_command", "body": b}))
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
    for (body, w) in plans().iter().zip(&wasm) {
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
        assert_eq!(app, site, "{body}");
    }
    assert!(wasm[0]["s"]
        .as_str()
        .unwrap()
        .contains("mzlab-cli flow run"));
}
