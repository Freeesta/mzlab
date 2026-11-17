//! The binary fast path of the desktop (`State::spectra`, twin of the wasm `engine.spectra`) merges unit-resolution centroids per nominal mass: the AUTO rule of
//! `/api/spectra`. Compared with Python's answer (`tools/spectra_reference.py`); the wasm twin is held byte-equal to it by parity_routes.rs.
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

const NAME: &str = "FullScan_t10.mzML";

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mzlab/web/esempi")
        .join(NAME)
}

/// Python's `/api/spectra` for the example (`tools/spectra_reference.py`): `{auto: [{sid, mz, y}], off: [...]}`; None without python3 / numpy.
fn python() -> Option<Value> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    match Command::new("python3")
        .current_dir(&root)
        .arg("tools/spectra_reference.py")
        .arg(example())
        .output()
    {
        Ok(o) if o.status.success() => serde_json::from_slice(&o.stdout).ok(),
        other => {
            assert!(
                std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
                "the reference did not run: {other:?}"
            );
            eprintln!("python3/numpy not available: test skipped");
            None
        }
    }
}

#[test]
fn the_fast_spectra_path_gives_the_peaks_python_gives_with_merge_omitted() {
    let Some(py) = python() else { return };
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
    let nums = |v: &Value| -> Vec<f64> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect()
    };
    let (auto, off) = (
        py["auto"].as_array().unwrap(),
        py["off"].as_array().unwrap(),
    );
    assert_eq!(fast[1] as usize, auto.len(), "the same scans");
    let mut w = 2usize;
    let mut merged_something = false;
    for (a, o) in auto.iter().zip(off) {
        let np = fast[w + 2] as usize;
        let (mz, y) = (&fast[w + 3..w + 3 + np], &fast[w + 3 + np..w + 3 + 2 * np]);
        w += 3 + 2 * np;
        assert_eq!(
            fast[w - 3 - 2 * np] as i64,
            a["sid"].as_i64().unwrap(),
            "scan number"
        );
        let (pm, py) = (nums(&a["mz"]), nums(&a["y"]));
        assert_eq!(np, pm.len(), "scan {}: the number of peaks", a["sid"]);
        for j in 0..np {
            // the engine holds intensities as float32, Python reads the mzML as float64 (the example writes 5400000.1): the intensity differs by the float32
            // resolution and the intensity-weighted m/z by at most one unit of its third decimal; the peaks themselves and their number are the same
            assert!(
                (mz[j] - pm[j]).abs() < 0.0015,
                "scan {} peak {j}: m/z {} vs Python's {}",
                a["sid"],
                mz[j],
                pm[j]
            );
            assert!(
                (y[j] - py[j]).abs() <= 1e-6 * py[j].abs() + 0.15,
                "scan {} peak {j}: intensity {} vs Python's {}",
                a["sid"],
                y[j],
                py[j]
            );
        }
        merged_something |= a["mz"].as_array().unwrap().len() < o["mz"].as_array().unwrap().len();
    }
    assert!(
        merged_something,
        "the example has centroids of one nominal mass: merge=0 keeps more peaks than AUTO"
    );
}
