//! `annot.level`: the native rule gives the levels of `tests/livelli_casi.json` (the file `tests/test_livelli_schymanski.py` runs through `livelli.js`),
//! and the chain convert -> features -> match -> formula -> level runs on a synthetic caffeine file. Synthetic data only.

use mzlab_core::json::{self, Value};

use crate::flow_ident_tests::{caffeine_mgf, caffeine_mzml, fixture};
use crate::flow_level::{assess_with, Analog, Cand, Evidence, Std, Suspect};

fn num(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::f64)
}

fn evidence(input: &Value) -> Evidence {
    let cands = input.get("cands").map(Value::arr).unwrap_or(&[]);
    let analogs = input.get("analogs").map(Value::arr).unwrap_or(&[]);
    Evidence {
        cands: cands
            .iter()
            .map(|c| Cand {
                name: c.get("name").and_then(Value::str).unwrap_or("").into(),
                ik: c.get("ik").and_then(Value::str).unwrap_or("").into(),
                ent: num(c, "ent"),
                cos: num(c, "cos"),
                shared: num(c, "shared"),
                dprec: num(c, "dprec"),
                nlib: num(c, "nlib"),
            })
            .collect(),
        analogs: analogs
            .iter()
            .map(|a| Analog {
                name: a.get("name").and_then(Value::str).unwrap_or("").into(),
                mcos: num(a, "mcos").unwrap_or(0.0),
                delta: num(a, "delta").unwrap_or(0.0),
            })
            .collect(),
        n_scans: num(input, "nScans").unwrap_or(1.0),
        formula_unique: input.get("formulaUnique").and_then(Value::bool) == Some(true),
        std: input
            .get("std")
            .filter(|s| s.f64().is_none() && !matches!(s, Value::Null))
            .map(|s| Std {
                name: s.get("name").and_then(Value::str).unwrap_or("").into(),
                drt: num(s, "drt"),
                ent: num(s, "ent"),
                cos: num(s, "cos"),
                dprec: num(s, "dprec"),
                ..Std::default()
            }),
        suspect: input.get("suspect").map(|s| Suspect {
            name: s.get("name").and_then(Value::str).unwrap_or("").into(),
            matched: num(s, "matched").unwrap_or(0.0) as usize,
            ..Suspect::default()
        }),
    }
}

#[test]
fn the_native_rule_gives_the_levels_of_the_shared_cases() {
    let doc = json::parse(include_bytes!("../../../tests/livelli_casi.json")).unwrap();
    let cases = doc.get("cases").unwrap().arr();
    assert!(cases.len() >= 10);
    let mut run = 0;
    for c in cases {
        if c.get("native").and_then(Value::bool) == Some(false) {
            continue;
        }
        let name = c.get("name").and_then(Value::str).unwrap();
        let profile = c.get("profile").and_then(Value::str).unwrap();
        let over: Vec<(&str, f64)> = match c.get("thresholds") {
            Some(Value::Obj(o)) => o
                .iter()
                .filter_map(|(k, v)| v.f64().map(|v| (k.as_str(), v)))
                .collect(),
            _ => Vec::new(),
        };
        let a = assess_with(&evidence(c.get("input").unwrap()), profile, &over).unwrap();
        assert_eq!(
            (a.level.as_str(), a.why),
            (
                c.get("level").and_then(Value::str).unwrap(),
                c.get("why").and_then(Value::str).unwrap()
            ),
            "{name}"
        );
        // the program never proposes 2b: only the chemist's decision gives it
        assert_ne!(a.level, "2b");
        run += 1;
    }
    assert_eq!(run, cases.len());
}

