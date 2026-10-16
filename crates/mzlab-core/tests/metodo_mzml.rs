//! `metodo/1` of an mzML written by another program (`Mzml::metodo`): header facts, constant and changing scan parameters, the schema, and the
//! document an mzML already carries (it wins). The mzML is built here; the parity with the raw file of the same run needs the private data.
use mzlab_core::mzmlhead::Mzml;
use serde_json::Value;

fn spectrum(i: usize, level: u32, inject: f64, ce: Option<f64>) -> String {
    let ce = ce.map_or(String::new(), |c| {
        format!("<cvParam cvRef=\"MS\" accession=\"MS:1000045\" name=\"collision energy\" value=\"{c}\"/>")
    });
    format!(
        "<spectrum index=\"{i}\" id=\"scan={i}\" defaultArrayLength=\"2\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"{level}\"/>\
<scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000927\" name=\"ion injection time\" value=\"{inject}\"/>\
<cvParam cvRef=\"MS\" accession=\"MS:1000800\" name=\"mass resolving power\" value=\"60000\"/></scan></scanList>{ce}\
<binaryDataArrayList count=\"0\"></binaryDataArrayList></spectrum>"
    )
}

fn mzml(extra_run: &str) -> Vec<u8> {
    let scans: String = (0..10)
        .map(|i| {
            if i % 2 == 0 {
                spectrum(i, 1, 50.0, None)
            } else {
                spectrum(i, 2, 20.0 + i as f64, Some(30.0))
            }
        })
        .collect();
    format!(
        "<?xml version=\"1.0\"?><mzML><cvList count=\"1\"/><fileDescription><fileContent/><sourceFileList count=\"1\">\
<sourceFile id=\"sf1\" name=\"run1.raw\" location=\"file:///x\"><cvParam cvRef=\"MS\" accession=\"MS:1000569\" name=\"SHA-1\" value=\"AABB\"/></sourceFile></sourceFileList></fileDescription>\
<referenceableParamGroupList count=\"1\"><referenceableParamGroup id=\"CommonInstrumentParams\"><cvParam cvRef=\"MS\" accession=\"MS:1001742\" name=\"LTQ Orbitrap Velos\"/>\
<cvParam cvRef=\"MS\" accession=\"MS:1000529\" name=\"instrument serial number\" value=\"SN123\"/></referenceableParamGroup></referenceableParamGroupList>\
<sampleList count=\"1\"><sample id=\"s1\" name=\"campione A\"/></sampleList>\
<softwareList count=\"1\"><software id=\"pwiz\" version=\"3.0\"><cvParam cvRef=\"MS\" accession=\"MS:1000615\" name=\"ProteoWizard software\"/></software></softwareList>\
<instrumentConfigurationList count=\"1\"><instrumentConfiguration id=\"IC1\"><referenceableParamGroupRef ref=\"CommonInstrumentParams\"/></instrumentConfiguration></instrumentConfigurationList>\
<dataProcessingList count=\"1\"><dataProcessing id=\"dp\"><processingMethod order=\"0\" softwareRef=\"pwiz\"><cvParam cvRef=\"MS\" accession=\"MS:1000544\" name=\"Conversion to mzML\"/></processingMethod></dataProcessing></dataProcessingList>\
<run id=\"r\" defaultInstrumentConfigurationRef=\"IC1\" startTimeStamp=\"2024-05-01T10:00:00Z\">{extra_run}<spectrumList count=\"10\">{scans}</spectrumList></run></mzML>"
    )
    .into_bytes()
}

fn doc(bytes: Vec<u8>) -> Value {
    let n = bytes.len() as u64;
    let s = Mzml::new(bytes, n)
        .metodo("x.mzML", 1 << 20)
        .unwrap()
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

#[test]
fn the_header_gives_instrument_serial_software_origin_and_sample() {
    let d = doc(mzml(""));
    assert_eq!(d["schema"], "metodo/1");
    assert_eq!(d["strumento"]["modello"], "LTQ Orbitrap Velos");
    assert_eq!(d["strumento"]["serie"], "SN123");
    assert_eq!(
        d["strumento"]["software"][0]["nome"],
        "ProteoWizard software"
    );
    assert_eq!(d["origine"]["formato"], "mzml");
    assert_eq!(d["origine"]["file_creato"], "2024-05-01T10:00:00Z");
    assert_eq!(d["origine"]["file"][1]["nome"], "run1.raw");
    assert_eq!(d["origine"]["file"][1]["sha1"], "aabb");
    assert_eq!(d["campione"]["nome"], "campione A");
    assert!(d["campi"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["origine"] == "mzml:dataProcessing"));
}

#[test]
fn constant_scan_parameters_go_to_ms_and_changing_ones_to_an_interval() {
    let d = doc(mzml(""));
    let exps = d["ms"]["periodi"][0]["esperimenti"].as_array().unwrap();
    assert_eq!(exps.len(), 2);
    let p = |e: &Value, id: &str| {
        e["parametri"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id)
            .cloned()
    };
    assert_eq!(p(&exps[0], "ion_injection_time").unwrap()["valore"], 50.0);
    assert_eq!(p(&exps[0], "resolving_power").unwrap()["valore"], 60000.0);
    assert_eq!(p(&exps[1], "collision_energy").unwrap()["valore"], 30.0);
    assert!(p(&exps[1], "ion_injection_time").is_none());
    let c = d["campi"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["chiave"] == "MS2 ion injection time")
        .unwrap();
    assert_eq!(c["valore"], "21..29");
    assert_eq!(c["origine"], "mzml:spectrum");
}

#[test]
fn every_key_is_known_to_the_schema() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../mzlab/web/schemi/metodo-1.json")).unwrap();
    for (key, value) in doc(mzml("")).as_object().unwrap() {
        let section = &schema["properties"][key.as_str()];
        assert!(!section.is_null(), "{key} is not in the schema");
        if let (Some(known), Some(have)) = (section["properties"].as_object(), value.as_object()) {
            for k in have.keys() {
                assert!(known.contains_key(k), "{key}.{k} is not in the schema");
            }
        }
    }
}

#[test]
fn a_document_the_file_carries_wins_and_a_file_without_run_has_none() {
    let own = "{\"schema\":\"metodo/1\",\"x\":1}";
    let up = mzlab_core::metodo::userparam_xml(own);
    let d = doc(mzml(&up));
    assert_eq!(d["x"], 1);
    let junk = b"<mzML><cvList/></mzML>".to_vec();
    let n = junk.len() as u64;
    assert_eq!(Mzml::new(junk, n).metodo("x", 1 << 20).unwrap(), None);
}

#[test]
fn hostile_and_cut_files_never_panic() {
    let full = mzml("");
    for cut in [10, 400, full.len() / 2, full.len() - 5] {
        let part = full[..cut].to_vec();
        let n = part.len() as u64;
        let _ = Mzml::new(part, n).metodo("x", 1 << 20);
    }
    let junk = b"<run <run <spectrumList <spectrum \"\"\"".to_vec();
    let n = junk.len() as u64;
    let _ = Mzml::new(junk, n).metodo("x", 1 << 20);
}
