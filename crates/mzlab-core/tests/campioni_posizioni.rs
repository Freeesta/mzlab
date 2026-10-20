//! `campioni::proponi_spiegato`: the parts of the name that decided each field, and the check that the values are the ones of `proponi`.

use mzlab_core::campioni::{self, from_json, proponi};
use serde_json::Value;

/// The names of `tools/project_riferimento.py` (the parity table of `guess_sample`), all of them.
fn generated() -> Vec<String> {
    let heads = [
        "",
        "s_",
        "Sample-",
        "std_",
        "STD",
        "stds ",
        "calib_",
        "standard-",
        "cal",
        "taratura_",
        "curva ",
        "blank",
        "BLK_",
        "bianco_",
        "mrm-",
        "full_",
        "x",
    ];
    let cores = [
        "t0", "t7.5", "t30", "t7,5", "5", "0.5", "10", "120min", "2h", "1d", "3days", "3day",
        "2 ore", "4gg", "1giorno", "2giorni", "30 min", "7_2ppm", "5ppm", "0.5ppm", "10mgL",
        "5_ug-L", "2 mg/L", "3p5ppb", "1ng/mL", "1,5 ppt", "5 g/l", "2mg/l-1", "4 mgml", "9µg/L",
        "8μg/l", "1d2", "t5m", "5ms", "20 xyz", "", "12", "3.", "3,",
    ];
    let tails = ["", "_1", "-2", " rep", "_t3", "ppm", "_A", "5", "mgL"];
    let exts = ["", ".mzML", ".MZML", ".raw", ".d", ".wiff"];
    let mut out = vec![];
    for h in heads {
        for c in cores {
            for t in tails {
                out.extend(exts.iter().map(|e| format!("{h}{c}{t}{e}")));
            }
        }
    }
    out
}

/// The spans of a name as (field, text) pairs.
fn spans(files: &[&str]) -> Vec<Vec<(String, String)>> {
    let v: Vec<(String, String, String)> = files
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    campioni::proponi_spiegato(&v)
        .into_iter()
        .zip(files)
        .map(|(p, n)| {
            let ch: Vec<char> = n.chars().collect();
            p.positions
                .iter()
                .map(|q| (q.field.to_string(), ch[q.start..q.end].iter().collect()))
                .collect()
        })
        .collect()
}

fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

#[test]
fn spans_say_which_part_of_the_name_decided() {
    let got = spans(&[
        "/data/run1/X_TiO2_t005min.mzML",
        "Y_TiO2_90 s(2).mzML",
        "X_TiO2_t010min",
        "bianco_solv.mzML",
        "std_7_2ppm.mzML",
        "std_5.mzML",
        "QC_1.mzML",
        "X_dark.mzML",
        "X_infusion.mzML",
        "sample.mzML",
    ]);
    assert_eq!(
        got[0],
        pairs(&[
            ("tipo", "t005min"),
            ("tempo", "t005min"),
            ("serie", "X_TiO2")
        ])
        .into_iter()
        .skip(1)
        .collect::<Vec<_>>()
    );
    assert_eq!(
        got[1],
        pairs(&[("tempo", "90"), ("serie", "Y_TiO2"), ("replica", "(2)")])
    );
    assert_eq!(got[2], pairs(&[("tempo", "t010min"), ("serie", "X_TiO2")]));
    assert_eq!(
        got[3],
        pairs(&[("tipo", "bianco"), ("tipo_bianco", "solv")])
    );
    assert_eq!(
        got[4],
        pairs(&[("tipo", "std"), ("concentrazione", "7_2ppm")])
    );
    assert_eq!(got[5], pairs(&[("tipo", "std"), ("concentrazione", "5")]));
    assert_eq!(got[6], pairs(&[("tipo", "QC")]));
    assert_eq!(got[7], pairs(&[("tipo", "dark")]));
    assert_eq!(got[8], pairs(&[("tipo", "infusion")]));
    assert_eq!(got[9], pairs(&[]));
}

#[test]
fn spans_never_change_the_values_and_stay_inside_the_name() {
    let names = generated();
    assert!(names.len() > 30000);
    let v: Vec<(String, String, String)> = names
        .iter()
        .map(|n| (n.clone(), "ab".to_string(), String::new()))
        .collect();
    let plain = proponi(&v);
    let explained = campioni::proponi_spiegato(&v);
    assert_eq!(plain.len(), explained.len());
    for ((p, e), n) in plain.iter().zip(&explained).zip(&names) {
        assert_eq!(p, &e.campione, "{n:?}");
        let len = n.chars().count();
        for q in &e.positions {
            assert!(q.start < q.end && q.end <= len, "{n:?} {q:?}");
        }
    }
}

#[test]
fn route_with_spans() {
    let input = r#"{"files":[{"name":"X_t5.mzML","sha256":"ab"}]}"#;
    let out = campioni::route("proponi_spiegato", input).unwrap();
    assert!(out.starts_with("{\"campioni\":{\"campioni\":["), "{out}");
    assert!(
        out.ends_with(",\"posizioni\":[[{\"campo\":\"tempo\",\"fine\":4,\"inizio\":2}]]}"),
        "{out}"
    );
    let doc: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        campioni::to_json(&from_json(doc["campioni"].to_string().as_bytes()).unwrap()),
        campioni::route("proponi", input).unwrap()
    );
}