/// A copy of the caffeine mzML with numpress arrays, the one open format `convert.mzml` takes without a vendor file.
fn numpress_caffeine() -> Vec<u8> {
    use mzlab_core::mzmlwrite::{Header, MzmlWriter, Opzioni};
    let mut sample = mzlab_core::mzml::read(&caffeine_mzml()[..]).unwrap();
    let mut w = MzmlWriter::con_opzioni(
        Vec::new(),
        &Header::default(),
        Opzioni {
            numpress: true,
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(sample.spectra.len()).unwrap();
    for s in &mut sample.spectra {
        s.mz.extend([310.0, 320.0, 330.0]);
        s.intensity.extend([2e3, 2e3, 2e3]);
        s.wide = None;
        w.spectrum(s).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// The caffeine entry of a library: the peaks of the query spectrum, so that the hit passes every gate of the cascade.
const LIBRARY: &str = "Name: Caffeine\nPrecursorMZ: 195.08765\nPrecursor_type: [M+H]+\nIon_mode: Positive\nFormula: C8H10N4O2\nInChIKey: RYYVLZVUVIJVGH-UHFFFAOYSA-N\nNum Peaks: 5\n138.06619 1000\n110.07127 600\n83.06037 150\n123.05524 90\n163.5 400\n\n";

const CHAIN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"caf.mzML","type":"raw"},{"name":"caf.mgf","type":"spectra"},{"name":"lib.msp","type":"library"}],"steps":[
    {"id":"c","step":"convert.mzml@1","in":{"raw":"file:caf.mzML"}},
    {"id":"t","step":"features.table@1","in":{"source":["c.mzml"]},"params":{"polarity":1}},
    {"id":"m","step":"spectra.match@1","in":{"query":"file:caf.mgf","library":"file:lib.msp"},"algorithm":"spectral_entropy"},
    {"id":"f1","step":"formula.ms1@1","in":{"features":"t.report","index":["c.mzml"]}},
    {"id":"f2","step":"formula.ms2@1","in":{"spectra":"file:caf.mgf","formulas":"f1.formulas"}},
    {"id":"l","step":"annot.level@1","in":{"candidates":"m.hits","formulas":"f2.formulas"}}]}"#;

fn rows(dir: &std::path::Path) -> Vec<(String, String, String)> {
    let doc = json::parse(&std::fs::read(dir.join("work/l.json")).unwrap()).unwrap();
    doc.get("rows")
        .unwrap()
        .arr()
        .iter()
        .map(|r| {
            let s = |k: &str| r.get(k).and_then(Value::str).unwrap_or("").to_string();
            (s("id"), s("level"), s("why"))
        })
        .collect()
}

#[test]
fn the_chain_reaches_a_level() {
    let dir = fixture("level");
    std::fs::write(dir.join("caf.mzML"), numpress_caffeine()).unwrap();
    std::fs::write(dir.join("lib.msp"), LIBRARY).unwrap();
    // one spectrum: a library match with a single scan needs confirmation (3a); two replicate spectra of the same entry confirm it (2a)
    let one = caffeine_mgf();
    std::fs::write(dir.join("caf.mgf"), &one).unwrap();
    let run = |dir: &std::path::Path| {
        let (run, prov) =
            crate::flow::run_plan(CHAIN.as_bytes(), dir, &dir.join("work"), 1, None).unwrap();
        assert!(
            run.skipped.is_empty() && !prov.contains("\"error\""),
            "{prov}"
        );
        prov
    };
    let prov = run(&dir);
    assert!(prov.contains("annot.level@1"), "{prov}");
    assert_eq!(rows(&dir), [("q0".into(), "3a".into(), "confirm".into())]);
    let first = std::fs::read(dir.join("work/l.json")).unwrap();
    run(&dir);
    assert_eq!(first, std::fs::read(dir.join("work/l.json")).unwrap());
    let twice = one.clone() + &one;
    std::fs::write(dir.join("caf.mgf"), twice).unwrap();
    run(&dir);
    let both = rows(&dir);
    assert_eq!(both.len(), 2);
    assert!(both.iter().all(|r| r.1 == "2a" && r.2 == "2a"), "{both:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Query 7 is caffeine at 1.0 min (SPECTRUMID, as `spectra.ms2` writes it with a feature table); feature 9 has a `screen/1` suspect with two fragments found, feature 8 with one.
const STANDARDS_PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"q.mgf","type":"spectra"},{"name":"lib.msp","type":"library"},{"name":"std.csv","type":"library"},{"name":"screen.json","type":"candidates"},{"name":"f.json","type":"formulas"}],"steps":[
    {"id":"m","step":"spectra.match@1","in":{"query":"file:q.mgf","library":"file:lib.msp"},"algorithm":"spectral_entropy"},
    {"id":"l","step":"annot.level@1","in":{"candidates":["m.hits","file:screen.json"],"formulas":"file:f.json","standards":"file:std.csv","spectra":"file:q.mgf"}__PARAMS__}]}"#;

