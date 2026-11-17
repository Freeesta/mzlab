//! `api/tic_background` (`ticbg_route`) gives the same text in the app (`rpc.rs` op `ticbg`) and in the lazy `.wasm` `ticfondo`
//! (`tests/rotte_wasm_node.js`, `MZLAB_WASM_PKG`, run by `tests/test_parita_rotte_app.py`), on the synthetic run with six background lines
//! of `mzlab_core::ticbg_synth`: automatic, options, the file as its own blank, a bad value.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

const QUERIES: [(&str, i64); 5] = [
    ("", -1),
    ("&kind=bpc&action=exclude&ex=305:0.5&keep=149", -1),
    ("&mode=quiet&rt0=1&rt1=5&res=hr&floor=0", -1),
    ("&mode=blank&blank=0", 0),
    ("&mode=nope", -1),
];

#[test]
fn the_tic_without_background_is_the_same_in_the_app_and_in_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("fondo.mzML");
    std::fs::write(&file, mzlab_core::ticbg_synth::mzml(600, 1, false)).unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let (fid, _) = st.pick(file.clone()).unwrap();
    for op in ["lr_put", "lr_index"] {
        st.rpc(op, &json!({"name": "fondo.mzML", "fid": fid}), &[])
            .unwrap();
    }
    let mut plan = vec![json!({"op": "idxopen", "slot": 0, "name": "fondo.mzML"})];
    for (q, blank) in QUERIES {
        plan.push(
            json!({"op": "ticbg", "slot": 0, "blank": blank, "query": format!("?k=0&level=1{q}")}),
        );
    }
    let plan_file = dir.path().join("plan.json");
    std::fs::write(&plan_file, serde_json::to_string(&plan).unwrap()).unwrap();
    let idx = dir.path().join("idx").join("fondo.mzML.mzidx");
    let out = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/rotte_wasm_node.js"))
        .args([Path::new(&pkg), &idx, &file, &plan_file])
        .output()
        .expect("node runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wasm: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    for (k, (step, w)) in plan.iter().zip(&wasm).enumerate() {
        let mut p = step.clone();
        p["fid"] = json!(fid);
        match st.rpc(step["op"].as_str().unwrap(), &p, &[]) {
            Ok(Answer::Text(t)) => {
                assert_eq!(w["s"].as_str(), Some(t.as_str()), "step {k}: {step}")
            }
            Ok(Answer::Bytes(_)) => panic!("bytes"),
            // the same key in both: the wasm's error is the JSON `{error_key, params}`
            Err(e) => assert!(
                w["e"]
                    .as_str()
                    .unwrap_or("")
                    .contains(&format!("\"{}\"", e.key)),
                "step {k}: {w}"
            ),
        }
    }
    // the automatic answer finds the six background lines of the synthetic run
    let auto: Value = serde_json::from_str(wasm[1]["s"].as_str().unwrap()).unwrap();
    let mut ions: Vec<i64> = auto["ions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["mz"].as_f64().unwrap().round() as i64)
        .collect();
    ions.sort();
    assert_eq!(ions, [141, 149, 158, 198, 214, 236]);
}
