//! The `metodo/1` document of the real Agilent runs (skipped without the data repository): what the sections say, that no field of the side files is
//! left out of `campi`, that an anonymous document keeps no person or place, and that the mzML carries the same document.

mod agilent_folder;

use agilent_folder::*;
use mzlab_core::agilent::AgilentD;
use mzlab_core::metodo::{from_userparam, Source};
use serde_json::Value;

const TEXTS: [&str; 3] = ["acqmeth.txt", "runstart.txt", "IndexedDataConverter.log"];

fn source(_: &AgilentD, anonymous: bool) -> Source {
    Source {
        format: "agilent-d",
        files: vec![],
        sample: None,
        anonymous,
    }
}

fn parse(doc: &str) -> Value {
    serde_json::from_str(doc).expect("the document is JSON")
}

#[test]
fn the_document_of_a_real_run_says_what_the_files_say() {
    let Some(all) = folders() else { return };
    for f in all {
        let d = Folder::load(&f).open();
        let label = d.name.clone();
        let doc = parse(&d.metodo(&source(&d, false)));
        assert_eq!(doc["schema"], "metodo/1", "{label}");
        assert_eq!(doc["origine"]["formato"], "agilent-d", "{label}");
        // every key of the document and of its sections is one the schema knows
        let schema: Value =
            serde_json::from_str(include_str!("../../../mzlab/web/schemi/metodo-1.json")).unwrap();
        for (key, value) in doc.as_object().unwrap() {
            let section = &schema["properties"][key.as_str()];
            assert!(!section.is_null(), "{label}: {key} is not in the schema");
            if let (Some(known), Some(have)) =
                (section["properties"].as_object(), value.as_object())
            {
                for k in have.keys() {
                    assert!(
                        known.contains_key(k),
                        "{label}: {key}.{k} is not in the schema"
                    );
                }
            }
        }
        let dev = d.device.as_ref().unwrap();
        assert_eq!(doc["strumento"]["serie"], dev.serial.as_str(), "{label}");
        assert!(
            doc["strumento"]["modello"]
                .as_str()
                .unwrap()
                .contains(&dev.model),
            "{label}"
        );
        assert_eq!(doc["campione"]["nome"], d.sample_name().unwrap(), "{label}");
        assert_eq!(doc["ms"]["sorgente"]["tipo"], "EI", "{label}");
        let exps = doc["ms"]["periodi"][0]["esperimenti"].as_array().unwrap();
        let m = d.method.as_ref().unwrap();
        if m.mode == "SIM" {
            assert_eq!(
                exps.len(),
                m.groups.len(),
                "{label}: one experiment per group"
            );
            let ions: usize = m.groups.iter().map(|g| g.ions.len()).sum();
            let listed: usize = exps
                .iter()
                .map(|e| {
                    e["parametri"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|p| p["unita"] == "m/z")
                        .count()
                })
                .sum();
            assert_eq!(listed, ions, "{label}: ions of the groups");
        } else {
            let mz = exps[0]["mz"].as_array().unwrap();
            assert_eq!(
                (mz[0].as_f64(), mz[1].as_f64()),
                (m.low_mass, m.high_mass),
                "{label}"
            );
        }
        assert!(
            doc["grezzo"].as_array().unwrap().len() >= 2,
            "{label}: the texts"
        );
    }
}

#[test]
fn no_field_of_the_side_files_is_left_out_of_campi() {
    let Some(all) = folders() else { return };
    for f in all {
        let d = Folder::load(&f).open();
        let doc = parse(&d.metodo(&source(&d, false)));
        let campi = doc["campi"].as_array().unwrap();
        let in_campi = |i: &&mzlab_core::agilent::meta::Item| {
            !(i.name == "text" && TEXTS.contains(&i.origin.as_str())) && !i.value.is_empty()
        };
        let wanted = d.metadata.iter().filter(in_campi).count();
        assert_eq!(campi.len(), wanted, "{}: fields in campi", d.name);
        for i in d.metadata.iter().filter(in_campi) {
            assert!(
                campi.iter().any(|c| c["origine"] == i.origin.as_str()
                    && c["chiave"] == i.name.as_str()
                    && c["valore"] == i.value.as_str()),
                "{}: {} {} is missing",
                d.name,
                i.origin,
                i.name
            );
        }
        // the whole texts are in grezzo
        for t in d
            .metadata
            .iter()
            .filter(|i| i.name == "text" && TEXTS.contains(&i.origin.as_str()))
        {
            let found = doc["grezzo"].as_array().unwrap().iter().any(|g| {
                g["testo"].as_str().is_some_and(|x| {
                    t.value
                        .trim()
                        .starts_with(x.chars().take(40).collect::<String>().as_str())
                })
            });
            assert!(found, "{}: text of {} is not in grezzo", d.name, t.origin);
        }
    }
}

#[test]
fn an_anonymous_document_keeps_no_person_or_place() {
    let Some(all) = folders() else { return };
    for f in all {
        let d = Folder::load(&f).open();
        let anon = d.metodo(&source(&d, true));
        let who = [
            d.device.as_ref().unwrap().serial.clone(),
            d.field(
                "AcqData-CumulativeAuditTrail.xml",
                "AuditTrailDataSet/AuditTrailEntry/ComputerName",
            )
            .unwrap()
            .to_string(),
            d.field(
                "AcqData/sequence.xml",
                "SequenceTableDataSet@SequenceOperator",
            )
            .unwrap()
            .to_string(),
        ];
        for w in who.iter().filter(|w| w.len() >= 3) {
            // a user name is a word of its own: «Manu» is not «Manual»
            let whole = anon
                .match_indices(w.as_str())
                .any(|(at, m)| !anon[at + m.len()..].starts_with(|c: char| c.is_alphanumeric()));
            assert!(!whole, "{}: {w} is still in the anonymous document", d.name);
        }
        let doc = parse(&anon);
        assert!(
            doc["operatore"].is_null() && doc["strumento"]["serie"].is_null(),
            "{}",
            d.name
        );
        // the paths the folder names (data, method, sequence) are gone; the install folder of the acquisition program in the run start is no person
        for i in d
            .metadata
            .iter()
            .filter(|i| i.value.get(1..3) == Some(":\\"))
        {
            let escaped = i.value.replace('\\', "\\\\");
            assert!(
                !anon.contains(&escaped),
                "{}: path {} is left",
                d.name,
                i.value
            );
        }
    }
}

#[test]
fn the_mzml_carries_the_same_document() {
    let Some(all) = folders() else { return };
    let mut fo = Folder::load(&all[0]);
    let d = fo.open();
    let mut out = Vec::new();
    let n = fo.peak.len() as u64;
    d.to_mzml(&mut fo.peak, n, &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    let head = &text[text.find("<run ").unwrap()..];
    let doc = from_userparam(head).expect("the mzML has the document");
    let json = parse(&doc);
    assert_eq!(json["schema"], "metodo/1");
    let sha = json["origine"]["file"].as_array().unwrap();
    assert!(sha
        .iter()
        .any(|f| f["nome"] == "AcqData/MSPeak.bin"
            && f["sha1"].as_str().is_some_and(|h| h.len() == 40)));
    // an anonymous conversion has the serial number neither in the header nor in the document
    let serial = d.device.as_ref().unwrap().serial.clone();
    let mut fo = Folder::load(&all[0]);
    let anon = fo.open().anonymous(true);
    let mut out = Vec::new();
    let n = fo.peak.len() as u64;
    anon.to_mzml(&mut fo.peak, n, &mut out).unwrap();
    assert!(!String::from_utf8(out).unwrap().contains(&serial));
}