fn suspect_screen() -> String {
    let cand = |name: &str, n: usize| {
        let mz: Vec<String> = (0..n).map(|k| format!("{}.0", 100 + k)).collect();
        format!(
            r#"{{"skeleton":"SSSSSSSSSSSSSS","inchikey":"SSSSSSSSSSSSSS-UHFFFAOYSA-N","name":"{name}","smiles":"","neutral_mass":200.0,"rt_diff_min":null,"fragments_listed":3,"fragments_matched":{n},"fragment_mz":[{}],"files":[0],"memberships":[]}}"#,
            mz.join(",")
        )
    };
    let hit = |f: usize, c: String| {
        format!(
            r#"{{"feature":{f},"mz":201.0,"adduct":"[M+H]+","delta_ppm":0.1,"formula":"C9H9N","level":5,"candidates":[{c}]}}"#
        )
    };
    format!(
        r#"{{"format":"screen/1","polarity":1,"hits":[{},{}]}}"#,
        hit(8, cand("One fragment", 1)),
        hit(9, cand("Two fragments", 2))
    )
}

/// The files of the plan above in a fresh folder.
fn standards_inputs(tag: &str) -> std::path::PathBuf {
    let dir = fixture(tag);
    let mgf = caffeine_mgf().replace("TITLE=caf\n", "TITLE=caf\nSPECTRUMID=7\nRTINSECONDS=60\n");
    std::fs::write(dir.join("q.mgf"), mgf).unwrap();
    std::fs::write(dir.join("lib.msp"), LIBRARY).unwrap();
    std::fs::write(
        dir.join("std.csv"),
        "name,inchikey,rt\nCaffeine,RYYVLZVUVIJVGH-UHFFFAOYSA-N,1.15\n",
    )
    .unwrap();
    std::fs::write(dir.join("screen.json"), suspect_screen()).unwrap();
    std::fs::write(
        dir.join("f.json"),
        r#"{"format":"formulas/1","features":[]}"#,
    )
    .unwrap();
    dir
}

fn standards_rows(params: &str) -> Vec<(String, String, String)> {
    let dir = standards_inputs("level_std");
    let plan = STANDARDS_PLAN.replace("__PARAMS__", params);
    let (run, prov) =
        crate::flow::run_plan(plan.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let out = rows(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    out
}

#[test]
fn a_standard_gives_level_1_and_suspect_fragments_give_3a() {
    let by_id = |rows: &[(String, String, String)], id: &str| {
        rows.iter()
            .find(|r| r.0 == id)
            .map(|r| (r.1.clone(), r.2.clone()))
    };
    // the Flow defaults: 0.15 min from the standard is inside 0.2
    let rows = standards_rows("");
    assert_eq!(by_id(&rows, "7"), Some(("1".into(), "standard".into())));
    assert_eq!(
        by_id(&rows, "9"),
        Some(("3a".into(), "suspect_fragments".into()))
    );
    assert_eq!(by_id(&rows, "8"), Some(("5".into(), "empty".into())));
    // the profile thresholds (0.1 min, 0.8) as parameters: not Level 1, the library hit of one scan needs confirmation
    let rows = standards_rows(r#","params":{"std_rt_max":0.1,"std_score_min":0.8}"#);
    assert_eq!(by_id(&rows, "7"), Some(("3a".into(), "confirm".into())));
    // a third fragment needed: the suspect with two no longer counts
    let rows = standards_rows(r#","params":{"suspect_fragments_min":3}"#);
    assert_eq!(by_id(&rows, "9"), Some(("5".into(), "empty".into())));
}

#[test]
fn the_cli_sets_the_profile_thresholds_back() {
    let dir = standards_inputs("level_std_cli");
    let plan = STANDARDS_PLAN.replace("__PARAMS__", "");
    std::fs::write(dir.join("plan.json"), plan).unwrap();
    let run = |sets: &[&str], out: &str| {
        let (out, plan) = (dir.join(out), dir.join("plan.json"));
        let mut args = vec![
            crate::flow_run_tests::p(&plan),
            "--in",
            crate::flow_run_tests::p(&dir),
            "--out",
            crate::flow_run_tests::p(&out),
        ];
        args.extend(sets.iter().flat_map(|s| ["--set", *s]));
        let o = crate::flow_run_tests::flow_run(&args);
        assert_eq!(o.code, 0, "{}", o.stderr);
        let doc = json::parse(&std::fs::read(out.join("l.json")).unwrap()).unwrap();
        doc.get("rows")
            .unwrap()
            .arr()
            .iter()
            .find(|r| r.get("id").and_then(Value::str) == Some("7"))
            .and_then(|r| r.get("level").and_then(Value::str).map(String::from))
            .unwrap()
    };
    assert_eq!(run(&[], "out_default"), "1");
    assert_eq!(
        run(&["l.std_rt_max=0.1", "l.std_score_min=0.8"], "out_profile"),
        "3a"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
