//! `raw info` and `raw xic` on a file of alternating polarity, in the app and in the `.wasm`: the slot is opened on every section, then on `+` and on `-`
//! (the `pol` of `idxopen`), and each section must give the same bytes in both engines and its own answer (an ion of the positive scans only, one of the
//! negative scans only). The shared example holds positive scans only, so this is the fixture that makes the polarity step mean something.
use super::{compare, from_wasm, run_wasm, Got};
use mzlab_desktop::State;
use serde_json::{json, Value};

const MIXED: &str = "mixed.mzML";

/// 120 MS1 scans, 2 s apart, alternating `+` / `-`: m/z 241.3 peaks (RT 3.0) in the positive scans only and m/z 300.7 in the negative scans only.
fn mixed_mzml() -> Vec<u8> {
    use mzlab_core::mzml::Spectrum;
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let h = Header {
        run: "mixed".into(),
        sources: vec![MIXED.into()],
        software: ("mzLab".into(), "0".into()),
        ..Default::default()
    };
    let mut w = MzmlWriter::new(Vec::new(), &h).unwrap();
    w.open_spectra(120).unwrap();
    for t in 0..120usize {
        let rt = 1.0 + t as f64 * 0.0333;
        let positive = t % 2 == 0;
        let d = (rt - 3.0) / 0.15;
        // whole ion counts (libm: the same bits on every platform), one count of chemical noise at m/z 150
        let ions = libm::round(4000.0 * libm::exp(-0.5 * d * d)) as f32;
        let (mz, intensity) = if positive {
            (vec![150.0, 241.3], vec![1.0, ions])
        } else {
            (vec![150.0, 300.7], vec![1.0, ions])
        };
        let s = Spectrum {
            index: t,
            id: format!("scan={}", t + 1),
            level: 1,
            rt,
            polarity: if positive { 1 } else { -1 },
            mz,
            intensity,
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// The questions of one section: the file's polarity and ion quantum, and the two masses (one per polarity) over the peak.
fn section(pol: i32) -> Vec<Value> {
    let route =
        |path: &str, query: &str| json!({"op": "route", "slot": 0, "path": path, "query": query});
    vec![
        json!({"op": "idxopen", "slot": 0, "name": MIXED, "pol": pol}),
        route("/api/rawinfo", "?probe=16"),
        route("/api/rawxic", "?mz=241.3,300.7&rt=2.5,3.5&da=0.2"),
        route("/api/rawscan", "?rt=2.5,3.5"),
    ]
}

#[test]
fn raw_info_xic_and_scan_give_each_polarity_the_same_answer_in_the_app_and_in_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(MIXED);
    std::fs::write(&file, mixed_mzml()).unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let (fid, _) = st.pick(file.clone()).unwrap();
    st.rpc("lr_put", &json!({"name": MIXED, "fid": fid}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": MIXED}), &[]).unwrap();
    let plan: Vec<Value> = [0, 1, -1, 0].into_iter().flat_map(section).collect();
    let idx = dir.path().join("idx").join(format!("{MIXED}.mzidx"));
    let wasm = run_wasm(&pkg, dir.path(), &idx, &file, &plan);
    compare(
        &st,
        fid,
        &plan,
        &wasm,
        &[1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15],
    );
    // each section answers for itself: `+` and `-` differ from each other, and the last (every section) is the first again
    let text = |k: usize| match from_wasm(&wasm[k]) {
        Got::Text(t) => t,
        other => panic!("step {k}: {other:?}"),
    };
    let xic = |s: usize| text(4 * s + 2);
    assert_ne!(xic(1), xic(2), "the two polarities gave one answer");
    assert_eq!(xic(0), xic(3));
    assert!(xic(1).contains("\"verdict\":\"peak\""), "{}", xic(1));
    assert!(xic(2).contains("\"verdict\":\"peak\""), "{}", xic(2));
}
