//! The route `feat/grafo` with `kind: "draw"` gives the `draw/1` scene (or the molfile) of `mzlab_feat::chem::draw::draw_json`, the function of
//! `mzlab-cli mol draw`, with the request as an object or as text, and keeps the key of an error; the 30 specs of `tests/dati/draw_d1.json` give the
//! same bytes as the molkit `.wasm` and the CLI.

use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::json;

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

#[test]
fn draw_route_equals_the_function() {
    let st = State::new();
    let req = json!({"spec": "smiles:C/C=C/C(=O)O", "format": "scene"});
    let direct = mzlab_feat::chem::draw::draw_json(&req.to_string()).unwrap();
    assert!(direct.starts_with(r#"{"schema":"draw/1""#), "{direct}");
    let as_object = json!({"kind": "draw", "request": req});
    let as_text = json!({"kind": "draw", "request": req.to_string()});
    assert_eq!(text(st.rpc("feat/grafo", &as_object, &[]).unwrap()), direct);
    assert_eq!(text(st.rpc("feat/grafo", &as_text, &[]).unwrap()), direct);
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "draw", "request": {"spec": "smiles:C1CC"}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("err.mol.draw.smiles"));
}

#[test]
fn draw_route_gives_the_shared_fixture() {
    let st = State::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fx: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("tests/dati/draw_d1.json")).unwrap(),
    )
    .unwrap();
    let cases = fx["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 30);
    for c in cases {
        for (format, want) in [
            ("scene", &c["scene"]),
            ("molfile", &c["molfile"]),
            ("ket", &c["ket"]),
            ("cdxml", &c["cdxml"]),
        ] {
            let mut request = c["request"].clone();
            request["format"] = json!(format);
            let got = st.rpc(
                "feat/grafo",
                &json!({"kind": "draw", "request": request}),
                &[],
            );
            match want.as_str() {
                Some(w) => assert_eq!(text(got.unwrap()), w, "{} {format}", c["name"]),
                None => assert_eq!(
                    got.err().map(|e| e.key.to_string()),
                    c["error_key"].as_str().map(str::to_string),
                    "{} {format}",
                    c["name"]
                ),
            }
        }
        // a scene carries the same KET or CDXML text as the plain format
        for format in ["ket", "cdxml"] {
            if let Some(want) = c[format].as_str() {
                let mut request = c["request"].clone();
                request["format"] = json!("scene");
                request["file"] = json!(format);
                let got = st.rpc(
                    "feat/grafo",
                    &json!({"kind": "draw", "request": request}),
                    &[],
                );
                let v: serde_json::Value = serde_json::from_str(&text(got.unwrap())).unwrap();
                assert_eq!(v["file"].as_str(), Some(want), "{} {format}", c["name"]);
            }
        }
    }
}
