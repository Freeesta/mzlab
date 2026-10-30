//! `variabili` and `nome_gruppo` of `campioni::route` (`nomivar/1`, study `docs/studi/RAGGRUPPAMENTO_FILE.md` §3): the worked list of the study,
//! permutations, noise, spellings, manual variables, the limits and the speed.
use mzlab_core::campioni::route_tutto as route;
use serde_json::Value;
use std::collections::BTreeMap;

fn federico() -> Vec<String> {
    let mut v: Vec<String> = [
        "MQ_blk",
        "MQ_Wash_1",
        "MQ_Wash_2",
        "TW_blk",
        "MQ_std_BTZ_1ppm",
        "QC_Std_5ppm_mid",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for c in ["H2O2", "N2", "O2", "PMS"] {
        for t in ["T00", "T01", "T02", "T05"] {
            v.push(format!("MQ_{c}_{t}"));
        }
    }
    for c in ["AIR", "H2O2"] {
        for t in ["T00", "T01", "T02", "T05"] {
            v.push(format!("TW_{c}_{t}"));
        }
    }
    v
}

fn body(names: &[String], manual: &str) -> String {
    let f: Vec<String> = names
        .iter()
        .map(|n| format!("{{\"name\":{n:?}}}"))
        .collect();
    format!("{{\"files\":[{}]{manual}}}", f.join(","))
}

fn ask(names: &[String]) -> Value {
    serde_json::from_str(&route("variabili", &body(names, "")).unwrap()).unwrap()
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// One variable as `value label -> names`, plus the names without a value: the same for every order of the input.
fn view(r: &Value, names: &[String]) -> BTreeMap<String, String> {
    let nm = |a: &Value| -> Vec<String> {
        let mut x: Vec<String> = a
            .as_array()
            .unwrap()
            .iter()
            .map(|i| names[i.as_u64().unwrap() as usize].clone())
            .collect();
        mzlab_core::sortc::slice_ord(&mut x);
        x
    };
    let mut out = BTreeMap::new();
    for (i, v) in r["variabili"].as_array().unwrap().iter().enumerate() {
        let vals: Vec<String> = v["valori"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| format!("{}={}:{:?}", x["chiave"], x["etichetta"], nm(&x["file"])))
            .collect();
        out.insert(
            format!("{i}:{}", v["id"]),
            format!(
                "{} {} {:?} | {}",
                v["punteggio"],
                v["tipo"],
                nm(&v["senza"]),
                vals.join(" ")
            ),
        );
    }
    out
}

fn ids(r: &Value) -> Vec<String> {
    r["variabili"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["id"].as_str().unwrap().to_string())
        .collect()
}

fn counts(r: &Value, id: &str) -> Vec<(String, usize)> {
    let v = r["variabili"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == id)
        .unwrap_or_else(|| panic!("no variable {id}"));
    v["valori"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            (
                x["etichetta"].as_str().unwrap().to_string(),
                x["file"].as_array().unwrap().len(),
            )
        })
        .collect()
}

fn c(a: &[(&str, usize)]) -> Vec<(String, usize)> {
    a.iter().map(|(k, n)| (k.to_string(), *n)).collect()
}

#[test]
fn the_list_of_the_study() {
    let got = route("variabili", &body(&federico(), "")).unwrap();
    assert_eq!(
        got,
        include_str!("campioni_variabili/federico.json").trim_end()
    );
    let r: Value = serde_json::from_str(&got).unwrap();
    assert_eq!(r["schema"], "nomivar/1");
    assert_eq!(r["comune"], "");
    assert_eq!(
        ids(&r),
        ["matrix", "cond", "time", "role", "matcond", "conc", "rep"]
    );
    assert_eq!(counts(&r, "matrix"), c(&[("MQ", 20), ("TW", 9)]));
    assert_eq!(
        counts(&r, "cond"),
        c(&[("H2O2", 8), ("AIR", 4), ("N2", 4), ("O2", 4), ("PMS", 4)])
    );
    assert_eq!(
        counts(&r, "time"),
        c(&[("T00", 6), ("T01", 6), ("T02", 6), ("T05", 6)])
    );
    // the role is the one of the name parser: `QC_Std_5ppm_mid` is a standard there
    assert_eq!(
        counts(&r, "role"),
        c(&[("sample", 24), ("blank", 4), ("std", 2)])
    );
    assert_eq!(
        counts(&r, "matcond"),
        c(&[
            ("MQ\u{b7}H2O2", 4),
            ("MQ\u{b7}N2", 4),
            ("MQ\u{b7}O2", 4),
            ("MQ\u{b7}PMS", 4),
            ("TW\u{b7}AIR", 4),
            ("TW\u{b7}H2O2", 4)
        ])
    );
    assert_eq!(counts(&r, "conc"), c(&[("1 ppm", 1), ("5 ppm", 1)]));
    assert_eq!(counts(&r, "rep"), c(&[("1", 1), ("2", 1)]));
    // BTZ, mid and Wash are no conditions: the six files without a time have no condition
    let cond = &r["variabili"][1];
    assert_eq!(cond["senza"], serde_json::json!([0, 1, 2, 3, 4, 5]));
    assert_eq!(r["variabili"][0]["senza"], serde_json::json!([5]));
}

#[test]
fn every_permutation_gives_the_same_variables() {
    let names = federico();
    let base = view(&ask(&names), &names);
    let mut seed = 0x9e3779b97f4a7c15u64;
    for _ in 0..20 {
        let mut p = names.clone();
        for i in (1..p.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            p.swap(i, (seed % (i as u64 + 1)) as usize);
        }
        assert_eq!(view(&ask(&p), &p), base);
    }
}

#[test]
fn noise_adds_no_variable() {
    let mut names = federico();
    let before = ids(&ask(&names));
    names.push("MQ_H2O2_T00_final".into());
    names.push("QC_mid2".into());
    let r = ask(&names);
    assert_eq!(ids(&r), before);
    // the stray word after the time changes no condition
    assert_eq!(counts(&r, "cond")[0], ("H2O2".to_string(), 9));
}

#[test]
fn case_and_zeros_are_one_value() {
    let names = strs(&[
        "MQ_blk",
        "MQ_BLK",
        "MQ_Blank",
        "MQ_H2O2_T0",
        "MQ_H2O2_T00",
        "TW_H2O2_T0",
        "TW_H2O2_t00",
        "TW_blk",
    ]);
    let r = ask(&names);
    assert_eq!(counts(&r, "role"), c(&[("blank", 4), ("sample", 4)]));
    // one time for the four timed files, so no variable
    assert!(!ids(&r).contains(&"time".to_string()));
    let m = &r["variabili"][0];
    assert_eq!(m["id"], "matrix");
}

#[test]
fn terms_whole_token_or_inside() {
    let names = federico();
    let m = |terms: &str, sub: bool| {
        let man = format!(",\"manuale\":{{\"has\":{terms},\"substring\":{sub}}}");
        let r: Value =
            serde_json::from_str(&route("variabili", &body(&names, &man)).unwrap()).unwrap();
        r["manuale"].clone()
    };
    let whole = m("[\"O2\"]", false);
    assert_eq!(whole["id"], "has");
    assert_eq!(whole["valori"][0]["file"].as_array().unwrap().len(), 4);
    assert_eq!(whole["multipli"], serde_json::json!([]));
    let inside = m("[\"O2\"]", true);
    assert_eq!(inside["valori"][0]["file"].as_array().unwrap().len(), 12);
    let two = m("[\"O2\",\"H2O2\"]", true);
    // the eight H2O2 files hold both terms; each goes to the first term
    assert_eq!(two["multipli"].as_array().unwrap().len(), 8);
    assert_eq!(two["valori"][0]["file"].as_array().unwrap().len(), 12);
    assert_eq!(two["senza"].as_array().unwrap().len(), 18);
}

#[test]
fn a_position_with_chosen_separators() {
    let names = federico();
    let pos = |n: i64, sep: &str| {
        let man = format!(",\"manuale\":{{\"pos\":{n},\"sep\":{sep:?}}}");
        let r: Value =
            serde_json::from_str(&route("variabili", &body(&names, &man)).unwrap()).unwrap();
        r["manuale"].clone()
    };
    let first = pos(1, "_");
    assert_eq!(first["id"], "pos:1");
    assert_eq!(first["valori"][0]["etichetta"], "MQ");
    assert_eq!(first["valori"][0]["file"].as_array().unwrap().len(), 20);
    let last = pos(-1, "_-. ");
    assert_eq!(last["id"], "pos:-1");
    assert_eq!(last["valori"][0]["etichetta"], "T00");
    // the second token with an underscore only: `MQ_blk` and `TW_blk` share `blk`
    let second = pos(2, "_");
    assert_eq!(second["valori"][0]["file"].as_array().unwrap().len(), 8);
}

#[test]
fn names_without_separators_read_like_the_separated_ones() {
    let sep = federico();
    let glued: Vec<String> = sep[6..].iter().map(|n| n.replace('_', "")).collect();
    let a = ask(&sep[6..]);
    let b = ask(&glued);
    for id in ["matrix", "cond", "time", "matcond"] {
        assert_eq!(counts(&a, id), counts(&b, id), "{id}");
    }
    let tokens: Vec<&str> = b["files"][0]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["testo"].as_str().unwrap())
        .collect();
    assert_eq!(tokens, ["MQ", "H2O2", "T00"]);
    // a chemical piece is never cut
    let chem = ask(&strs(&["Na2SO4_T00", "Na2SO4_T05", "TiO2_T00", "TiO2_T05"]));
    assert_eq!(chem["files"][0]["tokens"][0]["testo"], "Na2SO4");
    assert_eq!(chem["files"][2]["tokens"][0]["testo"], "TiO2");
}

