//! The steps of mzFlow in both polarities and both resolutions (`docs/studi/MATRICE_NEG_LR.md`) on the invented bentazone series of
//! `flow_bentazone_tests.rs`: what runs in HR+, HR-, LR+ and LR- and follows the files. The `gap_*` tests are ignored: each states what a gap of the
//! matrix should do and fails today; the chat that closes the gap removes its `#[ignore]`.

use std::path::{Path, PathBuf};

use crate::flow_bentazone_tests::{fixture, mzml, plan, NAMES};

const CASES: [(&str, i8, bool); 4] = [
    ("hrp", 1, true),
    ("hrn", -1, true),
    ("lrp", 1, false),
    ("lrn", -1, false),
];

/// The table steps after the chain of `plan`: every one reads only what a full-scan series gives.
const EXTRA: &str = r#",{"id":"est","step":"features.estimate@1","in":{"index":["idx.index"]}},
    {"id":"dig","step":"tp.dig_lr@1","in":{"table":"gap.table","samples":"file:samples.json","parent":"file:parent.json","index":["idx.index"]}},
    {"id":"fil","step":"table.filter@1","in":{"table":"gap.table","samples":"file:samples.json"},"params":{DUP}},
    {"id":"plz","step":"table.plausible@1","in":{"table":"gap.table"}},
    {"id":"kmd","step":"table.kmd@1","in":{"table":"gap.table"}},
    {"id":"csv","step":"export.table_csv@1","in":{"table":"gap.table","samples":"file:samples.json"}},
    {"id":"mzt","step":"export.mztab@1","in":{"table":"gap.table","samples":"file:samples.json"}},
    {"id":"qc","step":"qc.mass_accuracy@1","in":{"raw":["idx.index"]}},
    {"id":"rx","step":"structure.reactivity@1","in":{"parent":"file:parent.json"}}]}"#;

/// The plan of `plan` (base chain and Tabui) with `tail` (a JSON list of more steps, starting with a comma) before its end.
fn with(tail: &str) -> String {
    let base = plan(&|_| String::new(), false);
    format!("{}{tail}", base.strip_suffix("]}").unwrap())
}

/// The record of instance `id` in `prov/1`.
fn activity<'a>(prov: &'a mzlab_core::json::Value, id: &str) -> &'a mzlab_core::json::Value {
    prov.get("activity")
        .and_then(|a| a.get(id))
        .unwrap_or_else(|| panic!("no {id} in prov"))
}

/// Runs `text` in `d` and gives `prov/1` (instances in error included: the caller reads them).
fn run(d: &Path, text: &str) -> mzlab_core::json::Value {
    let (_, prov) = crate::flow::run_plan(text.as_bytes(), d, &d.join("work"), 2, None)
        .unwrap_or_else(|e| panic!("{} {:?}", e.key, e.params));
    mzlab_core::json::parse(prov.as_bytes()).unwrap()
}

fn error_of(prov: &mzlab_core::json::Value, id: &str) -> Option<String> {
    activity(prov, id)
        .get("error")
        .and_then(|e| e.get("error_key"))
        .and_then(|k| k.str())
        .map(str::to_string)
}

fn read(d: &Path, f: &str) -> String {
    std::fs::read_to_string(d.join("work").join(f)).unwrap()
}

