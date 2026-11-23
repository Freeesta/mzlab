//! The route `feat/grafo` with `kind: "labels"` gives the same `label/1` as `mzlab_feat::chem::labels::labels_json` (the function of the `.wasm`
//! `grafo` and of `mzlab-cli mol label`), with the request as an object or as text (a bracket delta too), and keeps the key of an error; `kind: "cut"` gives `cut/1` the same way;
//! `kind: "analysis"` gives `analysis/1`, `kind: "sel"` `selection/1` and `kind: "link"` `link/1` (every case of `tests/dati/link_golden.json`) the same way; `kind: "fragments"` gives the `frag/1` bytes of the fixture `tests/dati/frag_radicale.json`, `kind: "check"` the `check/1` bytes of `tests/dati/check_disegno.json`, `kind: "cip"` the `cip/1` bytes of `tests/dati/cip_disegno.json`, `kind: "descriptors"` the `descriptors/1` bytes of every case of `tests/dati/descriptors_disegno.json` (V2000 and V3000), `kind: "sdf"` the `sdf/1` bytes of `tests/dati/sdf_disegno.json`, and `kind: "mech"` the `mechanism/1` bytes of `tests/dati/mechanism_mclafferty.json`.

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
fn labels_route_equals_the_function() {
    let st = State::new();
    let req = r#"{"structures":[[{"el":"Cl"},{"el":"Pt"},{"el":"Cl"},{"el":"N","h":2},{"el":"N","h":2}],[{"el":"C","iso":13,"h":4}]],"deltas":["","+O"],"arrows":[[0,1]]}"#;
    let direct = mzlab_feat::chem::labels::labels_json(req).unwrap();
    let as_object = json!({"kind": "labels", "request": serde_json::from_str::<serde_json::Value>(req).unwrap()});
    let as_text = json!({"kind": "labels", "request": req});
    assert_eq!(text(st.rpc("feat/grafo", &as_object, &[]).unwrap()), direct);
    assert_eq!(text(st.rpc("feat/grafo", &as_text, &[]).unwrap()), direct);
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "labels", "request": "nope"}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.label.request"));
}

#[test]
fn cut_route_equals_the_function() {
    let st = State::new();
    let mf = "\n  qqq\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 N   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\nM  END";
    let req = json!({"molfile": mf, "bonds": [[0, 1]], "mode": "heterolytic", "cation": [1]});
    let direct = mzlab_feat::chem::cut::cut_json(&req.to_string()).unwrap();
    assert_eq!(
        text(
            st.rpc("feat/grafo", &json!({"kind": "cut", "request": req}), &[])
                .unwrap()
        ),
        direct
    );
    assert_eq!(
        text(
            st.rpc(
                "feat/grafo",
                &json!({"kind": "cut", "request": req.to_string()}),
                &[]
            )
            .unwrap()
        ),
        direct
    );
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "cut", "request": {"molfile": mf, "bonds": []}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.grafo.cut.none"));
}

#[test]
fn fragments_route_gives_the_bytes_of_the_fixture() {
    // the same fixture is checked against the `.wasm` `grafo` in tests/test_feat_grafo_wasm.py: a radical side says `odd`
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/dati/frag_radicale.json")).unwrap();
    let st = State::new();
    let p = json!({"kind": "fragments", "molfile": fx["molfile"], "a": -1, "b": -1});
    assert_eq!(
        text(st.rpc("feat/grafo", &p, &[]).unwrap()),
        fx["frag"].as_str().unwrap()
    );
}

#[test]
fn sel_route_equals_the_function() {
    let st = State::new();
    let mf = "\n  qqq\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\nM  END";
    let req = json!({"molfile": mf, "part": true});
    let direct = mzlab_feat::chem::sel::sel_json(&req.to_string()).unwrap();
    assert!(direct.contains("\"selection/1\""));
    for request in [req.clone(), json!(req.to_string())] {
        let p = json!({"kind": "sel", "request": request});
        assert_eq!(text(st.rpc("feat/grafo", &p, &[]).unwrap()), direct);
    }
    let bad = st.rpc("feat/grafo", &json!({"kind": "sel", "request": "{}"}), &[]);
    assert_eq!(bad.err().map(|e| e.key), Some("error.sel.request"));
}

#[test]
fn mech_route_gives_the_bytes_of_the_fixture() {
    let st = State::new();
    let fx: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/dati/mechanism_mclafferty.json"
    ))
    .unwrap();
    let got = text(
        st.rpc(
            "feat/grafo",
            &json!({"kind": "mech", "request": fx["request"]}),
            &[],
        )
        .unwrap(),
    );
    assert_eq!(got, fx["mechanism"].as_str().unwrap());
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "mech", "request": "nope"}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.grafo.mech.request"));
}

