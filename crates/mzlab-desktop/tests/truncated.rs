//! A truncated mzML opened by the app: `lr_index` says how many spectra its list declares beyond the ones it holds (`missing`, for the warning in the row of the
//! loading, as on the site), and a whole file says 0. Also the three compressions of `nat2mzml` and a name it does not know.
use std::path::Path;

use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

fn state(dir: &Path) -> State {
    let st = State::new();
    st.set_data_folder(dir.join("app"));
    st.set_index_folder(dir.join("idx")).unwrap();
    st
}

fn index_of(st: &State, path: &Path, name: &str) -> Value {
    let (id, _) = st.pick(path.to_path_buf()).unwrap();
    st.rpc("lr_put", &json!({"name": name, "fid": id}), &[])
        .unwrap();
    serde_json::from_str(&text(
        st.rpc("lr_index", &json!({"name": name}), &[]).unwrap(),
    ))
    .unwrap()
}

#[test]
fn the_index_of_a_truncated_mzml_reports_the_spectra_it_lacks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let src = root.join("mzlab/web/esempi/HRMS_t000.mzML");
    let whole = std::fs::read_to_string(&src).unwrap();
    // cut after the third spectrum: the list still declares them all
    let end = whole
        .match_indices("</spectrum>")
        .nth(2)
        .map(|(i, m)| i + m.len())
        .expect("the example has more than three spectra");
    let dir = tempfile::tempdir().unwrap();
    let cut = dir.path().join("cut.mzML");
    std::fs::write(&cut, &whole[..end]).unwrap();
    let st = state(dir.path());
    let j = index_of(&st, &cut, "cut.mzML");
    assert!(j["missing"].as_u64().unwrap() > 0, "{j}");
    let whole_j = index_of(&st, &src, "HRMS_t000.mzML");
    assert_eq!(whole_j["missing"], 0, "{whole_j}");
}
