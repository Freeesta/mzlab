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
    // api/canali reads the chromatograms of the mzML too (op `canali`, as the wasm engine); the plain route then serves the same channels from the cache
    let canali = text(r("canali", json!({"slot": 0, "name": name, "fid": id})).unwrap());
    assert!(canali.starts_with("{\"v\":\"canali/1\""), "{canali}");
    assert_eq!(text(r("route", json!({"slot": 0, "path": "/api/canali", "query": "?k=0"})).unwrap()), canali);
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

#[test]
fn molecules_of_the_user_are_a_canonical_file_that_a_session_clear_leaves_alone() {
    let data = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    let r = |op: &str, p: serde_json::Value| text(st.rpc(op, &p, &[]).unwrap());
    assert!(r("lr_molecole_archivio", json!({"mop": "leggi", "body": ""})).contains("\"molecole\":[]"));
    let doc = r#"{"schema":"molecules/1","molecole":[{"nome":"Esempina","formula":"C10H15NO"}]}"#;
    assert_eq!(r("lr_molecole_archivio", json!({"mop": "scrivi", "body": doc})), "{}");
    r("lr_clear", json!({}));
    let file = std::fs::read_to_string(data.path().join("app/molecole.json")).unwrap();
    assert_eq!(r("lr_molecole_archivio", json!({"mop": "leggi", "body": ""})), file, "the page gets the bytes of the file");
    assert!(file.starts_with("{\"molecole\":[{") && file.contains("\"id\":\"u:") && file.ends_with("\"schema\":\"molecules/1\"}"), "{file}");
    let st2 = State::new();
    st2.set_data_folder(data.path().join("app"));
    assert_eq!(text(st2.rpc("lr_molecole_archivio", &json!({"mop": "leggi", "body": ""}), &[]).unwrap()), file, "a new run reads it back");
    assert!(st2.rpc("lr_molecole_archivio", &json!({"mop": "scrivi", "body": "{}"}), &[]).is_err(), "a document that is not molecules/1 is refused");
}

#[test]
fn uploading_the_same_name_again_is_found_by_its_new_size() {
    let data = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    let first = st.upload_chunk("again.mzML", 0, 3, b"abc").unwrap().unwrap();
    assert_eq!(st.find("again.mzML", 3), Some(first));
    let second = st.upload_chunk("again.mzML", 0, 5, b"abcde").unwrap().unwrap();
    assert_eq!(second, first, "same path, same number");
    assert_eq!(st.find("again.mzML", 5), Some(first), "the lookup by the new size works");
    assert_eq!(st.find("again.mzML", 3), None, "the old size is gone");
}

/// Registers the example, builds its index and opens it in the first slot of the page's list (1_000_001); returns the engine, the name and the keep-alive folders.
fn indexed_example() -> (State, tempfile::TempDir, &'static str) {
    let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi/FullScan_t10.mzML");
    let data = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    st.set_index_folder(data.path().join("idx")).unwrap();
    let (id, _) = st.pick(ex).unwrap();
    let name = "FullScan_t10.mzML";
    st.rpc("lr_put", &json!({"name": name, "fid": id}), &[]).unwrap();
    st.rpc("lr_index", &json!({"name": name}), &[]).unwrap();
    st.rpc("lr_entry", &json!({"name": name}), &[]).unwrap();      // opens the index in the first slot of the page's list
    (st, data, name)
}

fn slot_answers(st: &State) -> bool {
    matches!(st.rpc("route", &json!({"slot": 1_000_001, "path": "/api/chrom", "query": "?k=0&kind=tic&level=1"}), &[]).unwrap(), Answer::Text(t) if !t.is_empty())
}

#[test]
fn freeing_derived_indexes_closes_them_first() {
    let (st, _keep, _) = indexed_example();
    assert!(slot_answers(&st));
    st.rpc("lr_space", &json!({"free": "derived"}), &[]).unwrap();
    assert!(!slot_answers(&st), "the index is unmapped before its file is deleted");
}