#[test]
fn analysis_route_equals_the_function() {
    let st = State::new();
    // Cu: abundances of `isotopi-tutti`, outside the core table
    let req =
        r#"{"atoms":[{"el":"Cl"},{"el":"Cu"},{"el":"Cl"},{"el":"N","h":3},{"el":"N","h":3}]}"#;
    let direct = mzlab_feat::chem::analysis::analysis_json(req).unwrap();
    let as_object = json!({"kind": "analysis", "request": serde_json::from_str::<serde_json::Value>(req).unwrap()});
    assert_eq!(text(st.rpc("feat/grafo", &as_object, &[]).unwrap()), direct);
    assert_eq!(
        text(
            st.rpc(
                "feat/grafo",
                &json!({"kind": "analysis", "request": req}),
                &[]
            )
            .unwrap()
        ),
        direct
    );
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "analysis", "request": {"atoms": []}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.anal.empty"));
}

#[test]
fn check_route_gives_the_bytes_of_the_fixture() {
    // the same fixture is checked against the `.wasm` `grafo` and `mzlab-cli mol check` in tests/test_feat_grafo_wasm.py
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/dati/check_disegno.json")).unwrap();
    let st = State::new();
    for req in [fx["request"].clone(), fx["request"].to_string().into()] {
        let p = json!({"kind": "check", "request": req});
        assert_eq!(
            text(st.rpc("feat/grafo", &p, &[]).unwrap()),
            fx["check"].as_str().unwrap()
        );
    }
    // the hydrogen of an aromatic ring: unstated it is reported with its atom, stated (`MRV_IMPLICIT_H`) it is counted by `selection/1` too
    for (kind, request, answer) in [
        ("check", "aromatic_request", "aromatic_check"),
        ("check", "stated_request", "stated_check"),
        ("sel", "stated_request", "stated_sel"),
    ] {
        let mut req = fx[request].clone();
        if kind == "check" {
            req = json!({"molfile": req["molfile"]});
        }
        let p = json!({"kind": kind, "request": req});
        assert_eq!(
            text(st.rpc("feat/grafo", &p, &[]).unwrap()),
            fx[answer].as_str().unwrap(),
            "{answer}"
        );
    }
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "check", "request": {"molfile": "x"}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.check.molfile"));
}

#[test]
fn sdf_route_gives_the_bytes_of_the_fixture() {
    // the same fixture is checked against the molkit `.wasm` and `mzlab-cli mol export` in tests/test_molkit_sdf.py
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/dati/sdf_disegno.json")).unwrap();
    let st = State::new();
    for req in [fx["request"].clone(), fx["request"].to_string().into()] {
        let p = json!({"kind": "sdf", "request": req});
        assert_eq!(
            text(st.rpc("feat/grafo", &p, &[]).unwrap()),
            fx["sdf"].as_str().unwrap()
        );
    }
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "sdf", "request": {"molfile": "x"}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.sdf.molfile"));
}

#[test]
fn cip_route_gives_the_bytes_of_the_fixture() {
    // the same fixture is checked against the molkit `.wasm` and `mzlab-cli mol cip` in tests/test_molkit_wasm.py
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/dati/cip_disegno.json")).unwrap();
    let st = State::new();
    let p = json!({"kind": "cip", "request": fx["request"].clone()});
    assert_eq!(
        text(st.rpc("feat/grafo", &p, &[]).unwrap()),
        fx["cip"].as_str().unwrap()
    );
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "cip", "request": {"molfile": "x"}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.cip.molfile"));
}

#[test]
fn descriptors_route_gives_the_bytes_of_the_fixture() {
    // the same fixture is checked against the molkit `.wasm` and `mzlab-cli mol measure` in tests/test_molkit_wasm.py
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/dati/descriptors_disegno.json")).unwrap();
    let st = State::new();
    for c in fx["cases"].as_array().unwrap() {
        let p = json!({"kind": "descriptors", "request": c["request"].clone()});
        assert_eq!(
            text(st.rpc("feat/grafo", &p, &[]).unwrap()),
            c["descriptors"].as_str().unwrap(),
            "{}",
            c["name"]
        );
    }
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "descriptors", "request": {"molfile": "x"}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.descr.molfile"));
}

