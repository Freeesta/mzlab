//! The route `feat/grafo` with `kind: "labels"` gives the same `etich/1` as `mzlab_feat::chem::labels::labels_json` (the function of the `.wasm`
//! `grafo` and of `mzlab-cli mol-label`), with the request as an object or as text, and keeps the key of an error; `kind: "cut"` gives `cut/1` the same way;
//! `kind: "analysis"` gives `anal/1` and `kind: "sel"` `sel/1` the same way; `kind: "fragments"` gives the `frag/1` bytes of the fixture `tests/dati/frag_radicale.json`, `kind: "check"` the `check/1` bytes of `tests/dati/check_disegno.json`, `kind: "sdf"` the `sdf/1` bytes of `tests/dati/sdf_disegno.json`, and `kind: "mech"` the `mecc/1` bytes of `tests/dati/mecc_mclafferty.json`.

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
    let req = r#"{"structures":[[{"el":"Cl"},{"el":"Pt"},{"el":"Cl"},{"el":"N","h":2},{"el":"N","h":2}],[{"el":"C","iso":13,"h":4}]],"arrows":[[0,1]]}"#;
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
    assert_eq!(bad.err().map(|e| e.key), Some("error.etich.request"));
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
    assert!(direct.contains("\"sel/1\""));
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
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/dati/mecc_mclafferty.json")).unwrap();
    let got = text(
        st.rpc(
            "feat/grafo",
            &json!({"kind": "mech", "request": fx["request"]}),
            &[],
        )
        .unwrap(),
    );
    assert_eq!(got, fx["mecc"].as_str().unwrap());
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
    // the same fixture is checked against the `.wasm` `grafo` and `mzlab-cli mol-check` in tests/test_feat_grafo_wasm.py
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
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "check", "request": {"molfile": "x"}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.check.molfile"));
}

#[test]
fn sdf_route_gives_the_bytes_of_the_fixture() {
    // the same fixture is checked against the molkit `.wasm` and `mzlab-cli mol-export` in tests/test_molkit_sdf.py
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