#[test]
fn every_table_step_runs_in_both_polarities_and_both_resolutions() {
    for (tag, pol, hr) in CASES {
        let d = fixture(&format!("nl_{tag}"), pol, hr);
        // the duplicate window is in Da on the unit axis and in ppm at high resolution
        let dup = if hr {
            r#""dup_ppm":10"#
        } else {
            r#""dup_da":0.3"#
        };
        let prov = run(&d, &with(&EXTRA.replace("DUP", dup)));
        for id in [
            "det", "ali", "gap", "gen", "lnk", "flt", "pos", "net", "est", "dig", "fil", "plz",
            "kmd", "csv", "mzt", "qc", "rx",
        ] {
            assert_eq!(error_of(&prov, id), None, "{tag} {id}");
        }
        // the files decide the polarity of the hypotheses, of the link and of the Dig, and the unit mass of the hypotheses
        let gen = activity(&prov, "gen").get("params").unwrap();
        assert_eq!(
            gen.get("polarity").and_then(|v| v.f64()),
            Some(f64::from(pol)),
            "{tag}"
        );
        assert_eq!(
            gen.get("nominal").and_then(|v| v.bool()),
            Some(!hr),
            "{tag}"
        );
        let lnk = activity(&prov, "lnk").get("params").unwrap();
        assert_eq!(
            lnk.get("polarity").and_then(|v| v.f64()),
            Some(f64::from(pol)),
            "{tag}"
        );
        let adduct = if pol < 0 { "[M-H]-" } else { "[M+H]+" };
        assert!(
            read(&d, "dig.json").contains(&format!("\"adduct\":\"{adduct}\"")),
            "{tag}"
        );
        // the unit axis of the detection reaches the alignment: its tolerance is `tol_da` (Da), the ppm one is for high resolution
        let ali = activity(&prov, "ali").get("params").unwrap();
        let tol_da = ali.get("tol_da").and_then(|v| v.f64());
        assert_eq!(tol_da.is_some(), !hr, "{tag} align tol_da {tol_da:?}");
        // the exports name the polarity of the files
        let word = if pol < 0 {
            "negative scan"
        } else {
            "positive scan"
        };
        assert!(read(&d, "mzt.mzTab").contains(word), "{tag}");
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
fn the_estimate_measures_a_negative_file_once_the_polarity_is_written() {
    let d = fixture("nl_est", -1, true);
    let tail = r#",{"id":"est","step":"features.estimate@1","in":{"index":["idx.index"]},"params":{"polarity":-1}}]}"#;
    let prov = run(&d, &with(tail));
    assert_eq!(error_of(&prov, "est"), None);
    let est = read(&d, "est-BEN_T00.json");
    assert!(!est.contains("\"noise\":null"), "{est}");
    let _ = std::fs::remove_dir_all(&d);
}

/// The plan of `features.table` and `formula.ms1` over the files of `d`, `ion` as written (or the default).
fn formula_plan(ion: Option<&str>) -> String {
    let src: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#""file:{n}.mzML""#))
        .collect();
    let raw = NAMES
        .iter()
        .map(|n| format!(r#"{{"name":"{n}.mzML","type":"raw"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let params = ion.map_or(String::new(), |i| format!(r#","params":{{"ion":"{i}"}}"#));
    format!(
        r#"{{"format":"mzflow/2","inputs":[{raw}],"steps":[
        {{"id":"t","step":"features.table@1","in":{{"source":[{s}]}}}},
        {{"id":"f1","step":"formula.ms1@1","in":{{"features":"t.report","index":[{s}]}}{params}}}]}}"#,
        s = src.join(",")
    )
}

#[test]
fn formula_ms1_reads_a_negative_file_with_a_negative_ion() {
    let d = fixture("nl_f1", -1, true);
    let prov = run(&d, &formula_plan(Some("[M-H]-")));
    assert_eq!(error_of(&prov, "f1"), None);
    let f1 = read(&d, "f1.json");
    assert!(
        !f1.contains("err.flow.formula.scan"),
        "{}",
        &f1[..f1.len().min(600)]
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_inclusion_list_takes_a_negative_adduct() {
    let d = fixture("nl_inc", -1, true);
    std::fs::write(d.join("tp.json"), r#"[{"formula":"C10H12N2O4S","rt":4.3}]"#).unwrap();
    let text = r#"{"format":"mzflow/2","inputs":[{"name":"tp.json","type":"tp"}],"steps":[
        {"id":"inc","step":"export.inclusion@1","in":{"tp":"file:tp.json"},"params":{"adducts":"[M-H]-"}}]}"#;
    let prov = run(&d, text);
    assert_eq!(error_of(&prov, "inc"), None);
    // C10H12N2O4S 256.0518 - proton
    assert!(
        read(&d, "inc.csv").contains("\n255.0445,1,"),
        "{}",
        read(&d, "inc.csv")
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// The folder of a series whose file `k` is written with `of(k)` = (polarity, high resolution).
fn mixed(tag: &str, of: impl Fn(usize) -> (i8, bool)) -> PathBuf {
    let d = fixture(tag, 1, true);
    for (k, n) in NAMES.iter().enumerate() {
        let (pol, hr) = of(k);
        std::fs::write(d.join(format!("{n}.mzML")), mzml(k, pol, hr)).unwrap();
    }
    d
}

/// Whether instance `id` stopped or warned: an error, or a warning key that contains `word`.
fn flagged(prov: &mzlab_core::json::Value, id: &str, word: &str) -> bool {
    let a = activity(prov, id);
    a.get("error").is_some()
        || a.get("warnings").is_some_and(|w| {
            w.arr()
                .iter()
                .any(|k| k.str().is_some_and(|s| s.contains(word)))
        })
}

#[test]
#[ignore = "gap NL-1: features.estimate applies the default polarity +1 and axis ppm instead of the files' (MATRICE_NEG_LR.md)"]
fn gap_the_estimate_follows_the_polarity_and_the_resolution_of_the_files() {
    for (tag, pol, hr) in [("nl_g1n", -1i8, true), ("nl_g1l", 1, false)] {
        let d = fixture(tag, pol, hr);
        let tail = r#",{"id":"est","step":"features.estimate@1","in":{"index":["idx.index"]}}]}"#;
        run(&d, &with(tail));
        let est = read(&d, "est-BEN_T00.json");
        assert!(!est.contains("\"noise\":null"), "{tag}: {est}");
        assert!(est.contains(&format!("\"polarity\":{pol}")), "{tag}: {est}");
        if !hr {
            assert!(est.contains("\"axis\":\"Da\""), "{tag}: {est}");
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
#[ignore = "gap NL-2: formula.ms1 keeps [M+H]+ on negative files and finds no scan (MATRICE_NEG_LR.md)"]
fn gap_formula_ms1_takes_the_ion_of_the_polarity_of_the_files() {
    let d = fixture("nl_g2", -1, true);
    run(&d, &formula_plan(None));
    let f1 = read(&d, "f1.json");
    assert!(
        !f1.contains("err.flow.formula.scan"),
        "{}",
        &f1[..f1.len().min(600)]
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore = "gap NL-3: table.match writes the MGF of a negative table as positive when the plan does not write the polarity (MATRICE_NEG_LR.md)"]
fn gap_the_polarity_of_the_mgf_of_table_match_is_the_one_of_the_files() {
    let d = fixture("nl_g3", -1, true);
    run(&d, &with("]}"));
    let table = std::fs::read(d.join("work/gap.mzfeat")).unwrap();
    let idx: Vec<PathBuf> = NAMES
        .iter()
        .map(|n| d.join(format!("work/idx-{n}.mzidx")))
        .collect();
    assert_eq!(crate::flow_export::polarity_of(&table, &idx).unwrap(), -1);
    assert_eq!(crate::flow_export::polarity(&table).unwrap(), -1);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore = "gap NL-4: a series that mixes polarities is aligned in silence (MATRICE_NEG_LR.md)"]
fn gap_a_series_of_mixed_polarity_is_refused_or_warned() {
    let d = mixed("nl_g4", |k| (if k < 2 { -1 } else { 1 }, true));
    let prov = run(&d, &with("]}"));
    assert!(
        flagged(&prov, "ali", "polarity"),
        "the alignment of + and - files says nothing"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore = "gap NL-5: a series that mixes unit and high resolution is aligned on the axis of its first file in silence (MATRICE_NEG_LR.md)"]
fn gap_a_series_of_mixed_resolution_is_refused_or_warned() {
    let d = mixed("nl_g5", |k| (1, k >= 2));
    let prov = run(&d, &with("]}"));
    assert!(
        flagged(&prov, "ali", "resolution"),
        "the alignment of Da and ppm files says nothing"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore = "gap NL-6: the tp.* steps declare resolution hr, so a plan whose files say lr is refused although they run on it (MATRICE_NEG_LR.md)"]
fn gap_a_unit_resolution_plan_that_says_so_passes_the_check() {
    let d = fixture("nl_g6", -1, false);
    let text = with("]}").replace(
        r#""type":"raw"}"#,
        r#""type":"raw","facets":{"resolution":["lr"],"polarity":[-1]}}"#,
    );
    if let Err(e) = crate::flow::run_plan(text.as_bytes(), &d, &d.join("work"), 2, None) {
        panic!("{} {:?}", e.key, e.params);
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// The cells of a CSV of `export.table_csv` by (row id, column name).
fn cells(csv: &str) -> std::collections::BTreeMap<(String, String), String> {
    let mut lines = csv.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    let mut out = std::collections::BTreeMap::new();
    for l in lines {
        let v: Vec<&str> = l.split(',').collect();
        for (c, h) in head.iter().enumerate() {
            out.insert(
                (v[0].to_string(), h.to_string()),
                v.get(c).unwrap_or(&"").to_string(),
            );
        }
    }
    out
}

#[test]
#[ignore = "gap NL-8: the samples file is read by position, so a list in another order than the files of the table moves the values under other files (MATRICE_NEG_LR.md)"]
fn gap_the_samples_are_matched_to_the_files_of_the_table_by_name() {
    let tail = r#",{"id":"csv","step":"export.table_csv@1","in":{"table":"gap.table","samples":"file:samples.json"}}]}"#;
    let mut got = Vec::new();
    for (tag, reverse) in [("nl_ord", false), ("nl_rev", true)] {
        let d = fixture(tag, 1, true);
        if reverse {
            let files: Vec<_> = NAMES
                .iter()
                .rev()
                .map(|n| (format!("{n}.mzML"), String::new(), String::new()))
                .collect();
            let text = mzlab_core::campioni::to_json(&mzlab_core::campioni::proponi(&files));
            std::fs::write(d.join("samples.json"), text).unwrap();
        }
        let prov = run(&d, &with(tail));
        assert_eq!(error_of(&prov, "csv"), None, "{tag}");
        got.push(cells(&read(&d, "csv.csv")));
        let _ = std::fs::remove_dir_all(&d);
    }
    assert_eq!(got[0], got[1]);
}
