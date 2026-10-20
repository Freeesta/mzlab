//! Plans of the identification steps run on a synthetic fixture: a three-compound library and two query spectra, all made here (no lab data).

use std::path::PathBuf;

/// A folder of the test with the library `lib.msp` and the queries `q.mgf`.
pub(crate) fn fixture(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_ident_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let rec = |name: &str, prec: f64, peaks: &[(f64, f64)]| {
        let mut s = format!("Name: {name}\nPrecursorMZ: {prec}\nPrecursor_type: [M+H]+\nIon_mode: Positive\nFormula: C10H10O\nNum Peaks: {}\n", peaks.len());
        for (m, i) in peaks {
            s += &format!("{m} {i}\n");
        }
        s + "\n"
    };
    let a = [
        (100.05, 400.0),
        (120.08, 1000.0),
        (150.10, 700.0),
        (180.12, 300.0),
        (210.15, 500.0),
    ];
    let b = [
        (90.03, 800.0),
        (130.06, 1000.0),
        (160.09, 300.0),
        (190.11, 600.0),
        (230.14, 200.0),
    ];
    let c: Vec<(f64, f64)> = [
        (100.05, 300.0),
        (120.08, 600.0),
        (200.12, 1000.0),
        (230.15, 900.0),
        (260.18, 400.0),
    ]
    .to_vec();
    let lib = rec("Alpha", 300.1234, &a) + &rec("Beta", 300.1240, &b) + &rec("Gamma", 400.1234, &c);
    std::fs::write(dir.join("lib.msp"), lib).unwrap();
    // q0: Alpha with noise in m/z and intensity; q1: Alpha's fragments, shifted by +100 Da (the precursor difference) for the last three
    let q0 = [
        (100.051, 380.0),
        (120.079, 950.0),
        (150.102, 720.0),
        (180.121, 310.0),
        (210.149, 480.0),
    ];
    let q1 = [
        (100.051, 380.0),
        (120.079, 950.0),
        (250.102, 720.0),
        (280.121, 310.0),
        (310.149, 480.0),
    ];
    let mgf = |t: &str, prec: f64, p: &[(f64, f64)]| {
        let mut s = format!("BEGIN IONS\nTITLE={t}\nPEPMASS={prec}\nCHARGE=1+\nIONMODE=Positive\n");
        for (m, i) in p {
            s += &format!("{m} {i}\n");
        }
        s + "END IONS\n"
    };
    std::fs::write(
        dir.join("q.mgf"),
        mgf("q0", 300.1235, &q0) + &mgf("q1", 400.1234, &q1),
    )
    .unwrap();
    dir
}

fn plan(algorithm: &str, params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"q.mgf","type":"spectra"}},{{"name":"lib.msp","type":"library"}}],"steps":[
        {{"id":"m","step":"spectra.match@1","in":{{"query":"file:q.mgf","library":"file:lib.msp"}},"algorithm":"{algorithm}","params":{params}}}]}}"#
    )
}

fn hits(dir: &std::path::Path) -> mzlab_core::json::Value {
    let t = std::fs::read(dir.join("work/m.json")).unwrap();
    mzlab_core::json::parse(&t).unwrap()
}

fn run(dir: &std::path::Path, algorithm: &str, params: &str) -> String {
    let (run, prov) = crate::flow::run_plan(
        plan(algorithm, params).as_bytes(),
        dir,
        &dir.join("work"),
        1,
        None,
    )
    .unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    prov
}

