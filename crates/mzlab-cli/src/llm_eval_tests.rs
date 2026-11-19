//! `mzlab-cli llm eval` on the cases of `tests/data/llm/eval` and `llm context` on a small run folder.

use std::path::PathBuf;

use crate::execute;

fn data(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/llm")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn run(args: &[&str]) -> crate::Outcome {
    execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_ten_cases_pass_on_their_recordings() {
    let o = run(&["llm", "eval", &data("eval"), "--replay", &data("")]);
    let t = String::from_utf8(o.stdout).unwrap();
    assert_eq!(o.code, 0, "{t}");
    let v: serde_json::Value = serde_json::from_str(&t).unwrap();
    assert!(t.starts_with("{\"schema\":\"evalrun/1\""), "{t}");
    assert_eq!(v["percent"], 100);
    assert_eq!(v["cases"].as_array().unwrap().len(), 10);
    assert!(v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["failed"].as_array().unwrap().is_empty()));
}

#[test]
fn a_missing_recording_or_a_provider_is_an_error() {
    let dir = std::env::temp_dir().join(format!("mzlab-eval-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let o = run(&[
        "llm",
        "eval",
        &data("eval"),
        "--replay",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(o.code, 4);
    let o = run(&[
        "llm",
        "eval",
        &data("eval"),
        "--provider",
        "anthropic",
        "--key-file",
        "k",
    ]);
    assert!(
        o.code == 2 && o.stderr.contains("err.llm.network"),
        "{}",
        o.stderr
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn context_reads_a_run_folder_and_can_hide_names() {
    let dir = std::env::temp_dir().join(format!("mzlab-ctx-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("report.json"),
        "{\"inputs\":[{\"name\":\"secret_sample.mzML\"}],\"steps\":[]}",
    )
    .unwrap();
    std::fs::write(
        dir.join("features.csv"),
        "id,mz,area\nf1,100.5,10\nf2,200.25,20\n",
    )
    .unwrap();
    let d = dir.to_str().unwrap();
    let a = run(&["llm", "context", d]);
    assert_eq!(a.code, 0, "{}", a.stderr);
    let t = String::from_utf8(a.stdout.clone()).unwrap();
    assert!(
        t.contains("secret_sample.mzML") && t.contains("\"rows\":2"),
        "{t}"
    );
    assert_eq!(run(&["llm", "context", d]).stdout, a.stdout);
    let h = String::from_utf8(run(&["llm", "context", d, "--hide-names"]).stdout).unwrap();
    assert!(!h.contains("secret_sample") && h.contains("\"f1\""));
    assert_eq!(run(&["llm", "context", d, "--budget", "5"]).code, 2);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn every_llm_failure_is_one_error_document_with_its_code() {
    let dir = std::env::temp_dir().join(format!("mzlab-llmfail-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let d = dir.to_str().unwrap();
    for (a, code) in [
        (vec!["llm", "eval", "/nonexistent/cases", "--replay", d], 4),
        (
            vec!["llm", "eval", &data("eval"), "--replay", "/nonexistent/rec"],
            4,
        ),
        (vec!["llm", "eval", &data("eval"), "--replay", d], 4),
        (vec!["llm", "context", "/nonexistent/run"], 4),
        (
            vec![
                "llm",
                "replay",
                "/nonexistent/r.jsonl",
                "--conv",
                "/nonexistent/c.json",
            ],
            4,
        ),
        (vec!["llm", "eval", "--bogus"], 2),
    ] {
        let mut j = a.clone();
        j.push("--json");
        let o = run(&j);
        assert_eq!(o.code, code, "{a:?}: {}", o.stderr);
        let t = String::from_utf8(o.stdout).unwrap();
        assert!(
            t.starts_with("{\"schema\":\"error/1\",\"code\":"),
            "{a:?}: {t}"
        );
        assert!(t.ends_with('\n'));
    }
    std::fs::remove_dir_all(dir).ok();
}

/// A failing case is a verdict: exit 1 and the `evalrun/1` stays on stdout, not an `error/1`.
#[test]
fn a_failed_case_keeps_its_document_at_exit_1() {
    let dir = std::env::temp_dir().join(format!("mzlab-evalfail-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cases = dir.join("cases");
    let rec = dir.join("rec");
    std::fs::create_dir_all(&cases).unwrap();
    std::fs::create_dir_all(&rec).unwrap();
    // a case against the recording of another one: its checks fail
    let src = PathBuf::from(data("eval"));
    let mut ids: Vec<_> = std::fs::read_dir(&src)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    ids.sort();
    let (first, second) = (&ids[0], &ids[1]);
    std::fs::copy(first, cases.join(first.file_name().unwrap())).unwrap();
    let id = first.file_stem().unwrap().to_str().unwrap();
    let other = PathBuf::from(data("")).join(format!(
        "{}.jsonl",
        second.file_stem().unwrap().to_str().unwrap()
    ));
    std::fs::copy(other, rec.join(format!("{id}.jsonl"))).unwrap();
    let o = run(&[
        "llm",
        "eval",
        cases.to_str().unwrap(),
        "--replay",
        rec.to_str().unwrap(),
        "--json",
    ]);
    let t = String::from_utf8(o.stdout.clone()).unwrap();
    assert_eq!(o.code, 1, "{t}");
    assert!(t.starts_with("{\"schema\":\"evalrun/1\""), "{t}");
    assert!(t.contains("\"failed\":[\"") && t.ends_with('\n'), "{t}");
    std::fs::remove_dir_all(dir).ok();
}

/// What goes wrong before a case is scored is one `error/1` (never an `evalrun/1`): a case or recording of the wrong format is integrity
/// (3), a recording that runs out before the case is done is data (1), a missing file is input (4).
#[test]
fn eval_errors_before_scoring_take_the_codes_of_the_contract() {
    let dir = std::env::temp_dir().join(format!("mzlab-evalcodes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (cases, rec) = (dir.join("cases"), dir.join("rec"));
    std::fs::create_dir_all(&cases).unwrap();
    std::fs::create_dir_all(&rec).unwrap();
    let first = PathBuf::from(data("eval")).join("E1.json");
    std::fs::copy(&first, cases.join("E1.json")).unwrap();
    let eval = |case_dir: &std::path::Path| {
        let o = run(&[
            "llm",
            "eval",
            case_dir.to_str().unwrap(),
            "--replay",
            rec.to_str().unwrap(),
            "--json",
        ]);
        (o.code, String::from_utf8(o.stdout).unwrap())
    };
    // no recording of the case: input
    assert_eq!(eval(&cases).0, 4);
    // a recording that is not one: integrity
    std::fs::write(rec.join("E1.jsonl"), "not json\n").unwrap();
    let (code, t) = eval(&cases);
    assert_eq!(code, 3, "{t}");
    assert!(t.contains("\"error_key\":\"err.llm.replay.format\""), "{t}");
    // a recording with no turn left: data, an error document and not a verdict
    std::fs::write(rec.join("E1.jsonl"), "").unwrap();
    let (code, t) = eval(&cases);
    assert_eq!(code, 1, "{t}");
    assert!(
        t.starts_with("{\"schema\":\"error/1\"") && t.ends_with('\n'),
        "{t}"
    );
    // a case that is not one: integrity
    let bad = dir.join("bad");
    std::fs::create_dir_all(&bad).unwrap();
    std::fs::write(bad.join("X.json"), "{}").unwrap();
    let (code, t) = eval(&bad);
    assert_eq!(code, 3, "{t}");
    assert!(t.contains("\"error_key\":\"err.llm.eval.case\""), "{t}");
    std::fs::remove_dir_all(dir).ok();
}
