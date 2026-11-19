//! Tests of `ion match` on synthetic tables.

use crate::execute;

/// Synthetic tables: no real list, only masses of simple formulas.
const TABLES: &str = r#"{"losses":[{"id":"H2O","f":"H2O"},{"id":"NH3","f":"NH3"}],
    "adducts":[{"id":"[M+H]+","k":1,"add":"H","z":1},{"id":"[M+Na]+","k":1,"add":"Na","z":1},{"id":"[M-H]-","k":1,"sub":"H","z":-1}],
    "contam":[{"id":"peg1","mz":63.04406,"pol":1},{"id":"x","mz":18.2,"pol":-1}]}"#;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

/// One file per call: the tests run in parallel, and a shared file could be read while another test rewrites it.
fn tables_file() -> ((), String) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("mzlab_match_mz_{}_{n}.json", std::process::id()));
    std::fs::write(&p, TABLES).unwrap();
    ((), p.to_str().unwrap().to_string())
}

#[test]
fn the_output_is_the_core_answer_byte_for_byte() {
    let (_d, t) = tables_file();
    let o = run(&[
        "ion", "match", "18.0106", "--tables", &t, "--set", "hr=1", "--set", "mda=3",
    ]);
    assert_eq!(o.code, 0);
    let req = format!(
        r#"{{"x":18.0106,"hr":true,"tol_mda":3,"tol_ppm":5,"pol":"",{}"#,
        TABLES.trim().strip_prefix("{").unwrap()
    );
    let want = mzlab_core::search::match_number(&req).unwrap();
    assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
}

#[test]
fn a_contaminant_is_a_hit_with_its_error_and_never_a_verdict() {
    let (_d, t) = tables_file();
    let o = run(&[
        "ion", "match", "63.0441", "--tables", &t, "--set", "hr=1", "--set", "ppm=10", "--set",
        "pol=+",
    ]);
    let s = String::from_utf8(o.stdout).unwrap();
    assert!(s.contains("peg1") && s.contains("contaminant"), "{s}");
}

#[test]
fn the_contaminant_window_takes_da_or_ppm_and_is_reported() {
    let (_d, t) = tables_file();
    // 63.2 is 0.156 Da from peg1: inside 0.2 Da, outside 0.1 Da, at unit resolution
    let hit = |tol: &str| {
        let o = run(&[
            "ion",
            "match",
            "63.2",
            "--tables",
            &t,
            "--set",
            "unit=da",
            "--set",
            &format!("tol={tol}"),
        ]);
        assert_eq!(o.code, 0);
        String::from_utf8(o.stdout).unwrap()
    };
    assert!(hit("0.2").contains("peg1") && !hit("0.1").contains("peg1"));
    let s = hit("0.2");
    assert!(
        s.contains(r#""contam":{"#) && s.contains(r#""unit":"da""#),
        "{s}"
    );
    // same bytes as the core answer
    let req = format!(
        r#"{{"x":63.2,"hr":false,"tol_mda":3,"tol_ppm":5,"pol":"","tol_unit":"da","tol":0.2,{}"#,
        TABLES.trim().strip_prefix("{").unwrap()
    );
    assert_eq!(
        s,
        format!("{}\n", mzlab_core::search::match_number(&req).unwrap())
    );
    assert_eq!(
        run(&["ion", "match", "63.2", "--tables", &t, "--set", "unit=da", "--set", "tol=0"]).code,
        2
    );
}

#[test]
fn usage_errors_exit_2() {
    let (_d, t) = tables_file();
    for a in [
        vec!["ion", "match"],
        vec!["ion", "match", "18"],
        vec!["ion", "match", "18", "--tables", "/nonexistent.json"],
        vec!["ion", "match", "18", "--tables", &t, "--set", "ppm=abc"],
        vec!["ion", "match", "18", "--tables", &t, "--set", "nope=1"],
        vec!["ion", "match", "18", "--tables", &t, "--nope"],
    ] {
        assert_eq!(run(&a).code, crate::exit::usage_or_missing(&a), "{a:?}");
    }
    assert_eq!(run(&["ion", "match", "--help"]).code, 0);
}
