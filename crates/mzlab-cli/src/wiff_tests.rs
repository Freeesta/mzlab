//! `mzlab-cli wiff` on a pair written by `tools/wiff_riferimento.py` (skipped without python3): the JSON, the exit codes and the usage errors.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures() -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!("mzlab_cli_wiff_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_cmd: python3 not available");
    }
    ok.map(|_| out)
}

fn run(args: &[&str]) -> crate::Outcome {
    crate::execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn list_gives_the_schema_and_exit_0_for_a_clean_file() {
    let Some(d) = fixtures() else { return };
    let f = d.join("two.wiff");
    let o = run(&["wiff", "list", f.to_str().unwrap(), "--json"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["schema"], "wiff.list/1");
    assert_eq!(v["files"][0]["samples"].as_array().unwrap().len(), 2);
}

#[test]
fn list_exits_3_when_a_sample_is_refused() {
    let Some(d) = fixtures() else { return };
    let f = d.join("periods.wiff");
    let o = run(&["wiff", "list", f.to_str().unwrap()]);
    assert_eq!(o.code, 3, "{}", o.stderr);
    assert!(String::from_utf8_lossy(&o.stdout).contains("\"refused\""));
}

#[test]
fn verify_exits_0_for_equal_and_1_for_different() {
    let Some(d) = fixtures() else { return };
    let (a, b) = (d.join("two.wiff"), d.join("one.wiff"));
    let (a, b) = (a.to_str().unwrap(), b.to_str().unwrap());
    assert_eq!(run(&["wiff", "verify", a, "--against", a]).code, 0);
    assert_eq!(run(&["wiff", "verify", b, "--against", a]).code, 1);
}

#[test]
fn usage_errors_exit_2() {
    assert_eq!(run(&["wiff"]).code, 2);
    assert_eq!(run(&["wiff", "list"]).code, 2);
    assert_eq!(run(&["wiff", "verify", "a.wiff"]).code, 2);
    assert_eq!(run(&["wiff", "list", "/nonexistent/x.wiff"]).code, 2);
}