#[test]
fn folders_are_a_variable_when_they_differ() {
    let names = strs(&[
        "PMS/MQ_x_T00.mzML",
        "PMS/MQ_x_T05.mzML",
        "N2/MQ_x_T00.mzML",
        "N2/MQ_x_T05.mzML",
    ]);
    let r = ask(&names);
    assert_eq!(counts(&r, "folder"), c(&[("N2", 2), ("PMS", 2)]));
    assert_eq!(r["files"][0]["tokens"][0]["classe"], "folder");
    assert_eq!(r["files"][0]["tokens"][0]["testo"], "PMS");
    let same = ask(&strs(&["PMS/a_T00.mzML", "PMS/a_T05.mzML"]));
    assert!(!ids(&same).contains(&"folder".to_string()));
    assert_eq!(same["files"][0]["tokens"][0]["testo"], "a");
}

#[test]
fn degenerate_inputs() {
    for names in [
        vec![],
        strs(&["MQ_H2O2_T00"]),
        strs(&["same", "same", "same"]),
        strs(&["", "x"]),
    ] {
        let r = ask(&names);
        assert!(r["variabili"].as_array().unwrap().is_empty() || names.len() > 1);
        assert_eq!(r["files"].as_array().unwrap().len(), names.len());
    }
    assert_eq!(ask(&strs(&["same", "same"]))["comune"], "same");
    assert_eq!(ask(&strs(&["MQ_a_T00", "MQ_b_T00"]))["comune"], "MQ*T00");
    assert_eq!(ask(&strs(&["MQ_a_T00", "MQ_b_T05"]))["comune"], "MQ*");
}

