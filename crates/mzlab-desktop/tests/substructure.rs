//! The route `feat/grafo` with `kind: "sub"` gives the same `sub/1` as `mzlab_feat::chem::substructure::search_json` (the function of
//! `mzlab-cli mol-search --sub`), with the request as an object or as text, and keeps the key of an error; a structure list (`db`) gives the
//! `subdb/1` answers of `tests/dati/sub_db.json`, the same bytes as the molkit `.wasm` and the CLI.

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

#[test]
fn structure_list_route_gives_the_shared_fixture() {
    let st = State::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fx: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("tests/dati/sub_db.json")).unwrap(),
    )
    .unwrap();
    for c in fx["cases"].as_array().unwrap() {
        let got = text(
            st.rpc(
                "feat/grafo",
                &json!({"kind": "sub", "request": c["request"]}),
                &[],
            )
            .unwrap(),
        );
        assert_eq!(got, c["answer"].as_str().unwrap(), "{}", c["name"]);
    }
    let empty = st.rpc(
        "feat/grafo",
        &json!({"kind": "sub", "request": {"query": "C", "db": "\n"}}),
        &[],
    );
    assert_eq!(empty.err().map(|e| e.key), Some("error.sub.db"));
}
