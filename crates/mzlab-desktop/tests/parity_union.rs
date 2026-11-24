//! The series union (`union/1`, op `lr_union`) in the app, in the CLI and in the site: the same requests go to the app's rpc, to the core function
//! `mzlab-cli quant union` uses and, when `MZLAB_WASM_PKG` names the built `.wasm` (see `parity_routes.rs`), to the module of the site; the answers must be the same bytes.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

/// A feature table: `shared` ions at m/z 100, 110, ... and RT 1.0, 1.5, ..., then `own` ions from `own_mz`, all shifted by `shift` minutes.
fn table(shared: usize, own: usize, own_mz: f64, shift: f64) -> String {
    let mut t = String::from("id,mz,rt,rt_range:min,rt_range:max,charge\n");
    let rows = (0..shared)
        .map(|i| (100.0 + 10.0 * i as f64, 1.0 + 0.5 * i as f64))
        .chain((0..own).map(|i| (own_mz + 10.0 * i as f64, 2.0 + 0.5 * i as f64)));
    for (k, (mz, rt)) in rows.enumerate() {
        let rt = rt + shift;
        t += &format!("{},{mz},{rt:.4},{:.4},{:.4},1\n", k + 1, rt - 0.1, rt + 0.1);
    }
    t
}

fn request(tables: &[(&str, String)], params: Value) -> String {
    let t: Vec<Value> = tables.iter().map(|(n, x)| json!({"name": n, "text": x})).collect();
    json!({"tables": t, "params": params}).to_string()
}

/// Requests that cover the merge, the map, the drops, the polarity and the errors.
fn bodies() -> Vec<String> {
    let shifted = [("s1", table(10, 3, 300.0, 0.0)), ("s2", table(10, 3, 400.0, 0.2))];
    vec![
        request(&shifted, json!({})),
        request(&shifted, json!({"rt_warp": "piecewise"})),
        request(&shifted, json!({"rt_warp": "piecewise", "min_series": 2, "pad": 0.05, "polarity": "-"})),
        request(&[("s1", table(2, 0, 0.0, 0.0)), ("s2", table(2, 0, 0.0, 0.2))], json!({"rt_warp": "piecewise", "rt_tol": "0.3"})),
        request(&[("s1", "id,mz,rt\na,200.0,3.00\nb,200.2,3.05\n".into())], json!({})),
        request(&[("s1", "id;mz;rt;polarity\na;200;3;-\nb;200;3;\n".into())], json!({"polarity": "+"})),
        request(&[("s1", "id,mz,rt\na,x,3\n".into())], json!({})),
        request(&[("s1", table(1, 0, 0.0, 0.0))], json!({"da": 9})),
        "{\"tables\":[]}".to_string(),
        "not json".to_string(),
    ]
}

fn on_app(st: &State, body: &str) -> Result<String, String> {
    match st.rpc("lr_union", &json!({"body": body}), &[]) {
        Ok(Answer::Text(t)) => Ok(t),
        Ok(Answer::Bytes(_)) => panic!("bytes"),
        Err(e) => Err(e.key.to_string()),
    }
}

#[test]
fn the_app_gives_the_union_of_the_core_and_the_same_error_keys() {
    let st = State::new();
    let mut ok = 0;
    for body in bodies() {
        let core = mzlab_core::series_union::route(&body).map_err(|e| e.key.to_string());
        assert_eq!(on_app(&st, &body), core, "{}", &body[..body.len().min(60)]);
        ok += core.is_ok() as usize;
    }
    assert_eq!(ok, 6, "six requests answer and four are errors");
}

#[test]
fn the_app_gives_the_same_bytes_as_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let plan: Vec<Value> = bodies().iter().map(|b| json!({"op": "union_tables", "body": b})).collect();
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
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
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
    assert!(wasm[0]["s"].as_str().unwrap().contains("\"schema\":\"union/1\""));
}
