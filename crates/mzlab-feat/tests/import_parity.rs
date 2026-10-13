//! The readers of the tables of other programs (`mzlab_feat::import`) give the same output as the JS readers they replace: `tests/data/untargeted/golden/golden.json`
//! (written by `genera.js` from `TP_Mine/js/25_untarget.js` on the fixtures of `tests/data/untargeted/`) holds the answers of the JS; the tables must be equal
//! (the masses come from `mzlab-core`, so the ppm of a structure check may differ in the last digits).

use mzlab_feat::import::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/untargeted")
}

fn golden() -> Value {
    serde_json::from_slice(&std::fs::read(data().join("golden/golden.json")).unwrap()).unwrap()
}

/// Equal JSON, numbers by value (7 and 7.0 are the same).
fn same(a: &Value, b: &Value, at: &str) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => assert_eq!(x.as_f64(), y.as_f64(), "{at}"),
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{at}: length");
            x.iter()
                .zip(y)
                .enumerate()
                .for_each(|(i, (p, q))| same(p, q, &format!("{at}[{i}]")));
        }
        (Value::Object(x), Value::Object(y)) => {
            let (mut kx, mut ky): (Vec<_>, Vec<_>) = (x.keys().collect(), y.keys().collect());
            kx.sort();
            ky.sort();
            assert_eq!(kx, ky, "{at}: keys");
            x.iter()
                .for_each(|(k, v)| same(v, &y[k], &format!("{at}.{k}")));
        }
        _ => assert_eq!(a, b, "{at}"),
    }
}

fn s(v: &Value) -> &str {
    v.as_str().unwrap()
}

fn opt(v: &Value) -> Option<f64> {
    v.as_f64()
}

#[test]
fn the_tables_equal_those_of_the_js_readers() {
    let g = golden();
    for c in g["cases"].as_array().unwrap() {
        let crlf = c.get("variant").is_some_and(|v| v == "crlf");
        let files: Vec<InFile> = c["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let text = std::fs::read_to_string(data().join(s(p))).unwrap();
                let name = s(p).rsplit('/').next().unwrap().to_string();
                InFile {
                    name,
                    text: if crlf {
                        text.replace('\n', "\r\n")
                    } else {
                        text
                    },
                    sha256: "abc".into(),
                }
            })
            .collect();
        let max = c.get("max").and_then(Value::as_u64).unwrap_or(0) as usize;
        let name = s(&c["name"]);
        let got = match build(&files, c["pol"].as_i64().unwrap() as i32, max) {
            Ok(t) => serde_json::from_str::<Value>(&table_json(&t)).unwrap(),
            Err(e) => serde_json::json!({ "error": e.key }),
        };
        same(&got, &c["expect"], name);
    }
}

#[test]
fn numbers_adducts_names_formulas_and_spectra_equal_the_js() {
    let g = golden();
    for p in g["num"].as_array().unwrap() {
        assert_eq!(num(s(&p[0])), opt(&p[1]), "num {:?}", p[0]);
    }
    for p in g["adducts"].as_array().unwrap() {
        assert_eq!(norm_adduct(s(&p[0])), s(&p[1]), "adduct {:?}", p[0]);
    }
    for p in g["names"].as_array().unwrap() {
        assert_eq!(clean_name(s(&p[0])), s(&p[1]), "name {:?}", p[0]);
    }
    for p in g["prop"].as_array().unwrap() {
        assert_eq!(
            is_propagated(s(&p[0])),
            p[1].as_bool().unwrap(),
            "propagated {:?}",
            p[0]
        );
    }
    for p in g["forms"].as_array().unwrap() {
        match (formula_mass(s(&p[0])), opt(&p[1])) {
            (Some(a), Some(b)) => assert!((a - b).abs() < 1e-6, "formula {:?}: {a} vs {b}", p[0]),
            (a, b) => assert_eq!(a, b, "formula {:?}", p[0]),
        }
    }
    for p in g["spec"].as_array().unwrap() {
        let m = parse_spectrum(s(&p[0]));
        let want = |k: &str| {
            p[1][k]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_f64)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            (m.mz, m.it),
            (want("mz"), want("it")),
            "spectrum {:?}",
            p[0]
        );
    }
}

#[test]
fn structure_checks_and_the_recognition_of_files_equal_the_js() {
    let g = golden();
    for p in g["checks"].as_array().unwrap() {
        let a = p[0].as_array().unwrap();
        let got = check_structure(
            s(&a[0]),
            opt(&a[1]),
            s(&a[2]),
            a[3].as_i64().unwrap() as i32,
            20.0,
        );
        let want = &p[1];
        assert_eq!(got.ok, want["ok"].as_bool(), "check {a:?}");
        assert_eq!(
            got.adduct.as_deref(),
            want.get("adduct").and_then(Value::as_str),
            "check {a:?}"
        );
        match (got.ppm, want.get("ppm").and_then(Value::as_f64)) {
            (Some(x), Some(y)) => assert!((x - y).abs() < 1e-3, "check {a:?}: {x} vs {y}"),
            (x, y) => assert_eq!(x, y, "check {a:?}"),
        }
    }
    for p in g["heads"].as_array().unwrap() {
        let (name, text) = if s(&p[0]).contains('/') {
            (
                "x".to_string(),
                std::fs::read_to_string(data().join(s(&p[0]))).unwrap(),
            )
        } else {
            (s(&p[0]).to_string(), String::new())
        };
        let text = match s(&p[0]) {
            "a.mgf" => "id,mz,rt\n".to_string(),
            "none" => "a,b\n1,2\n".to_string(),
            "quoted" => "\"id\",\"mz\",\"rt\"\n1,2,3\n".to_string(),
            "empty-first" => "\n\nid,mz,rt\n".to_string(),
            _ => text,
        };
        assert_eq!(detect(&name, &text), s(&p[1]), "detect {:?}", p[0]);
    }
}

#[test]
fn delimited_text_with_quotes_and_newlines_equals_the_js() {
    for c in golden()["inline"].as_array().unwrap() {
        let (mut header, mut rows) = (vec![], vec![]);
        let d = s(&c["d"]).as_bytes()[0];
        let st = each_row(
            s(&c["text"]),
            d,
            None,
            |h| {
                header = h.to_vec();
                Ok(())
            },
            |_, r, _| {
                rows.push(r.to_vec());
                Ok(())
            },
        )
        .unwrap();
        assert!(st.is_some());
        let want = |v: &Value| {
            v.as_array()
                .unwrap()
                .iter()
                .map(|x| s(x).to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(header, want(&c["header"]));
        assert_eq!(
            rows,
            c["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(want)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_byte_order_mark_does_not_hide_the_columns_and_empty_text_has_no_header() {
    let t = std::fs::read_to_string(data().join("mzmine/tabella_pos.csv")).unwrap();
    let f = |text: String| {
        vec![InFile {
            name: "t.csv".into(),
            text,
            sha256: String::new(),
        }]
    };
    let got = build(&f(format!("\u{feff}{t}")), 1, 0).unwrap();
    assert_eq!(got.features.len(), 6);
    assert!(each_row("", b',', None, |_| Ok(()), |_, _, _| Ok(()))
        .unwrap()
        .is_none());
    assert_eq!(build(&f(String::new()), 1, 0).unwrap_err().key, "noTable");
}
