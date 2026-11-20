//! Golden replay: every case of `tests/dati/mzdraw_log.json` (a start text, a command log) gives the recorded document, molfile, SVG and
//! log byte for byte, and every refused request its key. The `.wasm` is held to the same file by `tests/test_mzdraw_wasm.py`.
//! `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay` rewrites the recorded outputs after a wanted change.

use serde_json::{json, Value};
use std::path::PathBuf;

const OUTS: [&str; 4] = ["mzdraw", "molfile", "svg", "log"];

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/dati/mzdraw_log.json")
}

fn request(case: &Value, out: &str) -> String {
    let mut r = case["request"].clone();
    r["out"] = json!(out);
    r.to_string()
}

#[test]
fn every_case_replays_to_the_recorded_bytes() {
    let mut fx: Value = serde_json::from_str(&std::fs::read_to_string(fixture()).unwrap()).unwrap();
    let bless = std::env::var("MZDRAW_BLESS").is_ok();
    for case in fx["cases"].as_array_mut().unwrap() {
        for out in OUTS {
            let got = mzdraw::replay::replay(&request(case, out)).unwrap();
            if bless {
                case["out"][out] = json!(got);
            }
            assert_eq!(case["out"][out], json!(got), "{} as {out}", case["name"]);
        }
    }
    if bless {
        std::fs::write(fixture(), serde_json::to_string_pretty(&fx).unwrap() + "\n").unwrap();
    }
    assert!(fx["cases"].as_array().unwrap().len() >= 3);
}

#[test]
fn the_recorded_log_replays_to_the_same_document() {
    let fx: Value = serde_json::from_str(&std::fs::read_to_string(fixture()).unwrap()).unwrap();
    for case in fx["cases"].as_array().unwrap() {
        // the log of a session is itself a command log: replayed on the same start it gives the same document, and logs itself again
        let again = json!({ "log": serde_json::from_str::<Value>(case["out"]["log"].as_str().unwrap()).unwrap(), "out": "log" });
        assert_eq!(
            mzdraw::replay::replay(&again.to_string()).unwrap(),
            case["out"]["log"].as_str().unwrap(),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn a_refused_request_gives_its_key() {
    let fx: Value = serde_json::from_str(&std::fs::read_to_string(fixture()).unwrap()).unwrap();
    for case in fx["refused"].as_array().unwrap() {
        let e = mzdraw::replay::replay(&case["request"].to_string()).unwrap_err();
        assert_eq!(e.key, case["error_key"].as_str().unwrap());
    }
}
