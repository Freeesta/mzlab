//! The `metodo/1` document of the real Agilent method folders `GC-MS/*.m` (skipped without the data repository): the compounds, qualifier ions and calibration levels
//! are the values read from `quantitative.xml` with an independent parse (Python ElementTree), every name/value of the files is in `campi`, and every key is in the schema.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod agilent_folder;

use agilent_folder::dati;
use mzlab_core::agilent::{meta, quant::metodo_m};
use serde_json::Value;
use std::path::Path;

fn files_of(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(d: &Path, rel: &str, out: &mut Vec<(String, Vec<u8>)>) {
        let mut es: Vec<_> = std::fs::read_dir(d).unwrap().flatten().collect();
        es.sort_by_key(|e| e.file_name());
        for e in es {
            let n = e.file_name().to_string_lossy().into_owned();
            let r = if rel.is_empty() {
                n.clone()
            } else {
                format!("{rel}/{n}")
            };
            if e.path().is_dir() {
                walk(&e.path(), &r, out);
            } else {
                out.push((r, std::fs::read(e.path()).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out
}

fn document(dir: &Path) -> (Value, usize) {
    let owned = files_of(dir);
    let files: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let n_items = meta::collect(&files)
        .iter()
        .filter(|i| i.name != "text" && !i.value.is_empty())
        .count();
    (
        serde_json::from_str(&metodo_m(&name, &files)).expect("JSON"),
        n_items,
    )
}

fn schema_has_all_keys(doc: &Value, comp: &[Value]) {
    let schema: Value =
        serde_json::from_str(include_str!("../../../mzlab/web/schemi/metodo-1.json")).unwrap();
    for key in doc.as_object().unwrap().keys() {
        assert!(
            !schema["properties"][key.as_str()].is_null(),
            "{key} is not in the schema"
        );
    }
    for c in comp {
        for key in c.as_object().unwrap().keys() {
            assert!(
                !schema["properties"]["quantificazione"]["properties"]["composti"]["items"]
                    ["properties"][key.as_str()]
                .is_null(),
                "composti.{key}"
            );
        }
    }
}

#[test]
fn the_geosmin_and_mib_methods_say_what_quantitative_xml_says() {
    let Some(d) = dati() else { return };
    for (m, rts, sim) in [("scan", [7.195, 10.0], false), ("SIM", [7.7, 10.6], true)] {
        let (doc, n_items) = document(
            &d.join("GC-MS")
                .join(format!("Geosmin and MIB method for {m}.m")),
        );
        assert_eq!(doc["schema"], "metodo/1");
        assert_eq!(doc["origine"]["formato"], "agilent-m");
        let q = &doc["quantificazione"];
        let comp = q["composti"].as_array().unwrap();
        assert_eq!(comp.len(), 2, "{m}");
        assert_eq!(
            (comp[0]["nome"].as_str(), comp[1]["nome"].as_str()),
            (Some("2-mib"), Some("Geosmin"))
        );
        assert_eq!(
            (comp[0]["rt_min"].as_f64(), comp[1]["rt_min"].as_f64()),
            (Some(rts[0]), Some(rts[1]))
        );
        assert_eq!(
            comp[0]["tipo_scansione"],
            if sim { "Ms1SIM" } else { "Ms1Scan" }
        );
        assert!(comp[0].get("mz").is_none(), "a target m/z of 0 is not set");
        let ions = |i: usize| -> Vec<(f64, f64)> {
            comp[i]["ioni"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| {
                    (
                        x["mz"].as_f64().unwrap(),
                        x["risposta_relativa_pct"].as_f64().unwrap(),
                    )
                })
                .collect()
        };
        let near = |a: Vec<(f64, f64)>, b: &[(f64, f64)]| {
            assert_eq!(a.len(), b.len());
            for (x, y) in a.iter().zip(b) {
                assert!((x.0 - y.0).abs() < 1e-9 && x.1 == y.1, "{x:?} {y:?}");
            }
        };
        near(
            ions(0),
            &if sim {
                vec![(108.1, 25.0), (135.1, 15.0), (95.1, 25.0)]
            } else {
                vec![(108.1, 25.0), (135.1, 15.0)]
            },
        );
        near(ions(1), &[(111.0, 35.0), (125.0, 20.0), (182.0, 10.0)]);
        let lv: Vec<f64> = comp[0]["livelli"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["conc"].as_f64().unwrap())
            .collect();
        assert_eq!(lv, [0.01, 0.05, 0.1, 0.25, 1.0, 5.0, 10.0]);
        assert_eq!(comp[0]["unita_conc"], "ng/ml");
        assert_eq!(q["finestre"]["unita"], "Percent");
        // nothing the files say is dropped
        assert_eq!(doc["campi"].as_array().unwrap().len(), n_items, "{m}");
        assert!(doc["campi"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["mappato"] == "quantificazione"));
        schema_has_all_keys(&doc, comp);
    }
}

#[test]
fn a_method_folder_without_quantification_gives_the_files_only() {
    let doc: Value = serde_json::from_str(&metodo_m(
        "x.m",
        &[
            ("a.bin", b"\x00\x01"),
            ("DaMethod/Quant/quantitative.xml", b"<broken"),
        ],
    ))
    .unwrap();
    assert!(doc.get("quantificazione").is_none());
    assert_eq!(doc["origine"]["file"].as_array().unwrap().len(), 2);
    assert!(!doc["avvisi"].as_array().unwrap().is_empty());
}
