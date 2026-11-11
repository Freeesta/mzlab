//! The route `feat/grafo` with `kind: "labels"` gives the same `etich/1` as `mzlab_feat::chem::labels::labels_json` (the function of the `.wasm`
//! `grafo` and of `mzlab-cli mol-label`), with the request as an object or as text, and keeps the key of an error; `kind: "cut"` gives `cut/1` the same way;
//! `kind: "fragments"` gives the `frag/1` bytes of the fixture `tests/dati/frag_radicale.json`.

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
