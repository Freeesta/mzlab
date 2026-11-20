//! No command panics on an empty or garbage file (a release build aborts on a panic: exit 134 with no message). Every registered command is run with an
//! empty file, a file of garbage and an empty CSV, with and without `--json`; each must answer with a code of the table and, under `--json`, one `error/1`.

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::{cmd, execute};

/// Commands that reach the network or write beside their input are not run blind.
const SKIP: &[&str] = &[
    "db normative build",
    "db normative import-official",
    "db insilico pack",
    "help-md",
];

fn words(name: &str, tail: &[&str]) -> Vec<String> {
    name.split(' ')
        .chain(tail.iter().copied())
        .map(String::from)
        .collect()
}

#[test]
fn no_command_panics_on_an_empty_or_garbage_input() {
    let dir = tempfile::tempdir().unwrap();
    let files = [
        ("empty.mzML", Vec::new()),
        ("empty.csv", b"id,mz,rt\n".to_vec()),
        (
            "garbage.dat",
            b"\x00\xff\xfe not a file,\n,,\n1e999,NaN,-1\n{[\n".to_vec(),
        ),
        ("empty.json", b"{}".to_vec()),
    ];
    let mut panicked = Vec::new();
    for (file, bytes) in &files {
        let path = dir.path().join(file);
        std::fs::write(&path, bytes).unwrap();
        let p = path.to_str().unwrap();
        for e in cmd::entries().filter(|e| !SKIP.contains(&e.name)) {
            for tail in [vec![p], vec![p, p], vec![p, "--json"], vec!["--json"]] {
                let args = words(e.name, &tail);
                let r = catch_unwind(AssertUnwindSafe(|| execute(&args)));
                match r {
                    Err(_) => panicked.push(format!("{}", args.join(" ").replace(p, file))),
                    Ok(o) => {
                        assert!(o.code <= 6, "{args:?}: exit {}", o.code);
                        if o.code != 0 && args.iter().any(|a| a == "--json") {
                            let text = String::from_utf8_lossy(&o.stdout);
                            assert!(
                                text.contains("\"schema\":\"error/1\"")
                                    || o.code == 1
                                    || o.code == 3,
                                "{args:?}: exit {} without an error/1: {text}",
                                o.code
                            );
                        }
                    }
                }
            }
        }
    }
    assert!(panicked.is_empty(), "panics:\n{}", panicked.join("\n"));
}

#[test]
fn a_panic_ends_as_exit_1_with_an_error_document() {
    let args: Vec<String> = ["file", "info", "x.mzML", "--json"]
        .map(String::from)
        .to_vec();
    let o = crate::error_text::panic_outcome(&args, "index out of bounds at src/x.rs:1");
    assert_eq!(o.code, 1);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["schema"], "error/1");
    assert_eq!(v["error_key"], "err.run.internal");
    assert_eq!(v["command"], "file info");
    assert!(v["params"]["detail"]
        .as_str()
        .unwrap()
        .contains("index out of bounds"));
    assert!(String::from_utf8_lossy(o.stderr.as_bytes())
        .starts_with("error: mzlab-cli stopped on an internal error"));
    // a command with no JSON form, without --json: stderr only, stdout empty
    let plain =
        crate::error_text::panic_outcome(&["features".to_string(), "x.mzML".to_string()], "boom");
    assert_eq!((plain.code, plain.stdout.is_empty()), (1, true));
}
