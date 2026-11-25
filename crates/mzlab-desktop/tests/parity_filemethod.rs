//! `file method` in the app (`rpc.rs` op `filemethod`) gives the bytes of `mzlab-cli file method` for the same file: the page hands the `method/1` and
//! `items/1` texts it holds, the CLI reads them from the file. The wasm leg is `tests/test_file_method_wasm.py`.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::json;

/// A product-ion mzML (precursor 253.1, CE 35) whose file name carries another mass: the warning rides in both.
fn mzml() -> String {
    let scan = "<spectrum index=\"0\" id=\"scan=0\" defaultArrayLength=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"2\"/>\
<cvParam cvRef=\"MS\" accession=\"MS:1000127\" name=\"centroid spectrum\" value=\"\"/><scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"1\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/></scan></scanList>\
<precursorList count=\"1\"><precursor><selectedIonList count=\"1\"><selectedIon><cvParam cvRef=\"MS\" accession=\"MS:1000744\" name=\"selected ion m/z\" value=\"253.1\"/></selectedIon></selectedIonList>\
<activation><cvParam cvRef=\"MS\" accession=\"MS:1000045\" name=\"collision energy\" value=\"35\"/></activation></precursor></precursorList>\
<binaryDataArrayList count=\"2\"><binaryDataArray encodedLength=\"12\"><cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam cvRef=\"MS\" accession=\"MS:1000576\" name=\"no compression\"/><cvParam cvRef=\"MS\" accession=\"MS:1000514\" name=\"m/z array\"/><binary>AAAAAAAAWUA=</binary></binaryDataArray>\
<binaryDataArray encodedLength=\"12\"><cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam cvRef=\"MS\" accession=\"MS:1000576\" name=\"no compression\"/><cvParam cvRef=\"MS\" accession=\"MS:1000515\" name=\"intensity array\"/><binary>AAAAAAAAWUA=</binary></binaryDataArray></binaryDataArrayList></spectrum>";
    format!("<?xml version=\"1.0\"?><mzML xmlns=\"http://psi.hupo.org/ms/mzml\"><run id=\"r\"><spectrumList count=\"1\">{scan}</spectrumList></run></mzML>")
}

#[test]
fn the_app_gives_the_bytes_of_file_method() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Std_237.1.mzML");
    std::fs::write(&file, mzml()).unwrap();
    let path = file.to_string_lossy().into_owned();
    let cli = |a: &[&str]| {
        let o = mzlab_cli::execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(o.code, 0, "{}", o.stderr);
        String::from_utf8(o.stdout).unwrap()
    };
    let want = cli(&["file", "method", &path]);
    let items = cli(&["file", "items", &path, "--no-points"]);
    let bytes = std::fs::read(&file).unwrap();
    let mut m = mzlab_core::mzmlhead::Mzml::new(bytes.clone(), bytes.len() as u64);
    let method = m.method("Std_237.1.mzML", mzlab_core::method_fit::DOC_MAX as u64).unwrap().unwrap_or_default();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    let got = match st.rpc("filemethod", &json!({"sample": null, "method": method, "items": items.trim_end()}), &[]).unwrap() {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    };
    assert_eq!(got + "\n", want);
    assert!(want.contains("\"key\":\"warn.file.nameMz\""), "{want}");
    // a bad items text is the engine's error, not a panic
    assert!(st.rpc("filemethod", &json!({"sample": null, "method": "", "items": "{"}), &[]).is_err());
}
