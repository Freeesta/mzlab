//! The name model of `campioni/3`: template and regex, the spans, a model that does not match, the free columns, the 30 000 generated names.

use super::many;
use mzlab_core::campioni::{self, tabella_da_json, Cell};
use serde_json::{json, Value};

fn run(model: &str, names: &[&str]) -> Value {
    let files: Vec<Value> = names
        .iter()
        .map(|n| json!({"name": n, "sha256": ""}))
        .collect();
    serde_json::from_str(
        &campioni::route_tutto(
            "modello",
            &json!({"modello": model, "files": files}).to_string(),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn a_template_gives_the_fields_and_where_they_are() {
    let name = "plates/S12_neg_30min_r3.mzML";
    let out = run("{campione_id}_{polarita}_{tempo}min_r{replica}", &[name]);
    let c = &out["campioni"]["campioni"][0];
    assert_eq!(c["campione_id"], "S12");
    assert_eq!(c["polarita"], -1);
    assert_eq!(
        (
            c["tempo"].clone(),
            c["tempo_min"].clone(),
            c["replica"].clone()
        ),
        (30.into(), 30.into(), 3.into())
    );
    assert_eq!(c["tempo_confermato"], false);
    let chars: Vec<char> = name.chars().collect();
    for p in out["posizioni"][0].as_array().unwrap() {
        let (a, b) = (
            p["inizio"].as_u64().unwrap() as usize,
            p["fine"].as_u64().unwrap() as usize,
        );
        let got: String = chars[a..b].iter().collect();
        let want = match p["campo"].as_str().unwrap() {
            "campione_id" => "S12",
            "polarita" => "neg",
            "tempo" => "30",
            "replica" => "3",
            f => panic!("{f}"),
        };
        assert_eq!(got, want);
    }
    assert_eq!(out["posizioni"][0].as_array().unwrap().len(), 4);
}

#[test]
fn a_regex_with_named_groups_and_free_columns() {
    let names = ["P7_POS_t5.raw", "P7_NEG_t5.raw", "other.raw"];
    let out = run(
        r"(?P<campione_id>[A-Z]\d+)_(?P<polarita>POS|NEG)_t(?P<tempo>\d+)(?:\.raw)?",
        &names,
    );
    let cs = &out["campioni"]["campioni"];
    assert_eq!(
        (cs[0]["polarita"].clone(), cs[1]["polarita"].clone()),
        (1.into(), (-1).into())
    );
    assert_eq!(out["senza_corrispondenza"], json!([2]));
    assert_eq!(out["posizioni"][2], json!([]));
    // not matched: left as proponi gave it
    assert_eq!(cs[2]["campione_id"], "");
    let out = run(
        r"(?P<campione_id>\w+)_(?P<lotto>b\d)_(?P<operatore>[a-z]+)",
        &["S1_b2_ana"],
    );
    let t = tabella_da_json(out["campioni"].to_string().as_bytes()).unwrap();
    assert_eq!(t.campioni[0].batch, "b2");
    assert_eq!(
        t.campioni[0].extra_of("operatore"),
        Some(&Cell::Testo("ana".into()))
    );
    // lazy and alternatives, anchors, classes, repeats
    let out = run(
        r"(?i)^(?P<serie>.+?)_(?P<replica>\d{1,2})$",
        &["Run_A_B_12", "x_123"],
    );
    assert_eq!(out["campioni"]["campioni"][0]["serie"], "Run_A_B");
    assert_eq!(out["senza_corrispondenza"], json!([1]));
}

#[test]
fn bad_models_and_runaway_models_are_safe() {
    for m in [
        "(?P<a>x",
        "(?P<a>x)(?P<a>y)",
        "(?P<g>x)[a-",
        "(?P<g>x)|*a",
        "(?P<g>x)a{3,2}",
        "(?P<>x)",
        "(?P<a>x))",
        "(?P<a>\\",
    ] {
        let e = campioni::route_tutto("modello", &json!({"modello": m, "files": []}).to_string())
            .unwrap_err();
        assert_eq!(e.key, "error.badModello", "{m}");
    }
    // nested repeats that explode in a naive matcher end in "no match", quickly
    let long = "a".repeat(200) + "b";
    let out = run(r"(?P<campione_id>(a*)*)c", &[&long]);
    assert_eq!(out["senza_corrispondenza"], json!([0]));
    // a name longer than the limit is just not matched
    let huge = "a".repeat(5000);
    assert_eq!(
        run("{campione_id}", &[&huge])["senza_corrispondenza"],
        json!([0])
    );
    let _ = many;
}

#[test]
fn the_generated_names_never_panic_and_models_keep_the_rest() {
    // the table of `project_parity` (35 802 names) plus names the models read
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
    let mut names = Vec::new();
    for h in heads {
        for c in cores {
            for t in tails {
                for e in ["", ".mzML", ".MZML", ".raw", ".d", ".wiff"] {
                    names.push(format!("{h}{c}{t}{e}"));
                }
            }
        }
    }
    assert!(names.len() > 30_000);
    for i in 0..300 {
        names.push(format!("A{i}_pos_30min_r2.mzML"));
        names.push(format!("K{i}_neg_5min_r1.raw"));
        names.push(format!("Q{i}_30min"));
    }
    let many_names = names;
    let refs: Vec<&str> = many_names.iter().map(String::as_str).collect();
    for model in [
        "{campione_id}_{polarita}_{tempo}min_r{replica}",
        "{campione_id}_{tempo}{unita_tempo}",
        r"(?i)(?P<campione_id>[a-z]+\d*)[-_ ]?(?P<polarita>pos|neg)?.*",
        r"(?P<serie>.*?)(?P<tempo>\d+)(?P<unita>mgL|ppm)?",
    ] {
        let out = run(model, &refs);
        let cs = &out["campioni"]["campioni"];
        assert_eq!(cs.as_array().unwrap().len(), refs.len());
        let un = out["senza_corrispondenza"].as_array().unwrap().len();
        assert!(un < refs.len(), "{model}");
        // the spans stay inside the name
        for (n, ps) in refs.iter().zip(out["posizioni"].as_array().unwrap()) {
            let len = n.chars().count() as u64;
            assert!(ps
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["inizio"].as_u64() <= p["fine"].as_u64()
                    && p["fine"].as_u64().unwrap() <= len));
        }
    }
}
