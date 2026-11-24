//! `estimate/1` (op `lr_estimate`) in the app, in the CLI's core function and in the site: bodies of synthetic `estimates/1` objects (three files, two axes,
//! one file with an error, with and without a plan, a body that is not one) go to the app's rpc, to `mzlab_core::estimates::route` and, when `MZLAB_WASM_PKG`
//! names the built `.wasm` (see `parity_routes.rs`), to the module of the site; the answers must be the same bytes.
//! The `.wasm` leg is skipped when the variable is unset (a plain `cargo test` never runs it): `tests/test_parita_rotte_app.py` builds the module and sets it.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

fn file(name: &str, axis: &str, polarity: i32, k: f64) -> Value {
    json!({"schema":"estimates/1","file":name,"polarity":polarity,"axis":axis,"n":120,
        "noise":{"floor":1000.0*k,"mode":900.0*k,"sigma":0.4},
        "min_height":{"value":9000.0*k,"factor":10.0,"mode":900.0*k},
        "ppm":{"value":if axis=="ppm" {Value::from(4.0*k)} else {Value::Null},"sigma":0.1,"n":40},
        "fwhm":{"value":0.12*k,"n":40},"rt_tol":0.24*k,"dead":0.5,
        "da":{"value":if axis=="Da" {Value::from(0.3*k)} else {Value::Null},"sigma":0.05,"n":40},
        "defect":0.24})
}

fn plan() -> Value {
    json!({"format":"mzflow/2","steps":[
        {"id":"det","step":"features.detect@1","params":{"min_height":5000}},
        {"id":"ali","step":"features.align@1"},
        {"id":"gap","step":"features.gapfill@1"}]})
}

fn bodies() -> Vec<String> {
    let three: Vec<Value> = [1.0, 1.2, 0.9]
        .iter()
        .enumerate()
        .map(|(i, k)| file(&format!("s{i}.mzML"), "Da", 1, *k))
        .collect();
    let mut mixed = three.clone();
    mixed.push(file("neg.mzML", "ppm", -1, 1.0));
    let mut broken = three.clone();
    broken.push(json!({"file":"bad.mzML","error":{"schema":"error/1","error_key":"err.flow.io","params":{}}}));
    let ppm: Vec<Value> = (0..4)
        .map(|i| file(&format!("p{i}.mzML"), "ppm", 1, 1.0 + i as f64 / 10.0))
        .collect();
    let mut all: Vec<String> = vec![
        json!({"files": three, "plan": plan()}),
        json!({"files": three, "plan": null}),
        json!({"files": mixed, "plan": plan()}),
        json!({"files": broken, "plan": plan()}),
        json!({"files": ppm, "plan": plan()}),
        json!({"files": [three[0].clone()], "plan": plan()}),
        json!({"files": [{"file":"bad.mzML","error":{}}]}),
    ]
    .into_iter()
    .map(|v| v.to_string())
    .collect();
    all.extend([
        "{}".to_string(),
        "{\"files\":[]}".to_string(),
        "not json".to_string(),
    ]);
    all
}

fn on_app(st: &State, body: &str) -> Result<String, String> {
    match st.rpc("lr_estimate", &json!({"body": body}), &[]) {
        Ok(Answer::Text(t)) => Ok(t),
        Ok(Answer::Bytes(_)) => panic!("bytes"),
        Err(e) => Err(e.key.to_string()),
    }
}

#[test]
fn the_app_gives_the_answer_of_the_core_and_the_same_error_keys() {
    let st = State::new();
    let mut ok = 0;
    for body in bodies() {
        let core = mzlab_core::estimates::route(&body).map_err(|e| e.key.to_string());
        assert_eq!(on_app(&st, &body), core, "{}", &body[..body.len().min(60)]);
        ok += core.is_ok() as usize;
    }
    assert_eq!(ok, bodies().len() - 3, "seven answers and three errors");
}

#[test]
fn the_app_gives_the_same_bytes_as_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        eprintln!("the .wasm leg of parity_estimate did not run: MZLAB_WASM_PKG is not set (tests/test_parita_rotte_app.py runs it)");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let plan: Vec<Value> = bodies()
        .iter()
        .map(|b| json!({"op": "estimate_set", "body": b}))
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
        .contains("\"schema\":\"estimate/1\""));
}
