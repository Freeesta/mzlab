//! The ion-scale method values in the app (`rpc.rs` op `ionfacts`) are the document `mzlab_core::ionfacts::read_json` writes, and the CLI's
//! `raw xic --ions` logs the same value and the same place for the same file. The wasm leg is `tests/test_ion_facts_wasm.py`.
use mzlab_core::json::parse;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::json;

const METHOD: &str = r#"{"schema":"method/1","ms":{"periods":[{"experiments":[{"index":1,"type":"MRM","transitions":[{"q1":250.3,"q3":100,"dwell_ms":5},{"q1":260.3,"q3":90,"dwell_ms":5}]}]}]}}"#;

/// 200 SIM scans of one ion in 5 ms (200 cps) with three ions at 250.3 around 2.7 min, and `METHOD` in the `<run>`.
fn mzml() -> Vec<u8> {
    let mut h = Header {
        run: "r".into(),
        ..Default::default()
    };
    h.run_params
        .push((mzlab_core::method::USERPARAM.into(), METHOD.into()));
    let mut w = MzmlWriter::new(Vec::new(), &h).unwrap();
    w.open_spectra(200).unwrap();
    for i in 0..200 {
        let ions = if (85..90).contains(&i) { 3.0 } else { 0.0 };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 1.0 + i as f64 * 0.02,
            polarity: 1,
            filter: "+ SIM ms [250.000-251.000]".into(),
            mz: vec![150.3, 250.3],
            intensity: vec![200.0, (ions * 200.0) as f32],
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

#[test]
fn the_app_and_the_cli_read_the_same_dwell_from_the_method() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.mzML");
    let bytes = mzml();
    std::fs::write(&file, &bytes).unwrap();
    let path = file.to_string_lossy().into_owned();
    let o = mzlab_cli::execute(
        &[
            "raw", "xic", &path, "--mz", "250.3", "--rt", "2.5,3.0", "--da", "0.3", "--ions",
        ]
        .map(String::from),
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    let cli = parse(&o.stdout).unwrap();
    // the page reads the method text of the file, the CLI reads it from the file: one text
    let mut m = mzlab_core::mzmlhead::Mzml::new(bytes.clone(), bytes.len() as u64);
    let method = m
        .method("a.mzML", mzlab_core::method_fit::DOC_MAX as u64)
        .unwrap()
        .unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    let ask = |scan_type: &str| match st
        .rpc(
            "ionfacts",
            &json!({"method": method, "scan_type": scan_type}),
            &[],
        )
        .unwrap()
    {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    };
    let got = ask("");
    assert_eq!(got, mzlab_core::ionfacts::read_json(&method, "").unwrap());
    let doc = parse(got.as_bytes()).unwrap();
    assert_eq!(doc.get("query").unwrap().str(), Some("dwell_ms=5"));
    let read = &doc.get("read").unwrap().arr()[0];
    let choice = cli
        .get("provenance")
        .unwrap()
        .get("choices")
        .unwrap()
        .arr()
        .iter()
        .find(|c| c.get("what").unwrap().str() == Some("ion_method a.mzML dwell_ms"))
        .expect("the CLI logs the method value");
    assert_eq!(
        choice.get("value").unwrap().f64(),
        read.get("value").unwrap().f64()
    );
    assert!(choice
        .get("why")
        .unwrap()
        .str()
        .unwrap()
        .contains(read.get("from").unwrap().str().unwrap()));
    // a scan type that does not use the dwell reads nothing; an unknown one and a broken text are the engine's errors
    assert_eq!(ask("ems"), r#"{"query":"","read":[]}"#);
    assert!(st
        .rpc(
            "ionfacts",
            &json!({"method": method, "scan_type": "ms"}),
            &[]
        )
        .is_err());
    assert!(st
        .rpc("ionfacts", &json!({"method": "{", "scan_type": ""}), &[])
        .is_err());
}