/// Standard base64 with padding, written here and not borrowed from the engine: the route's own decoder is under test.
fn b64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in bytes.chunks(3) {
        let n = c
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for k in 0..4 {
            s.push(if k <= c.len() {
                A[(n >> (18 - 6 * k) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    s
}

/// The files of `tests/dati/cdxml` and the variants of `tests/test_molkit_wasm.py` (byte-order marks, UTF-16 with and without one, 8-bit text, bad UTF-8,
/// binary CDX, text that is not XML, the empty file) and the heads of a file of every length that gives each base64 padding.
fn open_inputs() -> Vec<(String, Vec<u8>)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/dati/cdxml");
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "cdxml"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    assert!(files.len() >= 10, "the fixtures of {}", dir.display());
    let base = String::from_utf8(
        files
            .iter()
            .find(|f| f.0 == "aromatics.cdxml")
            .unwrap()
            .1
            .clone(),
    )
    .unwrap();
    let utf16 = |s: &str, le: bool| {
        let mut b = if le {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for u in s.replace("UTF-8", "UTF-16").encode_utf16() {
            b.extend(if le { u.to_le_bytes() } else { u.to_be_bytes() });
        }
        b
    };
    let mut nul16 = Vec::new();
    for u in base.replace("UTF-8", "UTF-16").encode_utf16() {
        nul16.extend(u.to_le_bytes());
    }
    let latin1: Vec<u8> = base
        .replace("</page>", "<t id=\"900\"><s>Z\u{fc}rich</s></t></page>")
        .replace("UTF-8", "ISO-8859-1")
        .chars()
        .map(|c| c as u8)
        .collect();
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend(base.bytes());
    let mut cdx = b"VjCD0100\x03\x04\x02\x01".to_vec();
    cdx.extend(0..250u8);
    for (k, v) in [
        ("bom", bom),
        ("utf16le", utf16(&base, true)),
        ("utf16be", utf16(&base, false)),
        ("utf16 no mark", nul16),
        ("latin1", latin1),
        (
            "utf8 declared, bad bytes",
            b"<?xml version=\"1.0\" encoding=\"utf-8\"?><CDXML a=\"\"><s>\xff</s></CDXML>".to_vec(),
        ),
        ("crlf", base.replace('\n', "\r\n").into_bytes()),
        ("cdx", cdx),
        ("text", b"this is not a structure\n".to_vec()),
        ("empty", Vec::new()),
    ] {
        files.push((k.to_string(), v));
    }
    // every length from 0 to 4 (the paddings), so that no file is lucky
    for n in 0..5usize {
        files.push((
            format!("cdxml head {n}"),
            b"<CDXML a=\"\"/>"[..n.min(14)].to_vec(),
        ));
    }
    files
}

/// The app route gives the bytes of the function for every input, through its own base64 decoder.
#[test]
fn open_route_gives_the_bytes_of_the_function_for_every_fixture_and_variant() {
    let st = State::new();
    for (name, bytes) in &open_inputs() {
        let p = json!({"kind": "open", "request": {"data": b64(bytes)}});
        assert_eq!(
            text(st.rpc("feat/grafo", &p, &[]).unwrap()),
            mzlab_feat::chem::structure_file::open_json_bytes(bytes).unwrap(),
            "{name}"
        );
    }
    // a file over the limit keeps its key, as the .wasm throws it
    let big = vec![b' '; mzlab_feat::chem::structure_file::MAX_BYTES + 1];
    let err = st.rpc(
        "feat/grafo",
        &json!({"kind": "open", "request": {"data": b64(&big)}}),
        &[],
    );
    assert_eq!(err.err().map(|e| e.key), Some("error.open.tooBig"));
    // the size alone, as the page asks it before reading a dropped file: the limit, or the refusal with the real size
    let max = mzlab_feat::chem::structure_file::MAX_BYTES;
    let asked = |size: usize| {
        st.rpc(
            "feat/grafo",
            &json!({"kind": "open", "request": {"size": size}}),
            &[],
        )
    };
    assert_eq!(text(asked(1000).unwrap()), format!("{{\"limit\":{max}}}"));
    let Err(e) = asked(max + 1000) else {
        panic!("a file over the limit is refused")
    };
    assert_eq!(e.key, "error.open.tooBig");
    assert_eq!(e.params[0], ("size", (max + 1000).to_string()));
    for size in [json!(-1), json!(null)] {
        let r = st.rpc(
            "feat/grafo",
            &json!({"kind": "open", "request": {"size": size}}),
            &[],
        );
        assert_eq!(r.err().map(|e| e.key), Some("error.open.request"));
    }
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "open", "request": {"data": "not base64!"}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.open.request"));
}

#[test]
fn link_route_gives_the_golden_bytes() {
    let st = State::new();
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../tests/dati/link_golden.json")).unwrap();
    for c in &cases {
        let p = json!({"kind": "link", "request": c["request"]});
        assert_eq!(
            text(st.rpc("feat/grafo", &p, &[]).unwrap()),
            c["link"].as_str().unwrap()
        );
    }
    let bad = st.rpc("feat/grafo", &json!({"kind": "link", "request": "{}"}), &[]);
    assert_eq!(bad.err().map(|e| e.key), Some("error.link.request"));
}

#[test]
fn hilite_route_gives_the_bytes_of_the_fixture() {
    // the same fixture is checked against the molkit `.wasm` and `mzlab-cli mol hilite` in tests/test_molkit_wasm.py
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/dati/hilite_disegno.json")).unwrap();
    let st = State::new();
    for (req, want) in [("request", "hilite"), ("lr", "hilite_lr")] {
        let p = json!({"kind": "hilite", "request": fx[req].clone()});
        assert_eq!(
            text(st.rpc("feat/grafo", &p, &[]).unwrap()),
            fx[want].as_str().unwrap(),
            "{req}"
        );
    }
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "hilite", "request": {"molfile": "x"}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.hilite.molfile"));
}
