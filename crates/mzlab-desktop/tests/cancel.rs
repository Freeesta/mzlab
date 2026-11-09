//! «Cancel» of the loading in the app (`lr_cancel`), as on the site (rust-worker.js): the index of a cancelled file is not built and nothing of it stays in
//! the index folder; the page then removes the file (`lr_remove`, lr.js), which takes its indexes with it; a new upload of the name builds again.
//! Example file of the repository only (no laboratory data).
use mzlab_desktop::State;
use serde_json::json;
use std::path::Path;

fn left(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[test]
fn a_cancelled_load_leaves_no_index_and_a_new_upload_builds_again() {
    let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi/FullScan_t10.mzML");
    let data = tempfile::tempdir().unwrap();
    let idx = data.path().join("idx");
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    st.set_index_folder(idx.clone()).unwrap();
    let (id, _) = st.pick(ex.clone()).unwrap();
    let name = "FullScan_t10.mzML";
    let r = |op: &str, p: serde_json::Value| st.rpc(op, &p, &[]);
    r("lr_put", json!({"name": name, "fid": id})).unwrap();
    r("lr_cancel", json!({"names": [name]})).expect("the app answers «Cancel» of the loading");
    let e = r("lr_index", json!({"name": name}))
        .err()
        .expect("the index of a cancelled file is not built");
    assert_eq!(
        e.key, "err.load.cancelled",
        "the key the page reads to remove the file (lr.js)"
    );
    assert!(
        left(&idx).is_empty(),
        "nothing of the cancelled build stays: {:?}",
        left(&idx)
    );
    r("lr_remove", json!({"name": name})).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&match r("lr_boot", json!({})).unwrap() {
            mzlab_desktop::rpc::Answer::Text(t) => t,
            _ => panic!("bytes"),
        })
        .unwrap(),
        json!([])
    );
    // the same file uploaded again builds afresh
    r("lr_put", json!({"name": name, "fid": id})).unwrap();
    r("lr_index", json!({"name": name})).unwrap();
    assert_eq!(left(&idx), vec![format!("{name}.mzidx")]);
    assert!(ex.exists(), "the file of the student is never touched");
}

#[test]
fn removing_a_file_takes_its_index_with_it() {
    let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi/FullScan_t10.mzML");
    let data = tempfile::tempdir().unwrap();
    let idx = data.path().join("idx");
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    st.set_index_folder(idx.clone()).unwrap();
    let (id, _) = st.pick(ex).unwrap();
    let name = "FullScan_t10.mzML";
    st.rpc("lr_put", &json!({"name": name, "fid": id}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": name}), &[]).unwrap();
    assert_eq!(left(&idx), vec![format!("{name}.mzidx")]);
    st.rpc("lr_remove", &json!({"name": name}), &[]).unwrap();
    assert!(
        left(&idx).is_empty(),
        "an index with no file stays in the folder: {:?}",
        left(&idx)
    );
}
