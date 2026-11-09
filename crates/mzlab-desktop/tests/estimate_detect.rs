//! The detection estimate in the app on the fixture `tests/dati/stima_detect.json`: the route `feat/estimate_detect` answers the JSON of the `.wasm`
//! `estimate_detect` at each budget of the fixture, and `feat/detect`, under the budget of this computer, writes the bytes of the direct step.
#[path = "../../mzlab-feat-wasm/tests/estimate_common/mod.rs"]
mod common;

use mzlab_core::idxread::Index;
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use mzlab_feat_wasm::run;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

#[test]
fn the_route_answers_the_estimate_of_the_fixture_and_detects_what_fits() {
    let s = common::spec();
    let mb = s["mb"].as_u64().unwrap();
    let data = tempfile::tempdir().unwrap();
    let file = data.path().join("stima.mzML");
    std::fs::write(&file, common::mzml(&s)).unwrap();
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    st.set_index_folder(data.path().join("idx")).unwrap();
    let (id, _) = st.pick(file).unwrap();
    let name = "stima.mzML";
    st.rpc("lr_put", &json!({"name": name, "fid": id}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": name}), &[]).unwrap();
    for (budget, via) in common::cases(&s) {
        let got = text(
            st.rpc(
                "feat/estimate_detect",
                &json!({"name": name, "recipe": "", "budget_mb": budget}),
                &[],
            )
            .unwrap(),
        );
        assert_eq!(
            got,
            format!("{{\"mb\":{mb},\"via\":\"{via}\"}}"),
            "{budget}"
        );
    }
    let path = st.indexer().unwrap().path(name).unwrap();
    let b = std::fs::read(path).unwrap();
    let n = b.len() as u64;
    let mut ix = Index::open(b, n).unwrap();
    let r = run::recipe_for("", &mut ix).unwrap();
    let direct = run::detect(&mut ix, name, &r).unwrap();
    let a: Value = serde_json::from_str(&text(
        st.rpc("feat/detect", &json!({"name": name, "recipe": ""}), &[])
            .unwrap(),
    ))
    .unwrap();
    let sha: String = Sha256::digest(&direct)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect();
    assert_eq!(a["sha256"], sha);
}
