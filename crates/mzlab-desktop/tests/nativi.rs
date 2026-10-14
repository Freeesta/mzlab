//! A sample read as the maker wrote it (a synthetic Agilent `.D` folder): `natsamples` lists it, `lr_put` with `nat` registers it without copying the
//! members, `lr_index` builds its `.mzidx` from the reader, the routes answer, `nat2mzml` writes its mzML, and a new run of the app finds it again.
#[path = "../../mzlab-core/tests/agilent_synth/mod.rs"]
mod agilent_synth;

use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

fn state(dir: &std::path::Path) -> State {
    let st = State::new();
    st.set_data_folder(dir.join("app"));
    st.set_index_folder(dir.join("idx")).unwrap();
    st
}

#[test]
fn a_native_d_is_listed_indexed_read_and_written_as_mzml() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path().join("src/x.D/AcqData");
    std::fs::create_dir_all(&d).unwrap();
    let (idx, peak) = agilent_synth::encode(&agilent_synth::sample());
    std::fs::write(d.join("MSScan.bin"), idx).unwrap();
    std::fs::write(d.join("MSPeak.bin"), peak).unwrap();
    let st = state(dir.path());
    let r = |op: &str, p: Value| st.rpc(op, &p, &[]);
    let mut members = Vec::new();
    for n in ["MSScan.bin", "MSPeak.bin"] {
        let (fid, _) = st.pick(d.join(n)).unwrap();
        members.push(json!({"name": format!("x.D/AcqData/{n}"), "fid": fid}));
    }

    let listed: Value =
        serde_json::from_str(&text(r("natsamples", json!({"members": members})).unwrap())).unwrap();
    assert_eq!(listed["samples"][0]["scans"], 3, "{listed}");

    let nat = json!({"key": "k", "c": 0, "id": 0, "centroid": false, "members": members});
    let put: Value = serde_json::from_str(&text(
        r("lr_put", json!({"name": "x", "nat": nat})).unwrap(),
    ))
    .unwrap();
    assert!(put["size"].as_u64().unwrap() > 0, "{put}");
    r("lr_index", json!({"name": "x"})).unwrap();
    let entry = text(r("lr_entry", json!({"name": "x"})).unwrap());
    assert!(entry.contains("\"nat\":true"), "{entry}");
    assert!(text(r("idxopen", json!({"slot": 0, "name": "x"})).unwrap()).contains("\"scans\":3"));
    let tic = text(
        r(
            "route",
            json!({"slot": 0, "path": "/api/chrom", "query": "?k=0&kind=tic&level=1"}),
        )
        .unwrap(),
    );
    assert!(tic.starts_with("{\"rt\":["), "{tic}");

    let out: Value = serde_json::from_str(&text(
        r("nat2mzml", json!({"name": "x", "pid": 7})).unwrap(),
    ))
    .unwrap();
    assert!(out["size"].as_u64().unwrap() > 500, "{out}");

    let st2 = state(dir.path()); // a new run of the app on the same folders
    let boot = text(st2.rpc("lr_boot", &json!({}), &[]).unwrap());
    assert!(
        boot.contains("\"nat\":true") && boot.contains("\"indexed\":true"),
        "{boot}"
    );
    assert!(text(
        st2.rpc("idxopen", &json!({"slot": 0, "name": "x"}), &[])
            .unwrap()
    )
    .contains("\"scans\":3"));
    st2.rpc("lr_remove", &json!({"name": "x"}), &[]).unwrap();
    assert!(
        d.join("MSScan.bin").exists(),
        "the files of the student are never touched"
    );
}
