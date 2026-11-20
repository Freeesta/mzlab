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

/// The edit patches of `tests/dati/draw_d3.json` (the patch as text or as a list in the request) give the bytes of the CLI and the molkit `.wasm` through
/// the same route, and a failed operation keeps its key.
#[test]
fn draw_route_applies_the_edit_patches_of_the_shared_fixture() {
    let st = State::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fx: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("tests/dati/draw_d3.json")).unwrap(),
    )
    .unwrap();
    let cases = fx["cases"].as_array().unwrap();
    assert!(cases.len() >= 20, "{}", cases.len());
    let mut errors = 0;
    for c in cases {
        // the patch once as the text the CLI takes, once as the list itself
        let as_list =
            serde_json::from_str::<serde_json::Value>(c["request"]["edit"].as_str().unwrap()).ok();
        for (format, want) in [("scene", &c["scene"]), ("molfile", &c["molfile"])] {
            for list in [false, true] {
                let mut request = c["request"].clone();
                request["format"] = json!(format);
                if list {
                    match &as_list {
                        Some(l) => request["edit"] = l.clone(),
                        None => continue,
                    }
                }
                let got = st.rpc(
                    "feat/grafo",
                    &json!({"kind": "draw", "request": request}),
                    &[],
                );
                match want.as_str() {
                    Some(w) => assert_eq!(text(got.unwrap()), w, "{} {format}", c["name"]),
                    None => {
                        errors += 1;
                        assert_eq!(
                            got.err().map(|e| e.key.to_string()),
                            c["error_key"].as_str().map(str::to_string),
                            "{} {format}",
                            c["name"]
                        );
                    }
                }
            }
        }
    }
    assert!(errors >= 10, "{errors}");
}

/// A patch given as a list in the request is held to the same 64 KiB as the text form, with the same key.
#[test]
fn draw_route_refuses_a_patch_list_over_the_size_limit() {
    let st = State::new();
    let op = json!({"op": "atom.set", "id": 0, "label": "x".repeat(70)});
    let big = json!({"kind": "draw", "request": {"spec": "CCO", "edit": vec![op; 1000]}});
    let e = st.rpc("feat/grafo", &big, &[]).err();
    assert_eq!(e.map(|e| e.key), Some("err.mol.draw.editJson"));
}
