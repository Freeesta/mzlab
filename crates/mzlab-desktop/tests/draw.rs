//! The route `feat/grafo` with `kind: "draw"` gives the `draw/1` scene (or the molfile) of `mzlab_feat::chem::draw::draw_json`, the function of
//! `mzlab-cli mol draw`, with the request as an object or as text, and keeps the key of an error; the 30 specs of `tests/dati/draw_d1.json` give the
//! same bytes as the molkit `.wasm` and the CLI, for the molfile, the scene, the SVG and the PNG (length and CRC-32), and the picture requests with
//! options of its `images`.

use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::json;

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

fn bytes(a: Answer) -> Vec<u8> {
    match a {
        Answer::Bytes(b) => b,
        Answer::Text(_) => panic!("text"),
    }
}

/// The length and CRC-32 of a PNG, as the fixture has them.
fn png_of(b: &[u8]) -> (u64, u64) {
    (b.len() as u64, u64::from(mzlab_core::zip::crc32(b)))
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
        let ask = |format: &str| {
            let mut request = c["request"].clone();
            request["format"] = json!(format);
            st.rpc(
                "feat/grafo",
                &json!({"kind": "draw", "request": request}),
                &[],
            )
        };
        match c["svg"].as_str() {
            Some(svg) => assert_eq!(text(ask("svg").unwrap()), svg, "{} svg", c["name"]),
            None => assert!(ask("svg").is_err(), "{}", c["name"]),
        }
        match c["png"].as_object() {
            Some(p) => assert_eq!(
                png_of(&bytes(ask("png").unwrap())),
                (p["len"].as_u64().unwrap(), p["crc32"].as_u64().unwrap()),
                "{} png",
                c["name"]
            ),
            None => assert!(ask("png").is_err(), "{}", c["name"]),
        }
        for (format, want) in [("scene", &c["scene"]), ("molfile", &c["molfile"])] {
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
    }
}

#[test]
fn draw_route_gives_the_pictures_with_options_of_the_fixture() {
    let st = State::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fx: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("tests/dati/draw_d1.json")).unwrap(),
    )
    .unwrap();
    let images = fx["images"].as_array().unwrap();
    assert_eq!(images.len(), 7);
    for c in images {
        let got = st
            .rpc(
                "feat/grafo",
                &json!({"kind": "draw", "request": c["request"]}),
                &[],
            )
            .unwrap();
        match (c["text"].as_str(), c["png"].as_object()) {
            (Some(want), None) => assert_eq!(text(got), want, "{}", c["name"]),
            (None, Some(p)) => assert_eq!(
                png_of(&bytes(got)),
                (p["len"].as_u64().unwrap(), p["crc32"].as_u64().unwrap()),
                "{}",
                c["name"]
            ),
            _ => panic!("{}", c["name"]),
        }
    }
}
