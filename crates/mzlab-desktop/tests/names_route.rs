//! «Name files by sample name» on the app: the route `lr_samples` / `mzml_names` that the page posts to (`api/samples/mzml_names`) answers the names of the function
//! the site's `.wasm` and `mzlab-cli file convert --name sample` use (`mzlab_core::samples::route_all`). Synthetic names only.

use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};

#[test]
fn the_app_answers_the_names_route_of_the_samples() {
    // a clash, a name to clean, none, a plain name given and one left to the engine (`file`)
    let body = json!({"files": [
        {"who": "s.wiff@1", "file": "s.wiff", "id": 1, "sample": "Blank"},
        {"who": "s.wiff@2", "file": "s.wiff", "id": 2, "sample": "blank"},
        {"who": "a.raw", "file": "a.raw", "sample": "x y"},
        {"who": "b.raw", "file": "b.raw", "sample": null},
        {"who": "c.mzML", "plain": "c.mzML", "sample": null},
    ]})
    .to_string();
    let st = State::new();
    let p = json!({"cop": "mzml_names", "body": body});
    let Ok(Answer::Text(app)) = st.rpc("lr_samples", &p, &[]) else {
        panic!("the app did not answer the route")
    };
    assert_eq!(
        app,
        mzlab_core::samples::route_all("mzml_names", &body).unwrap()
    );
    let v: Value = serde_json::from_str(&app).unwrap();
    let outs: Vec<&str> = v["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["out"].as_str().unwrap())
        .collect();
    assert_eq!(
        outs,
        ["Blank.mzML", "blank-2.mzML", "x_y.mzML", "b.mzML", "c.mzML"]
    );
    assert!(v["files"][1]["note"]
        .as_str()
        .unwrap()
        .contains("already taken"));
}
