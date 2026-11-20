//! The panic hook of the real binary: a panic ends as exit 7 with one `error/1` on stdout under `--json`, never a bare abort.

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
        .args(args)
        .env("MZLAB_TEST_PANIC", "1")
        .output()
        .unwrap()
}

#[test]
fn a_panicking_process_exits_7_with_an_error_document() {
    if !cfg!(debug_assertions) {
        return; // the test route exists in debug builds only
    }
    let o = run(&["file", "info", "x.mzML", "--json"]);
    assert_eq!(o.status.code(), Some(7));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["schema"], "error/1");
    assert_eq!(v["code"], 7);
    assert_eq!(v["error_key"], "err.run.internal");
    assert!(v["params"]["detail"]
        .as_str()
        .unwrap()
        .contains("panic on purpose"));
    assert!(String::from_utf8_lossy(&o.stderr).starts_with("error: mzlab-cli stopped"));
    let plain = run(&["features", "x.mzML"]);
    assert_eq!(
        (plain.status.code(), plain.stdout.is_empty()),
        (Some(7), true)
    );
}

#[test]
fn version_json_works_in_any_position() {
    for args in [["--json", "--version"], ["--version", "--json"]] {
        let o = Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(0));
        let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        assert!(v.get("schema").is_some(), "{args:?}");
    }
}
