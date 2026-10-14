//! The operations of the page's engine (`rpc`) on an example file: register, index, list, open the index in a slot of the session, ask a route; the files and
//! the notebook survive a new `State` on the same folder.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::json;
use std::path::Path;

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

#[test]
fn register_index_open_and_route() {
    let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi/FullScan_t10.mzML");
    let data = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    st.set_index_folder(data.path().join("idx")).unwrap();
    let (id, _) = st.pick(ex.clone()).unwrap();
    let name = "FullScan_t10.mzML";
    let r = |op: &str, p: serde_json::Value| st.rpc(op, &p, &[]);
    assert_eq!(text(r("lr_boot", json!({})).unwrap()), "[]");
    let put: serde_json::Value = serde_json::from_str(&text(r("lr_put", json!({"name": name, "fid": id})).unwrap())).unwrap();
    assert!(put["size"].as_u64().unwrap() > 1000);
    r("lr_index", json!({"name": name})).unwrap();
    let entry = text(r("lr_entry", json!({"name": name})).unwrap());
    assert!(entry.contains("\"kind\":\"full\""), "{entry}");
    assert!(text(r("idxopen", json!({"slot": 0, "name": name})).unwrap()).contains("\"scans\":1110"));
    let tic = text(r("route", json!({"slot": 0, "path": "/api/chrom", "query": "?k=0&kind=tic&level=1"})).unwrap());
    assert!(tic.starts_with("{\"rt\":["), "{tic}");
    assert_eq!(text(r("route", json!({"slot": 9, "path": "/api/chrom", "query": ""})).unwrap()), "", "a slot that is not open is not answered");
    assert!(r("nope", json!({})).is_err());
    let tic_of = |pol: i64| {
        r("idxopen", json!({"slot": 1, "name": name, "pol": pol})).unwrap();
        text(r("route", json!({"slot": 1, "path": "/api/chrom", "query": "?k=0&kind=tic&level=1"})).unwrap())
    };
    assert_eq!(tic_of(0), tic, "pol 0 = every section");
    assert_ne!(tic_of(-1), tic, "pol is applied to the index as set_polarity of the wasm does");
    let space: serde_json::Value = serde_json::from_str(&text(r("lr_space", json!({"free": "derived"})).unwrap())).unwrap();
    assert_eq!(space["idx"], 0, "free = \"derived\" removes every index, as the wasm does: {space}");
    r("lr_index", json!({"name": name})).unwrap();                      // made again from the source for the rest of the test
    r("lr_kv_set", json!({"key": "notebook", "value": "{\"session\":[]}"})).unwrap();

    let st2 = State::new();                                              // a new run of the app on the same folders
    st2.set_data_folder(data.path().join("app"));
    st2.set_index_folder(data.path().join("idx")).unwrap();
    let boot = text(st2.rpc("lr_boot", &json!({}), &[]).unwrap());
    assert!(boot.contains("\"indexed\":true"), "{boot}");
    assert!(text(st2.rpc("lr_kv_get", &json!({"key": "notebook"}), &[]).unwrap()).contains("session"));
    st2.rpc("lr_clear", &json!({}), &[]).unwrap();
    assert_eq!(text(st2.rpc("lr_boot", &json!({}), &[]).unwrap()), "[]");
    assert!(ex.exists(), "the file of the student is never touched");
}
