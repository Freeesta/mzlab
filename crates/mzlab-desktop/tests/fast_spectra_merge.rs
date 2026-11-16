//! The binary fast path of the desktop (`State::spectra`, twin of the wasm `engine.spectra`) merges unit-resolution centroids per nominal mass: the AUTO rule of
//! `/api/spectra`. The wasm twin is held byte-equal to it by parity_routes.rs, the site's answer to the Python reference by the e2e_rust golden.
use mzlab_desktop::State;
use serde_json::json;
use std::path::{Path, PathBuf};

const NAME: &str = "FullScan_t10.mzML";

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mzlab/web/esempi")
        .join(NAME)
}

#[test]
fn the_fast_spectra_path_merges_unit_resolution_centroids_per_nominal_mass() {
    let dir = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let (fid, _) = st.pick(example()).unwrap();
    st.rpc("lr_put", &json!({"name": NAME, "fid": fid}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": NAME}), &[]).unwrap();
    st.open(0, fid).unwrap();
    let fast = st.spectra(0, 1, 0, 5, 0.5).unwrap();
    assert!(fast[1] >= 1.0, "scans came back");
    let mut w = 2usize;
    for _ in 0..fast[1] as usize {
        let np = fast[w + 2] as usize;
        let (mz, y) = (&fast[w + 3..w + 3 + np], &fast[w + 3 + np..w + 3 + 2 * np]);
        w += 3 + 2 * np;
        let nominal: Vec<i64> = mz.iter().map(|m| (m + 0.2).floor() as i64).collect();
        let mut u = nominal.clone();
        u.dedup();
        assert_eq!(u.len(), np, "one peak per nominal mass");
        let (em, ey) = mzlab_core::specpeaks::merge_unit(mz, y);
        assert_eq!(
            (em.len(), ey.len()),
            (np, np),
            "merge_unit leaves a merged scan unchanged"
        );
    }
}
