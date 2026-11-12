//! The route `feat/grafo` with `kind: "sub"` gives the same `sub/1` as `mzlab_feat::chem::substructure::search_json` (the function of
//! `mzlab-cli mol-search --sub`), with the request as an object or as text, and keeps the key of an error.

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
fn substructure_route_equals_the_function() {
    let st = State::new();
    let req = json!({"query": "Oc1ccccc1Cl", "targets": ["Oc1ccc(Cl)cc1Cl", "CCO", "C1CC"], "stereo": true, "max": 10});
    let direct = mzlab_feat::chem::substructure::search_json(&req.to_string()).unwrap();
    assert!(direct.contains(r#""hits":[{"i":0,"#), "{direct}");
    let as_object = json!({"kind": "sub", "request": req});
    let as_text = json!({"kind": "sub", "request": req.to_string()});
    assert_eq!(text(st.rpc("feat/grafo", &as_object, &[]).unwrap()), direct);
    assert_eq!(text(st.rpc("feat/grafo", &as_text, &[]).unwrap()), direct);
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "sub", "request": {"query": "C1C", "targets": []}}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.sub.query"));
}
