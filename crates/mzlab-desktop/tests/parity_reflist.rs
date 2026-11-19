//! The CSV list reader (`api/reflist`, op `lr_reflist`) in the app and in the site: the same bodies go to the app, to the core function the CLI uses
//! and, when `MZLAB_WASM_PKG` names the built `.wasm` (see `parity_routes.rs`), to the module of the site; the answers must be the same bytes.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

/// Bodies: good rows (mz, formula, adduct, polarity), wrong rows with their lines, a missing header, an empty file, a broken request.
fn bodies() -> Vec<String> {
    let csvs = [
        "mz;name;formula;polarity\n100.1;A;;+\n;B;C6H12O6;-\n",
        "mz,name\nbad,A\n,B\n",
        "no columns here\n1;2\n",
        "",
    ];
    let mut v: Vec<String> = csvs
        .iter()
        .map(|c| json!({"csv": c, "name": "L"}).to_string())
        .collect();
    v.push("{}".into());
    v
}

fn on_app(st: &State, body: &str) -> Result<String, String> {
    match st.rpc("lr_reflist", &json!({"body": body}), &[]) {
        Ok(Answer::Text(t)) => Ok(t),
        Ok(Answer::Bytes(_)) => panic!("bytes"),
        Err(e) => Err(e.key.to_string()),
    }
}

#[test]
fn the_app_gives_the_answer_of_the_core_and_the_same_error_keys() {
    let st = State::new();
    for body in bodies() {
        let core = mzlab_core::reflist::route(&body).map_err(|e| e.key.to_string());
        assert_eq!(on_app(&st, &body), core, "{body}");
    }
}

#[test]
fn the_app_gives_the_same_bytes_as_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let plan: Vec<Value> = bodies()
        .iter()
        .map(|b| json!({"op": "lr_reflist", "body": b}))
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
        assert_eq!(on_app(&st, body), site, "{body}");
    }
}
