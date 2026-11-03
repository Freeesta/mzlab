//! `flow/run` of the app on a plan with `imported_lists` (a private `.mzsus` handed over as `b64:` text, as the Flow page does) gives the bytes of the native `flow run`.
//! With `MZLAB_PARITA_NORMATIVE=<dir>` (the inputs and outputs a native run left there, see `tests/test_normative_imported_app_parity.py`); without it the test does nothing.
use std::path::Path;

use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"table.mzfeat","type":"table"},{"name":"annot.json","type":"annot"},{"name":"normative.mzsus","type":"regulatory"},{"name":"imp.mzsus","type":"regulatory"}],"steps":[
    {"id":"s","step":"screen.suspects@1","in":{"table":"file:table.mzfeat","suspects":["file:normative.mzsus"],"imported_lists":["file:imp.mzsus"]},"params":{"include":"proposal"}},
    {"id":"r","step":"annot.regulatory@1","in":{"annot":"file:annot.json","lists":"file:normative.mzsus","imported_lists":["file:imp.mzsus"]},"params":{"lists":"T_LIST_A,T_LIST_C"}}]}"#;

/// Standard base64 with padding.
fn b64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    bytes
        .chunks(3)
        .flat_map(|c| {
            let n = ((c[0] as u32) << 16)
                | ((*c.get(1).unwrap_or(&0) as u32) << 8)
                | (*c.get(2).unwrap_or(&0) as u32);
            (0..4).map(move |k| {
                if k <= c.len() {
                    A[((n >> (18 - 6 * k)) & 63) as usize] as char
                } else {
                    '='
                }
            })
        })
        .collect()
}

#[test]
fn the_app_gives_the_bytes_of_the_native_run_with_imported_lists() {
    let Some(dir) = std::env::var_os("MZLAB_PARITA_NORMATIVE") else {
        return;
    };
    let dir = Path::new(&dir);
    let files: Value = ["table.mzfeat", "annot.json", "normative.mzsus", "imp.mzsus"]
        .iter()
        .map(|n| {
            (
                n.to_string(),
                json!(format!("b64:{}", b64(&std::fs::read(dir.join(n)).unwrap()))),
            )
        })
        .collect::<serde_json::Map<_, _>>()
        .into();
    let data = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    st.set_flow_folder(data.path().join("flow"));
    let a = st
        .rpc("flow/run", &json!({ "plan": PLAN, "files": files }), &[])
        .unwrap();
    let out: Value = match a {
        Answer::Text(t) => serde_json::from_str(&t).unwrap(),
        Answer::Bytes(_) => panic!("bytes"),
    };
    let prov = out["prov"].as_str().unwrap();
    assert!(!prov.contains("\"error\""), "{prov}");
    let made = Path::new(out["dir"].as_str().unwrap());
    for f in ["s.json", "r.json"] {
        assert_eq!(
            std::fs::read(made.join(f)).unwrap(),
            std::fs::read(dir.join(f)).unwrap(),
            "{f}"
        );
    }
}
