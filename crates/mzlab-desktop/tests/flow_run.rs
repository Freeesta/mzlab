//! `flow/run` of the app: a plan of the graphical Flow runs on the registered files with the engine of `mzlab-cli`, and a file that is not registered is refused.
use std::path::Path;

use mzlab_core::mzmlwrite::{Header, MzmlWriter, Opzioni};
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"a.mzML","type":"raw"}],"steps":[
    {"id":"c","step":"convert.mzml@1","in":{"raw":"file:a.mzML"}}]}"#;

/// The example full scan with numpress arrays: the one open format `convert.mzml` takes without a vendor file.
fn numpress_copy(to: &Path) {
    let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi/FullScan_t10.mzML");
    let mut sample = mzlab_core::mzml::read(&std::fs::read(ex).unwrap()[..]).unwrap();
    let mut w = MzmlWriter::con_opzioni(
        Vec::new(),
        &Header::default(),
        Opzioni {
            numpress: true,
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(sample.spectra.len()).unwrap();
    for s in &mut sample.spectra {
        s.wide = None;
        w.spectrum(s).unwrap();
    }
    w.close_spectra().unwrap();
    std::fs::write(to, w.finish().unwrap()).unwrap();
}

#[test]
fn a_plan_runs_on_the_registered_files_and_an_unregistered_one_is_refused() {
    let data = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    let r = |p: Value| st.rpc("flow/run", &p, &[]);
    let missing = r(json!({ "plan": PLAN }))
        .err()
        .expect("a.mzML is not registered");
    assert_eq!(missing.key, "err.flow.input.missing");
    assert!(r(json!({})).is_err(), "no plan");
    let src = data.path().join("a.mzML");
    numpress_copy(&src);
    let (id, _) = st.pick(src).unwrap();
    st.rpc("lr_put", &json!({ "name": "a.mzML", "fid": id }), &[])
        .unwrap();
    let out: Value = serde_json::from_str(&text(r(json!({ "plan": PLAN })).unwrap())).unwrap();
    let prov = out["prov"].as_str().unwrap();
    assert!(
        prov.contains("\"schema\":\"prov/1\"") && !prov.contains("\"error\""),
        "{prov}"
    );
    assert!(
        Path::new(out["dir"].as_str().unwrap())
            .join("c-0.mzML")
            .metadata()
            .unwrap()
            .len()
            > 0
    );
}
