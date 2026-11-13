//! Tests of `match-mz` on synthetic tables.

use crate::execute;

/// Synthetic tables: no real list, only masses of simple formulas.
const TABLES: &str = r#"{"losses":[{"id":"H2O","f":"H2O"},{"id":"NH3","f":"NH3"}],
    "adducts":[{"id":"[M+H]+","k":1,"add":"H","z":1},{"id":"[M+Na]+","k":1,"add":"Na","z":1},{"id":"[M-H]-","k":1,"sub":"H","z":-1}],
    "contam":[{"id":"peg1","mz":63.04406,"pol":1},{"id":"x","mz":18.2,"pol":-1}]}"#;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn tables_file() -> ((), String) {
    let p = std::env::temp_dir().join(format!("mzlab_match_mz_{}.json", std::process::id()));
    std::fs::write(&p, TABLES).unwrap();
    ((), p.to_str().unwrap().to_string())
}

#[test]
fn the_output_is_the_core_answer_byte_for_byte() {
    let (_d, t) = tables_file();
    let o = run(&[
        "match-mz", "18.0106", "--tables", &t, "--set", "hr=1", "--set", "mda=3",
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
        "match-mz", "63.0441", "--tables", &t, "--set", "hr=1", "--set", "ppm=10", "--set", "pol=+",
    ]);
    let s = String::from_utf8(o.stdout).unwrap();
    assert!(s.contains("peg1") && s.contains("contaminant"), "{s}");
}

#[test]
fn usage_errors_exit_2() {
    let (_d, t) = tables_file();
    for a in [
        vec!["match-mz"],
        vec!["match-mz", "18"],
        vec!["match-mz", "18", "--tables", "/nonexistent.json"],
        vec!["match-mz", "18", "--tables", &t, "--set", "ppm=abc"],
        vec!["match-mz", "18", "--tables", &t, "--set", "nope=1"],
        vec!["match-mz", "18", "--tables", &t, "--nope"],
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    assert_eq!(run(&["match-mz", "--help"]).code, 0);
}
