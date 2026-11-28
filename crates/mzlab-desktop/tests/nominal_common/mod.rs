//! The synthetic unit-resolution profile file, the app's state with it open and the index, shared by the nominal-bin parity tests. Numbers invented.
#![allow(dead_code)]
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};

/// Four profile scans of 0.1 Da points on 100.02 + 0.1 i, a baseline of 5 and a fragment of six points from 106.62 to 107.12. Numbers invented.
pub fn split_fragment_mzml() -> Vec<u8> {
    use mzlab_core::mzml::Spectrum;
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(4).unwrap();
    let peak = [400.0f32, 800.0, 1000.0, 1000.0, 800.0, 400.0];
    for k in 0..4 {
        let mz: Vec<f64> = (0..100).map(|i| 100.02 + 0.1 * i as f64).collect();
        let intensity: Vec<f32> = (0..100usize)
            .map(|i| {
                5.0 + i
                    .checked_sub(66)
                    .and_then(|j| peak.get(j))
                    .copied()
                    .unwrap_or(0.0)
                    + k as f32
            })
            .collect();
        let s = Spectrum {
            index: k,
            id: format!("scan={}", k + 1),
            level: 1,
            rt: k as f64,
            polarity: 1,
            profile: true,
            filter: "ITMS + p ESI Full ms [100.00-110.00]".into(),
            mz,
            intensity,
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

pub fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

/// The file, the app's state with it open in slot 0 and the index the site and the CLI read.
pub struct Setup {
    pub dir: tempfile::TempDir,
    pub file: std::path::PathBuf,
    pub st: State,
    pub idx_path: std::path::PathBuf,
}

pub fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("split.mzML");
    std::fs::write(&file, split_fragment_mzml()).unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let (fid, _) = st.pick(file.clone()).unwrap();
    st.rpc("lr_put", &json!({"name": "split.mzML", "fid": fid}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": "split.mzML"}), &[])
        .unwrap();
    st.rpc("idxopen", &json!({"slot": 0, "name": "split.mzML"}), &[])
        .unwrap();
    let idx_path = dir.path().join("idx").join("split.mzML.mzidx");
    Setup {
        dir,
        file,
        st,
        idx_path,
    }
}

pub fn json_of(a: &Option<String>) -> Value {
    serde_json::from_str(a.as_ref().unwrap()).unwrap()
}
