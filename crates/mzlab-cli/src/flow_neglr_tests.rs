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
    {"id":"fil","step":"table.filter@1","in":{"table":"gap.table","samples":"file:samples.json"},"params":{"dup_ppm":10}},
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
        let prov = run(&d, &with(EXTRA));
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
        // the unit axis of the detection reaches the alignment and the gap filling (a negative tolerance is Da)
        let ali = activity(&prov, "ali").get("params").unwrap();
        let ppm = ali.get("ppm").and_then(|v| v.f64()).unwrap();
        assert_eq!(ppm < 0.0, !hr, "{tag} align ppm {ppm}");
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

/// Adds to every mzML of `d` five MS2 scans of the hydroxylated product (precursor [M-H]-, 255.0444) around its peak.
fn add_ms2(d: &Path, pol: i8) {
    let prec = 256.0517 + f64::from(pol) * 1.007276;
    let (acc, sign) = if pol < 0 {
        ("MS:1000129", "-")
    } else {
        ("MS:1000130", "+")
    };
    let frag = [80.0, 119.0, 148.0, 191.0, 211.0];
    for n in NAMES {
        let f = d.join(format!("{n}.mzML"));
        let mut x = std::fs::read_to_string(&f).unwrap();
        let mut ms2 = String::new();
        for i in 400..405 {
            let rt = 4.32 + (i - 399) as f64 * 0.003;
            let y: Vec<f64> = frag
                .iter()
                .enumerate()
                .map(|(k, _)| 1e4 / (k + 1) as f64)
                .collect();
            ms2.push_str(&format!(
                r#"<spectrum index="{i}" id="scan={}" defaultArrayLength="5"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="2"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="{acc}" name="scan"/><cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="ITMS {sign} c ESI d Full ms2 {prec:.2}@cid35.00 [50.00-300.00]"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><precursorList count="1"><precursor><isolationWindow><cvParam cvRef="MS" accession="MS:1000827" name="isolation window target m/z" value="{prec:.4}"/><cvParam cvRef="MS" accession="MS:1000828" name="isolation window lower offset" value="0.5"/><cvParam cvRef="MS" accession="MS:1000829" name="isolation window upper offset" value="0.5"/></isolationWindow><selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{prec:.4}"/></selectedIon></selectedIonList><activation><cvParam cvRef="MS" accession="MS:1000133" name="collision-induced dissociation"/></activation></precursor></precursorList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
                i + 1,
                crate::flow_bentazone_tests::array("MS:1000514", &frag),
                crate::flow_bentazone_tests::array("MS:1000515", &y)
            ));
        }
        // in time order: the scans go between the MS1 scans 216 and 217
        x = x.replace(
            r#"<spectrum index="217""#,
            &format!("{ms2}<spectrum index=\"217\""),
        );
        x = x.replace(r#"spectrumList count="400""#, r#"spectrumList count="405""#);
        std::fs::write(&f, x).unwrap();
    }
}

#[test]
fn a_negative_table_is_searched_as_negative_by_table_match() {
    let d = fixture("nl_g3", -1, true);
    add_ms2(&d, -1);
    run(&d, &with("]}"));
    let table = std::fs::read(d.join("work/gap.mzfeat")).unwrap();
    let idx: Vec<PathBuf> = NAMES
        .iter()
        .map(|n| d.join(format!("work/idx-{n}.mzidx")))
        .collect();
    assert_eq!(crate::flow_export::polarity_of(&table, &idx).unwrap(), -1);
    // the bytes the browser calls (`rows_mgf` of the table .wasm) with the indexes of the files
    let samples = std::fs::read(d.join("samples.json")).unwrap();
    let open = |k: usize| match idx.get(k) {
        Some(p) => crate::flow_base::open(p),
        None => Err(mzlab_core::error::Error::new("err.flow.tablematch.files")),
    };
    let mgf = mzlab_feat_table_wasm::rows_mgf::rows_mgf(
        &mut mzlab_feat::store::Slice(&table),
        table.len() as u64,
        &samples,
        &[],
        &mzlab_core::budget::sha256_hex(&table),
        open,
    )
    .unwrap();
    assert!(mgf.contains("ACCESSION="), "{mgf}");
    assert!(mgf.contains("IONMODE=Negative"), "{mgf}");
    assert!(!mgf.contains("IONMODE=Positive"), "{mgf}");
    // a library of the same spectrum in both polarities: only the negative entry is found, in `flow run` too
    let entry = mgf
        .split("BEGIN IONS")
        .nth(1)
        .unwrap()
        .split("END IONS")
        .next()
        .unwrap();
    let body: String = entry
        .lines()
        .filter(|l| {
            !l.starts_with("ACCESSION=")
                && !l.starts_with("FEATURE_ID=")
                && !l.starts_with("SCANS=")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let lib = |name: &str, mode: &str| {
        format!(
            "BEGIN IONS\nNAME={name}\n{}\nEND IONS\n",
            body.replace("IONMODE=Negative", &format!("IONMODE={mode}"))
                .replace(
                    "CHARGE=1-",
                    if mode == "Negative" {
                        "CHARGE=1-"
                    } else {
                        "CHARGE=1+"
                    }
                )
        )
    };
    std::fs::write(
        d.join("lib.mgf"),
        format!("{}{}", lib("PosRef", "Positive"), lib("NegRef", "Negative")),
    )
    .unwrap();
    let tail = r#",{"id":"tm","step":"table.match@1","in":{"table":"gap.table","samples":"file:samples.json","index":["idx.index"],"library":"file:lib.mgf"},"params":{"tol_da":0.02,"precursor_ppm":10,"top_n":50,"min_shared":1,"min_score":0.5}}]}"#;
    let text = with(tail).replacen(
        r#""inputs":["#,
        r#""inputs":[{"name":"lib.mgf","type":"library"},"#,
        1,
    );
    let prov = run(&d, &text);
    assert_eq!(error_of(&prov, "tm"), None);
    let hits = read(&d, "tm.json");
    assert!(hits.contains("NegRef"), "{hits}");
    assert!(!hits.contains("PosRef"), "{hits}");
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

#[test]
fn a_sample_naming_a_file_the_table_lacks_is_refused_with_its_name() {
    let tail = r#",{"id":"csv","step":"export.table_csv@1","in":{"table":"gap.table","samples":"file:samples.json"}}]}"#;
    let d = fixture("nl_ghost", 1, true);
    let files: Vec<_> = NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| {
            (
                if i == 0 {
                    "ghost.mzML".to_string()
                } else {
                    format!("{n}.mzML")
                },
                String::new(),
                String::new(),
            )
        })
        .collect();
    let text = mzlab_core::campioni::to_json(&mzlab_core::campioni::proponi(&files));
    std::fs::write(d.join("samples.json"), text).unwrap();
    let prov = run(&d, &with(tail));
    assert_eq!(error_of(&prov, "csv").as_deref(), Some("error.badCampioni"));
    let params = activity(&prov, "csv")
        .get("error")
        .and_then(|e| e.get("params"))
        .map(|p| format!("{p:?}"));
    assert!(
        params.as_deref().is_some_and(|p| p.contains(NAMES[0])),
        "{params:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
