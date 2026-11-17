//! `idxopen` of the app as the site's worker: a session slot of a file whose list slot is open shares its scan table (`routes_open::open_slot`),
//! with the same answers as a slot that opens the index itself; a file without an index is `error.noIndex`.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::rpc::Answer;
use crate::State;
use serde_json::json;
use std::sync::Arc;

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => String::from("bytes"),
    }
}

#[test]
fn a_session_slot_shares_the_scan_table_of_the_list_slot() {
    let ex = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mzlab/web/esempi/FullScan_t10.mzML");
    let data = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    st.set_index_folder(data.path().join("idx")).unwrap();
    let (id, _) = st.pick(ex).unwrap();
    let name = "FullScan_t10.mzML";
    let r = |op: &str, p: serde_json::Value| st.rpc(op, &p, &[]).map(text);
    r("lr_put", json!({"name": name, "fid": id})).unwrap();
    r("lr_index", json!({"name": name})).unwrap();
    r("lr_entry", json!({"name": name})).unwrap();
    let lr = *st.eng.lock().unwrap().lr_slot.get(name).unwrap();
    assert!(
        st.eng.lock().unwrap().idx.contains_key(&lr),
        "the loading table opened the list slot"
    );

    let shared = r("idxopen", json!({"slot": 0, "name": name})).unwrap();
    st.drop_lr_index(name);
    let own = r("idxopen", json!({"slot": 1, "name": name})).unwrap();
    assert_eq!(shared, own);
    {
        let eng = st.eng.lock().unwrap();
        assert!(!Arc::ptr_eq(&eng.idx[&0].idx.scans, &eng.idx[&1].idx.scans));
    }
    r("lr_entry", json!({"name": name})).unwrap();
    r("idxopen", json!({"slot": 0, "name": name})).unwrap();
    {
        let eng = st.eng.lock().unwrap();
        assert!(
            Arc::ptr_eq(&eng.idx[&0].idx.scans, &eng.idx[&lr].idx.scans),
            "one table for the two slots of the file"
        );
    }
    let q = |slot: u32| {
        r(
            "route",
            json!({"slot": slot, "path": "/api/chrom", "query": "?k=0&kind=tic&level=1"}),
        )
        .unwrap()
    };
    assert_eq!(q(0), q(1));

    let e = r("idxopen", json!({"slot": 2, "name": "absent.mzML"}))
        .err()
        .map(|e| e.key);
    assert_eq!(e, Some("error.noIndex"));
}
