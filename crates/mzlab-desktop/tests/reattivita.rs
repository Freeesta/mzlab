//! The route `feat/grafo` with `kind: "reactivity"` gives the same `reatt/1` as `mzlab_chem::reactivity_json` (the function of the `.wasm` `chem`),
//! with the options as an object or as text, and keeps the key of an error.

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
fn reactivity_route_equals_the_function() {
    let st = State::new();
    let direct = mzlab_chem::reactivity_json("Oc1ccccc1", r#"{"conformers":2}"#).unwrap();
    let as_object = json!({"kind": "reactivity", "smiles": "Oc1ccccc1", "opts": {"conformers": 2}});
    let as_text =
        json!({"kind": "reactivity", "smiles": "Oc1ccccc1", "opts": r#"{"conformers":2}"#});
    assert_eq!(text(st.rpc("feat/grafo", &as_object, &[]).unwrap()), direct);
    assert_eq!(text(st.rpc("feat/grafo", &as_text, &[]).unwrap()), direct);
    let bad = st.rpc(
        "feat/grafo",
        &json!({"kind": "reactivity", "smiles": "C1CC"}),
        &[],
    );
    assert_eq!(bad.err().map(|e| e.key), Some("error.chem.smiles"));
}

#[test]
fn positions_route_carries_the_prior_of_the_reactivity() {
    let st = State::new();
    let reatt = mzlab_chem::reactivity_json("Oc1ccccc1", r#"{"conformers":1}"#).unwrap();
    let peaks = "[[95.0491,100]]";
    let q = |r: &str| json!({"kind": "positions", "smiles": "Oc1ccccc1", "transform": "+O", "peaks": peaks, "tol": 0.5, "reatt": r});
    let with = text(st.rpc("feat/grafo", &q(&reatt), &[]).unwrap());
    let direct = mzlab_feat::chem::posizioni::positions_prior_json(
        "Oc1ccccc1",
        "+O",
        peaks,
        0.5,
        false,
        true,
        &reatt,
    )
    .unwrap();
    assert_eq!(with, direct);
    assert!(with.contains("\"prior\":[{\"atom\":"), "{with}");
    let without = text(st.rpc("feat/grafo", &q(""), &[]).unwrap());
    assert!(!without.contains("\"prior\""));
}
