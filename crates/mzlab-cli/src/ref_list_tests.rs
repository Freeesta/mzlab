//! Tests of `ref-list` on synthetic CSV files.

use crate::execute;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn file(csv: &str) -> String {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("mzlab_ref_list_{}_{n}.csv", std::process::id()));
    std::fs::write(&p, csv).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn the_output_is_the_core_answer_byte_for_byte() {
    let csv = "mz;name\n100.1;A\n";
    let o = run(&["db", "ref-list", &file(csv), "--name", "L"]);
    assert_eq!(o.code, 0);
    let body = r#"{"csv":"mz;name\n100.1;A\n","name":"L"}"#;
    let want = mzlab_core::reflist::route(body).unwrap();
    assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
}

#[test]
fn row_errors_exit_one_and_keep_the_good_rows() {
    let o = run(&["db", "ref-list", &file("mz,name\n100.1,A\nbad,B\n")]);
    assert_eq!(o.code, 1);
    let t = String::from_utf8(o.stdout).unwrap();
    assert!(
        t.contains("\"line\":3") && t.contains("err.reflist.mz") && t.contains("\"name\":\"A\""),
        "{t}"
    );
}

#[test]
fn match_finds_the_number_in_the_list() {
    let o = run(&[
        "db",
        "ref-list",
        &file("mz,name\n100.1,A\n"),
        "--match",
        "100.1005",
        "--set",
        "unit=ppm",
        "--set",
        "tol=10",
    ]);
    assert_eq!(o.code, 0);
    let t = String::from_utf8(o.stdout).unwrap();
    assert!(
        t.contains("\"match\":") && t.contains("\"id\":\"A\""),
        "{t}"
    );
}

#[test]
fn usage_and_unreadable_file() {
    assert_eq!(run(&["db", "ref-list"]).code, 2);
    assert_eq!(run(&["db", "ref-list", "/nonexistent/x.csv"]).code, 4);
}
