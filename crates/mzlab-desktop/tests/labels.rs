//! The route `feat/grafo` with `kind: "labels"` gives the same `etich/1` as `mzlab_feat::chem::etichette::labels_json` (the function of the `.wasm`
//! `grafo` and of `mzlab-cli mol-label`), with the request as an object or as text, and keeps the key of an error.

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
    let direct = mzlab_feat::chem::etichette::labels_json(req).unwrap();
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
