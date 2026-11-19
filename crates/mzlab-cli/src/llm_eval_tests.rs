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
