//! The spectrum of a high-resolution PROFILE file (the `hrp` case of `tools/bench_visore.py`, reduced): its MS1 section is single precision, so the
//! spacing of the points comes from the mzML. The index route answers nothing at first, `specexact` answers (the bridge asks it next), and the app
//! and the engine `.wasm` (`tests/rotte_wasm_node.js`, `MZLAB_WASM_PKG`, run by `tests/test_parita_rotte_app.py`) give the same text.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A reduced `hrp` of `tools/bench_visore.py`: Orbitrap profile survey scans (`FTMS` filter, 64-bit m/z on a regular grid, single-precision index
/// section), `n` scans of `points` points with a few Gaussian peaks. Numbers invented, fixed by the loop.
fn hr_profile_mzml(n: usize, points: usize) -> Vec<u8> {
    use mzlab_core::mzml::Spectrum;
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let h = Header {
        run: "hrp".into(),
        sources: vec!["hrp.mzML".into()],
        software: ("mzLab".into(), "0".into()),
        ..Default::default()
    };
    let mut w = MzmlWriter::new(Vec::new(), &h).unwrap();
    w.open_spectra(n).unwrap();
    let grid: Vec<f64> = (0..points)
        .map(|i| 100.0 + 900.0 * i as f64 / (points - 1) as f64)
        .collect();
    for t in 0..n {
        let rt = 0.3 + 0.1 * t as f64;
        let intensity = grid
            .iter()
            .enumerate()
            .map(|(i, &m)| {
                let mut y = 200.0 + (i * 7919 % 97) as f64;
                for (c, top) in [(255.1234, 1.0), (512.3456, 2.5), (801.9876, 4.0)] {
                    y += 1e6
                        * (-0.5 * ((rt - top) / 0.4f64).powi(2)).exp()
                        * (-0.5 * ((m - c) / (c / 120_000.0)).powi(2)).exp();
                }
                y as f32
            })
            .collect();
        let s = Spectrum {
            index: t,
            id: format!("controllerType=0 controllerNumber=1 scan={}", t + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: true,
            filter: "FTMS + p ESI Full ms [100.0000-1000.0000]".into(),
            mz: grid.clone(),
            intensity,
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// The windows the page asks of a profile file: one scan (the click on the TIC) and three scans.
const HRP_ONE: &str = "?k=0&rt0=2.29&rt1=2.31&level=1&precursor=&merge=1";
const HRP_THREE: &str = "?k=0&rt0=2.19&rt1=2.41&level=1&precursor=&merge=1";

/// The engine with the reduced `hrp` put, indexed and open in slot 0; returns it with the file and its id.
fn hrp_engine(dir: &Path) -> (State, PathBuf, u32) {
    let file = dir.join("hrp.mzML");
    std::fs::write(&file, hr_profile_mzml(60, 50_000)).unwrap();
    let st = State::new();
    st.set_data_folder(dir.join("app"));
    st.set_index_folder(dir.join("idx")).unwrap();
    let (fid, _) = st.pick(file.clone()).unwrap();
    st.rpc("lr_put", &json!({"name": "hrp.mzML", "fid": fid}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": "hrp.mzML"}), &[])
        .unwrap();
    (st, file, fid)
}

#[test]
fn the_spectrum_of_an_hr_profile_file_is_answered_by_the_app() {
    let dir = tempfile::tempdir().unwrap();
    let (st, _, fid) = hrp_engine(dir.path());
    let text = |op: &str, p: Value| match st.rpc(op, &p, &[]).unwrap() {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    };
    assert!(text("idxopen", json!({"slot": 0, "name": "hrp.mzML"})).contains("\"hr1\":true"));
    // the index alone cannot give the spacing of a single-precision profile section: the bridge then asks `specexact`, which reads it from the mzML
    assert_eq!(
        text(
            "route",
            json!({"slot": 0, "path": "/api/spectrum", "query": HRP_ONE})
        ),
        ""
    );
    for q in [HRP_ONE, HRP_THREE] {
        let s = text("specexact", json!({"slot": 0, "fid": fid, "query": q}));
        let v: Value = serde_json::from_str(&s).unwrap_or_else(|_| panic!("{q}: {s:.200}"));
        assert_eq!(v["mode"], "profile", "{q}: {s:.200}");
        assert!(
            v["pmz"].as_array().unwrap().len() > 1000 && !v["mz"].as_array().unwrap().is_empty(),
            "{q}: {s:.200}"
        ); // the line and its peaks
           // the spacing stays with the open index: the plain route then gives the same text
        assert_eq!(
            text(
                "route",
                json!({"slot": 0, "path": "/api/spectrum", "query": q})
            ),
            s,
            "{q}"
        );
    }
}

#[test]
fn an_hr_profile_file_gives_the_same_spectrum_in_the_app_and_in_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let (st, file, fid) = hrp_engine(dir.path());
    let plan = vec![
        json!({"op": "idxopen", "slot": 0, "name": "hrp.mzML"}),
        json!({"op": "route", "slot": 0, "path": "/api/chrom", "query": "?k=0&kind=tic&level=1"}),
        json!({"op": "route", "slot": 0, "path": "/api/spectrum", "query": HRP_ONE}),
        json!({"op": "specexact", "slot": 0, "query": HRP_ONE}),
        json!({"op": "specexact", "slot": 0, "query": HRP_THREE}),
        json!({"op": "route", "slot": 0, "path": "/api/spectrum", "query": HRP_THREE}),
    ];
    let plan_file = dir.path().join("plan.json");
    std::fs::write(&plan_file, serde_json::to_string(&plan).unwrap()).unwrap();
    let idx = dir.path().join("idx").join("hrp.mzML.mzidx");
    let out = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/rotte_wasm_node.js"))
        .args([Path::new(&pkg), &idx, &file, &plan_file])
        .output()
        .expect("node runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wasm: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    for (k, (step, w)) in plan.iter().zip(&wasm).enumerate() {
        let mut p = step.clone();
        p["fid"] = json!(fid);
        let app = match st.rpc(step["op"].as_str().unwrap(), &p, &[]).unwrap() {
            Answer::Text(t) => t,
            Answer::Bytes(_) => panic!("bytes"),
        };
        assert_eq!(w["s"].as_str(), Some(app.as_str()), "step {k}: {step}");
        // only the first spectrum of the index is left to `specexact`
        assert_eq!(app.is_empty(), k == 2, "step {k}: {app:.200}");
    }
}
