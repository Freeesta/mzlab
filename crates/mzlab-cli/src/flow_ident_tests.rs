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
