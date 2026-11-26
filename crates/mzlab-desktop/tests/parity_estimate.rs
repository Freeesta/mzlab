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

/// `file` of a unit-axis series that shows the signature of the nominal profile (`hint`) or has its numbers without it.
fn nominal_file(name: &str, hint: bool, k: f64) -> Value {
    let mut f = file(name, "Da", 1, k);
    f["nominal"] = json!({"rows":14,"between":if hint {6} else {0},"between_share":if hint {0.428571} else {0.0},
        "height_now":7000.0,"height_nominal":2500.0,"split":if hint {2} else {0},"split_share":if hint {0.142857} else {0.0},
        "reach_now":0.1,"reach_nominal":0.25,"cut_between":0.1,"cut_split":0.05,"hint":hint});
    f
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
    let seen: Vec<Value> = (0..3)
        .map(|i| nominal_file(&format!("n{i}.mzML"), i != 2, 1.0))
        .collect();
    let none: Vec<Value> = (0..3)
        .map(|i| nominal_file(&format!("t{i}.mzML"), false, 1.0))
        .collect();
    let mut all: Vec<String> = vec![
        json!({"files": three, "plan": plan()}),
        json!({"files": three, "plan": null}),
        json!({"files": mixed, "plan": plan()}),
        json!({"files": broken, "plan": plan()}),
        json!({"files": ppm, "plan": plan()}),
        json!({"files": [three[0].clone()], "plan": plan()}),
        json!({"files": [{"file":"bad.mzML","error":{}}]}),
        // the nominal hint: two of three files show it, with and without a plan, and none does
        json!({"files": seen, "plan": plan()}),
        json!({"files": seen, "plan": null}),
        json!({"files": none, "plan": plan()}),
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
    assert_eq!(ok, bodies().len() - 3, "ten answers and three errors");
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

#[test]
fn the_nominal_hint_is_named_where_files_show_it_and_never_among_the_proposals() {
    let seen: Vec<Value> = (0..3)
        .map(|i| nominal_file(&format!("n{i}.mzML"), i != 2, 1.0))
        .collect();
    let st = State::new();
    let body = json!({"files": seen, "plan": plan()}).to_string();
    let doc: Value = serde_json::from_str(&on_app(&st, &body).unwrap()).unwrap();
    let hints = doc["hints"].as_array().unwrap();
    assert_eq!(
        hints.len(),
        1,
        "one instance of features.detect in the plan"
    );
    assert_eq!(hints[0]["set"], "det.preset=nominal");
    assert_eq!(
        (hints[0]["files"].clone(), hints[0]["of"].clone()),
        (json!(2), json!(3))
    );
    assert!(doc["params"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["param"] != "preset"));
    assert!(doc["notes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["key"] == "note.estimate.nominal"));
    let none: Vec<Value> = (0..3)
        .map(|i| nominal_file(&format!("t{i}.mzML"), false, 1.0))
        .collect();
    let body = json!({"files": none, "plan": null}).to_string();
    let doc: Value = serde_json::from_str(&on_app(&st, &body).unwrap()).unwrap();
    assert!(doc["hints"].as_array().unwrap().is_empty());
}
