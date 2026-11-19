//! The CSV list reader (`api/reflist`, op `lr_reflist`) and the lookup of an m/z in a set of ions (`api/reflist_match`, op `lr_reflist_match`) in the app and in the site: the same bodies go to the app, to the core function the CLI uses
//! and, when `MZLAB_WASM_PKG` names the built `.wasm` (see `parity_routes.rs`), to the module of the site; the answers must be the same bytes.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

/// Bodies: good rows (mz, formula, adduct, polarity), wrong rows with their lines, a missing header, an empty file, a broken request.
fn bodies() -> Vec<(&'static str, String)> {
    let csvs = [
        "mz;name;formula;polarity\n100.1;A;;+\n;B;C6H12O6;-\n",
        "mz,name\nbad,A\n,B\n",
        "no columns here\n1;2\n",
        "",
    ];
    let mut v: Vec<(&str, String)> = csvs
        .iter()
        .map(|c| ("lr_reflist", json!({"csv": c, "name": "L"}).to_string()))
        .collect();
    v.push(("lr_reflist", "{}".into()));
    // the lookup: ppm and Da windows, a polarity, an ion in both polarities, a repeated group, a bad unit and a broken request
    let items = json!({"mz": [100.2, 99.9, 100.4, 100.7, 100.0], "pol": [1, -1, 1, 0, 0], "group": [0, 1, 2, 3, 3]});
    for req in [
        json!({"mz": 100.0, "unit": "da", "tol": 0.5, "items": items}),
        json!({"mz": 100.0, "pol": "-", "unit": "da", "items": items}),
        json!({"mz": 100.0, "unit": "ppm", "tol": 5000, "items": items}),
        json!({"mz": 100.0, "unit": "x", "items": items}),
        json!({"mz": 100.0, "items": {"mz": [1], "pol": [], "group": []}}),
    ] {
        v.push(("lr_reflist_match", req.to_string()));
    }
    v.push(("lr_reflist_match", "{}".into()));
    v
}

fn core(op: &str, body: &str) -> mzlab_core::error::Result<String> {
    if op == "lr_reflist" {
        mzlab_core::reflist::route(body)
    } else {
        mzlab_core::reflist::lookup::route(body)
    }
}

fn on_app(st: &State, op: &str, body: &str) -> Result<String, String> {
    match st.rpc(op, &json!({"body": body}), &[]) {
        Ok(Answer::Text(t)) => Ok(t),
        Ok(Answer::Bytes(_)) => panic!("bytes"),
        Err(e) => Err(e.key.to_string()),
    }
}

#[test]
fn the_app_gives_the_answer_of_the_core_and_the_same_error_keys() {
    let st = State::new();
    for (op, body) in bodies() {
        let core = core(op, &body).map_err(|e| e.key.to_string());
        assert_eq!(on_app(&st, op, &body), core, "{op} {body}");
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
        .map(|(op, b)| json!({"op": op, "body": b}))
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
    for ((op, body), w) in bodies().iter().zip(&wasm) {
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
        assert_eq!(on_app(&st, op, body), site, "{op} {body}");
    }
}