/// The three algorithms find Alpha for the first query; the record carries the candidate fields and the `prov/1` record names the step.
#[test]
fn match_finds_the_library_spectrum_with_each_algorithm() {
    let dir = fixture("match");
    for alg in ["spectral_entropy", "cosine", "modified_cosine"] {
        let prov = run(&dir, alg, "{}");
        assert!(prov.contains("spectra.match"), "{prov}");
        let h = hits(&dir);
        let list = h.get("hits").unwrap().arr();
        let first = list
            .iter()
            .find(|x| x.get("query").and_then(|q| q.str()) == Some("q0"))
            .unwrap();
        assert_eq!(first.get("name").unwrap().str(), Some("Alpha"), "{alg}");
        assert!(first.get("score").unwrap().f64().unwrap() > 0.9, "{alg}");
        assert!(first.get("shared").unwrap().f64().unwrap() >= 4.0);
        assert!(first.get("explained").unwrap().arr().len() >= 4);
        assert_eq!(first.get("algorithm").unwrap().str(), Some(alg));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The modified cosine reaches the analogue (the second query is Alpha shifted by 100 Da); the plain cosine finds nothing there.
#[test]
fn modified_cosine_finds_the_analogue_and_cosine_does_not() {
    let dir = fixture("analog");
    run(
        &dir,
        "modified_cosine",
        r#"{"analog_window_da":150,"analog_min_shift_da":1}"#,
    );
    let h = hits(&dir);
    let a = h
        .get("hits")
        .unwrap()
        .arr()
        .iter()
        .find(|x| {
            x.get("query").and_then(|q| q.str()) == Some("q1")
                && x.get("name").and_then(|n| n.str()) == Some("Alpha")
        })
        .expect("analogue");
    assert!((a.get("precursor_delta").unwrap().f64().unwrap().abs() - 100.0).abs() < 0.01);
    run(&dir, "cosine", "{}");
    let h = hits(&dir);
    assert!(h
        .get("hits")
        .unwrap()
        .arr()
        .iter()
        .all(|x| x.get("name").and_then(|n| n.str()) != Some("Alpha")
            || x.get("query").and_then(|q| q.str()) != Some("q1")));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Same plan, same bytes; `top_n` caps the candidates of a query; a stricter `min_score` drops them.
#[test]
fn match_is_deterministic_and_obeys_its_parameters() {
    let dir = fixture("det");
    run(&dir, "spectral_entropy", "{}");
    let one = std::fs::read(dir.join("work/m.json")).unwrap();
    run(&dir, "spectral_entropy", "{}");
    assert_eq!(one, std::fs::read(dir.join("work/m.json")).unwrap());
    run(&dir, "cosine", r#"{"top_n":1,"precursor_ppm":50}"#);
    let n = hits(&dir)
        .get("hits")
        .unwrap()
        .arr()
        .iter()
        .filter(|x| x.get("query").and_then(|q| q.str()) == Some("q0"))
        .count();
    assert_eq!(n, 1);
    run(&dir, "spectral_entropy", r#"{"min_score":0.999999}"#);
    assert!(hits(&dir).get("hits").unwrap().arr().len() <= 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// An mzML of caffeine: one ion, [M+H]+ 195.08765 with its 13C and 2x13C peaks, a Gaussian peak at 3.0 min, and a flat noise floor.
pub(crate) fn caffeine_mzml() -> Vec<u8> {
    use mzlab_core::mzml::Spectrum;
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "caffeine".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 120;
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let a = 3.0e6 * (-0.5 * ((rt - 3.0) / 0.07f64).powi(2)).exp();
        let mut peaks = vec![
            (195.08765, a as f32),
            (196.09100, (a * 0.101) as f32),
            (197.09435, (a * 0.0065) as f32),
            (150.0 + (i % 7) as f64, 2e3),
        ];
        peaks.retain(|p| p.1 > 100.0);
        peaks.sort_by(|x, y| x.0.total_cmp(&y.0));
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: "+ c ESI Full ms [100.000-400.000]".into(),
            precursor: None,
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// A MS2 spectrum of caffeine: the fragments of C8H10N4O2 and one peak no sub-formula explains.
pub(crate) fn caffeine_mgf() -> String {
    "BEGIN IONS\nTITLE=caf\nPEPMASS=195.08765\nCHARGE=1+\nIONMODE=Positive\n138.06619 1000\n110.07127 600\n83.06037 150\n123.05524 90\n163.5 400\nEND IONS\n".into()
}

fn formula_plan(ms2_alg: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"caf.mzML","type":"mzml"}},{{"name":"caf.mgf","type":"spectra"}}],"steps":[
        {{"id":"t","step":"features.table@1","in":{{"source":["file:caf.mzML"]}},"params":{{"polarity":1}}}},
        {{"id":"f1","step":"formula.ms1@1","in":{{"features":"t.report","index":["file:caf.mzML"]}}}},
        {{"id":"f2","step":"formula.ms2@1","in":{{"spectra":"file:caf.mgf","formulas":"f1.formulas"}},"algorithm":"{ms2_alg}"}}]}}"#
    )
}

/// features.table -> formula.ms1 -> formula.ms2 on a synthetic caffeine file: the formula is among the candidates of the feature and explains the fragments.
#[test]
fn formula_chain_finds_the_formula_and_explains_the_fragments() {
    let dir = fixture("formula");
    std::fs::write(dir.join("caf.mzML"), caffeine_mzml()).unwrap();
    std::fs::write(dir.join("caf.mgf"), caffeine_mgf()).unwrap();
    for alg in ["subformula", "h_rearrangement"] {
        let (run, prov) = crate::flow::run_plan(
            formula_plan(alg).as_bytes(),
            &dir,
            &dir.join("work"),
            1,
            None,
        )
        .unwrap();
        assert!(
            run.skipped.is_empty() && !prov.contains("\"error\""),
            "{prov}"
        );
        let f1 =
            mzlab_core::json::parse(&std::fs::read(dir.join("work/f1.json")).unwrap()).unwrap();
        let feats = f1.get("features").unwrap().arr();
        assert_eq!(feats.len(), 1, "one feature");
        let cands = feats[0]
            .get("result")
            .unwrap()
            .get("candidates")
            .unwrap()
            .arr();
        assert!(
            cands
                .iter()
                .any(|c| c.get("neutral").and_then(|n| n.str()) == Some("C8H10N4O2")),
            "{cands:?}"
        );
        let f2 =
            mzlab_core::json::parse(&std::fs::read(dir.join("work/f2.json")).unwrap()).unwrap();
        let sp = &f2.get("spectra").unwrap().arr()[0];
        let c = sp
            .get("candidates")
            .unwrap()
            .arr()
            .iter()
            .find(|c| c.get("formula").and_then(|n| n.str()) == Some("C8H10N4O2"))
            .expect("candidate explains the precursor");
        let frag = |m: f64| {
            c.get("fragments")
                .unwrap()
                .arr()
                .iter()
                .find(|f| (f.get("mz").unwrap().f64().unwrap() - m).abs() < 1e-3)
                .and_then(|f| f.get("formula").and_then(|x| x.str()).map(String::from))
        };
        assert_eq!(frag(138.06619).as_deref(), Some("C6H8N3O"), "{alg}");
        assert_eq!(frag(110.07127).as_deref(), Some("C5H8N3"), "{alg}");
        assert!(
            frag(163.5).is_none(),
            "{alg}: the odd peak stays unexplained"
        );
        assert_eq!(c.get("adduct").unwrap().str(), Some("[M+H]+"));
    }
    let _ = std::fs::remove_dir_all(&dir);
}
