//! The op `masscal` of the desktop engine (TIC menu «Check calibration (contaminants)…») gives the bytes of `mzlab_feat::massacc::check`, the function `flow run` and the site's
//! `.wasm` call: here an `.mzidx` built apart from the app's own, the same lab list and the same parameters. The file is synthetic: ions of the lab list seen at +3 ppm in every scan.
use mzlab_core::idxread::Index;
use mzlab_core::json::parse;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use mzlab_feat::massacc::{self, Params};
use serde_json::{json, Value};

const LIST: &[u8] = include_bytes!("../../../mzlab/web/contaminanti.json");
const SCANS: usize = 20;

fn b64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in data.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            s.push(if i <= c.len() {
                T[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    s
}

fn array(acc: &str, v: &[f64]) -> String {
    let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    format!(
        r#"<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/><cvParam cvRef="MS" accession="{acc}" name=""/><binary>{}</binary></binaryDataArray>"#,
        b64(&raw)
    )
}

/// Up to eight singly charged [M+H]+ ions of the list between m/z 150 and 900, at least 2 Da apart.
fn ions() -> Vec<f64> {
    let v: Value = serde_json::from_slice(LIST).unwrap();
    let mut all: Vec<f64> = v["lists"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|l| l["items"].as_array().unwrap().iter())
        .filter(|i| i["z"] == 1 && i["pol"] == 1 && i["adduct"] == "[M+H]+")
        .filter_map(|i| i["mz"].as_f64())
        .filter(|m| (150.0..900.0).contains(m))
        .collect();
    all.sort_by(f64::total_cmp);
    let mut far: Vec<f64> = vec![];
    for m in all {
        if far.last().is_none_or(|l| m - l >= 2.0) {
            far.push(m);
        }
    }
    let step = (far.len() / 8).max(1);
    far.into_iter().step_by(step).take(8).collect()
}

/// A positive-mode MS1 file whose every scan holds the ions at +3 ppm.
fn mzml() -> Vec<u8> {
    let (mz, y): (Vec<f64>, Vec<f64>) = ions().iter().map(|m| (m * (1.0 + 3e-6), 1e6)).unzip();
    let mut x = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="{SCANS}">"#
    );
    for i in 0..SCANS {
        x += &format!(
            r#"<spectrum index="{i}" id="scan={}" defaultArrayLength="{}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/><cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="FTMS + p ESI Full ms [50.00-1000.00]"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
            i + 1,
            mz.len(),
            1.0 + i as f64 * 0.1,
            array("MS:1000514", &mz),
            array("MS:1000515", &y)
        );
    }
    x += "</spectrumList></run></mzML>";
    x.into_bytes()
}

fn on_app(st: &State, name: &str, params: &str) -> String {
    match st
        .rpc("masscal", &json!({"name": name, "params": params}), &[])
        .unwrap()
    {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

fn direct(bytes: &[u8], name: &str, params: &str) -> String {
    let mut src = bytes.to_vec();
    let p = prepare(&mut src, bytes.len() as u64, name).unwrap();
    let idx = build_mzml_to_vec(&p.table, bytes, 64 << 20).unwrap().0;
    let n = idx.len() as u64;
    let params = Params::from_value(&parse(params.as_bytes()).unwrap()).unwrap();
    let mut open = |_: usize| Index::open(idx.clone(), n);
    massacc::check::<Vec<u8>, Vec<u8>>(
        &mut open,
        &[(name.to_string(), String::new())],
        None,
        LIST,
        None,
        None,
        &params,
    )
    .unwrap()
    .json
}

#[test]
fn the_app_reports_the_bytes_of_the_shared_check() {
    let dir = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let bytes = mzml();
    let file = dir.path().join("masscal.mzML");
    std::fs::write(&file, &bytes).unwrap();
    let (fid, _) = st.pick(file).unwrap();
    st.rpc("lr_put", &json!({"name": "masscal.mzML", "fid": fid}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": "masscal.mzML"}), &[])
        .unwrap();
    for params in ["{}", r#"{"ppm":10}"#] {
        let app = on_app(&st, "masscal.mzML", params);
        assert_eq!(app, direct(&bytes, "masscal.mzML", params), "{params}");
        let v: Value = serde_json::from_str(&app).unwrap();
        assert_eq!(v["format"], "massacc/1", "{app}");
    }
    let found = serde_json::from_str::<Value>(&on_app(&st, "masscal.mzML", "{}")).unwrap();
    assert!(
        found.to_string().contains("\"scans\":20"),
        "every ion is seen in all scans: {found}"
    );
}