#[test]
fn nome_gruppo_cases() {
    let g = |names: &[&str]| {
        let v: Vec<String> = strs(names);
        let b = format!("{{\"names\":{}}}", serde_json::to_string(&v).unwrap());
        let r: Value = serde_json::from_str(&route("nome_gruppo", &b).unwrap()).unwrap();
        r["nome"].as_str().unwrap().to_string()
    };
    assert_eq!(g(&["MQ_H2O2_T00", "MQ_H2O2_T01"]), "MQ_H2O2");
    assert_eq!(
        g(&["MQ_H2O2_T00", "MQ_H2O2_T01", "MQ_H2O2_T02.mzML"]),
        "MQ_H2O2"
    );
    assert_eq!(g(&["MQ_blk", "TW_N2_T00"]), "");
    assert_eq!(g(&["MQ-H2O2-T00", "MQ_H2O2_T01"]), "MQ_H2O2");
    assert_eq!(g(&["MQ-H2O2-T00", "MQ-H2O2-T01"]), "MQ-H2O2");
    assert_eq!(g(&["MQH2O2T00", "MQH2O2T01"]), "MQ_H2O2");
    assert_eq!(g(&[]), "");
    assert_eq!(g(&["only_one"]), "only_one");
}

#[test]
fn errors() {
    assert!(route("nope", "{}").is_err());
    assert!(route("variabili", "{").is_err());
    let over = body(&vec!["a".to_string(); 5001], "");
    let e = route("variabili", &over).unwrap_err();
    assert_eq!(
        (e.key, e.params[0].1.as_str()),
        ("error.badCampioni", "5000")
    );
    let long = body(&["x".repeat(257)], "");
    let e = route("variabili", &long).unwrap_err();
    assert_eq!(
        (e.key, e.params[0].1.as_str()),
        ("error.badCampioni", "256")
    );
    let e = route("nome_gruppo", "{\"names\":[1]}").unwrap_err();
    assert_eq!((e.key, e.params[0].0), ("error.badCampioni", "op"));
    let man = body(&federico(), ",\"manuale\":{\"pos\":0}");
    assert!(route("variabili", &man).is_err());
}

#[test]
fn ten_thousand_names_stay_linear() {
    // the route takes 5 000 names: the 10 000 are two calls
    let mut names = Vec::new();
    for m in ["MQ", "TW", "WW", "DW"] {
        for cnd in ["H2O2", "N2", "O2", "PMS", "AIR", "C"] {
            for t in 0..40 {
                for r in 1..=11 {
                    names.push(format!("{m}_{cnd}_T{t:02}_{r}.mzML"));
                }
            }
        }
    }
    names.truncate(10_000);
    let (a, b) = (body(&names[..5000], ""), body(&names[5000..], ""));
    let t0 = std::time::Instant::now();
    let (ra, rb) = (
        route("variabili", &a).unwrap(),
        route("variabili", &b).unwrap(),
    );
    let ms = t0.elapsed().as_millis();
    assert!(ra.contains("\"id\":\"matrix\"") && rb.contains("\"id\":\"time\""));
    // about 0.2 s in release, over half of it in the name parser of `proponi`; the bound only catches a quadratic step
    let bound = if cfg!(debug_assertions) { 10_000 } else { 1000 };
    assert!(ms < bound, "{ms} ms");
}
