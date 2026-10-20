//! `annot.level`: the native rule gives the levels of `tests/livelli_casi.json` (the file `tests/test_livelli_schymanski.py` runs through `livelli.js`),
//! and the chain convert -> features -> match -> formula -> level runs on a synthetic caffeine file. Synthetic data only.

use mzlab_core::json::{self, Value};

use crate::flow_ident_tests::{caffeine_mgf, caffeine_mzml, fixture};
use crate::flow_level::{assess, Analog, Cand, Evidence};

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
        let a = assess(&evidence(c.get("input").unwrap()), profile).unwrap();
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
    assert_eq!(run, cases.len() - 1);
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
