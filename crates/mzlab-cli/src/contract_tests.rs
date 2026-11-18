//! The agent contract on every command of the table: `--help`/`-h` with nothing run, unknown options, `--json`, exit codes 4 and 5, `error/1`
//! and a newline at the end of stdout. python3 builds the synthetic `.wiff` (`tools/wiff_grande.py`); without it the test fails, it never skips.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cmd::{entries, subwords};
use crate::{execute, Outcome};

fn run(a: &[&str]) -> Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn text(o: &Outcome) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// Every command path of the table: each entry, and each entry with each subcommand word its usage lines list.
fn paths() -> Vec<Vec<String>> {
    let mut v: Vec<Vec<String>> = Vec::new();
    for e in entries() {
        let base: Vec<String> = e.name.split(' ').map(String::from).collect();
        v.push(base.clone());
        for w in subwords(e) {
            let mut p = base.clone();
            p.push(w.to_string());
            v.push(p);
        }
    }
    v
}

/// One error/1 document on stdout, with the code, and nothing else there.
fn one_error(o: &Outcome, code: u8, what: &str) -> serde_json::Value {
    let out = text(o);
    assert_eq!(o.code, code, "{what}: {}", o.stderr);
    assert_eq!(
        out.matches("\"schema\":\"error/1\"").count(),
        1,
        "{what}: {out}"
    );
    assert!(
        out.ends_with('\n') && out.lines().count() == 1,
        "{what}: {out}"
    );
    let v: serde_json::Value =
        serde_json::from_str(out.trim_end()).unwrap_or_else(|e| panic!("{what}: {e}: {out}"));
    assert_eq!(v["code"], code, "{what}");
    assert!(
        v["error_key"].as_str().unwrap().starts_with("err."),
        "{what}"
    );
    assert!(
        !v["message"].as_str().unwrap().starts_with("error:")
            && !v["message"].as_str().unwrap().ends_with('\n'),
        "{what}"
    );
    v
}

#[test]
fn help_and_h_print_the_help_and_nothing_runs() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_contract_help_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (file, written) = (dir.join("in.mzML"), dir.join("written.out"));
    std::fs::write(&file, b"not an mzML: reading it would be an error").unwrap();
    for p in paths() {
        for flag in ["--help", "-h"] {
            // bare, and after a file and an output path that a command which ran would read and write
            for extra in [
                vec![],
                vec![file.to_str().unwrap(), "--out", written.to_str().unwrap()],
            ] {
                let mut a: Vec<&str> = p.iter().map(String::as_str).collect();
                a.extend(extra);
                a.push(flag);
                let o = run(&a);
                assert_eq!(o.code, 0, "{a:?}: {}", o.stderr);
                assert!(
                    !o.stdout.is_empty() && o.stderr.is_empty(),
                    "{a:?}: {}",
                    o.stderr
                );
                assert!(text(&o).ends_with('\n'), "{a:?}");
            }
        }
    }
    assert!(
        !written.exists(),
        "a command ran instead of printing its help"
    );
    for a in [&["--help"][..], &["-h"]] {
        let o = run(a);
        assert!(text(&o).contains("Start here:") && o.code == 0, "{a:?}");
    }
    assert!(!text(&run(&["features", "--help"])).contains("Start here:"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_unknown_option_is_exit_2_and_under_json_one_error_document_for_every_command() {
    for e in entries() {
        let name: Vec<&str> = e.name.split(' ').collect();
        let mut plain = name.clone();
        plain.push("--bogus");
        let o = run(&plain);
        assert_eq!(o.code, 2, "{plain:?}: {}", o.stderr);
        let mut json = plain.clone();
        json.push("--json");
        let v = one_error(&run(&json), 2, &format!("{json:?}"));
        assert_eq!(v["command"], e.name, "{json:?}");
    }
}

#[test]
fn positional_commands_reject_unknown_options() {
    for a in [
        vec!["samples", "group", "a.mzML", "--bogus"],
        vec!["explain-missing", "t.mzfeat", "e.csv", "--bogus"],
        vec!["normative", "bogus"],
        vec!["normative", "pack", "a", "b", "--bogus"],
        vec!["flow", "command", "plan.json", "--bogus"],
    ] {
        let o = run(&a);
        assert_eq!(o.code, 2, "{a:?}: {}", o.stderr);
        assert!(o.stdout.is_empty() || a[0] == "samples", "{a:?}");
    }
}

#[test]
fn a_command_without_a_json_form_says_so_and_never_reports_an_unknown_option() {
    let v = one_error(
        &run(&["features", "x.mzML", "--json"]),
        2,
        "features --json",
    );
    assert_eq!(v["error_key"], "err.cli.json.unsupported");
    assert!(!v["message"].as_str().unwrap().contains("unknown option"));
}

/// The commands that read an input, each with a path that does not exist: exit 4, and under `--json` an error/1 with code 4.
const MISSING: &[&[&str]] = &[
    &["features", "/nonexistent/x.mzML"],
    &[
        "convert",
        "/nonexistent/x.mzML",
        "--out",
        "/tmp/mzlab_contract_unused",
    ],
    &["validate", "/nonexistent/x.mzML"],
    &["channels", "/nonexistent/x.mzML"],
    &["items", "/nonexistent/x.mzML"],
    &["background", "/nonexistent/x.mzML"],
    &[
        "dda-window",
        "/nonexistent/x.mzML",
        "--from",
        "1",
        "--to",
        "2",
    ],
    &["dad", "spectrum", "/nonexistent/x.mzML", "--rt", "1"],
    &["samples", "list", "/nonexistent/x.mzML"],
    &["samples", "select", "/nonexistent/x.mzML", "--sample", "1"],
    &["folder", "list", "/nonexistent/dir"],
    &[
        "manual-table",
        "/nonexistent/x.mzML",
        "--set",
        "windows=/nonexistent/w.json",
    ],
    &[
        "peak-split",
        "/nonexistent/x.mzML",
        "--set",
        "tr=1",
        "--set",
        "a=1",
        "--set",
        "b=2",
    ],
    &["peak-edges", "/nonexistent/t.csv", "--at", "1"],
    &["peak-auto", "/nonexistent/t.csv"],
    &["peak-auto-lr", "/nonexistent/t.csv"],
    &[
        "peak-area",
        "/nonexistent/t.csv",
        "--from",
        "1",
        "--to",
        "2",
    ],
    &[
        "peak-window",
        "/nonexistent/t.csv",
        "--end",
        "1",
        "--other",
        "2",
        "--at",
        "1",
    ],
    &[
        "explain-missing",
        "/nonexistent/t.mzfeat",
        "/nonexistent/e.csv",
    ],
    &["match-mz", "1", "--tables", "/nonexistent/t.json"],
    &["msn-graph", "/nonexistent/g.json"],
    &["msn-lib", "export", "/nonexistent/lib.json"],
    &["msn-rules", "sum", "/nonexistent/lines.json"],
    &[
        "insilico",
        "pack",
        "/nonexistent/x.csv",
        "/tmp/mzlab_contract_unused",
    ],
    &["normative", "listings", "/nonexistent/pack"],
    &[
        "llm",
        "replay",
        "/nonexistent/r.jsonl",
        "--conv",
        "/nonexistent/c.json",
    ],
    &["llm", "context", "/nonexistent/dir"],
    &[
        "flow",
        "run",
        "/nonexistent/p.json",
        "--out",
        "/tmp/mzlab_contract_unused",
    ],
    &["flow", "check", "/nonexistent/p.json"],
    &["flow", "report", "/nonexistent/run"],
    &[
        "flow",
        "record",
        "/nonexistent/run",
        "--roles",
        "/nonexistent/r.json",
    ],
    &["flow", "verify", "/nonexistent/r.json"],
    &["flow", "command", "/nonexistent/p.json"],
    &["mol-label", "/nonexistent/r.json"],
    &["mol-anal", "/nonexistent/r.json"],
    &["mol-check", "/nonexistent/r.json"],
    &["mol-cip", "/nonexistent/r.json"],
    &["mol-hilite", "/nonexistent/r.json"],
    &["mol-sel", "/nonexistent/r.json"],
    &["mol-link", "/nonexistent/r.json"],
    &["mol-export", "/nonexistent/r.json"],
    &["mol-cut", "/nonexistent/m.mol", "--set", "bonds=1-2"],
    &["mol-mech", "/nonexistent/r.json"],
    &["wiff", "list", "/nonexistent/x.wiff"],
];

/// The commands that read no input file: a formula, a number, a query, or nothing.
const NO_FILE: &[&str] = &[
    "elements",
    "ion-pattern",
    "ion-mz",
    "mol-search",
    "help-md",
    "steps",
    "pair",
];

#[test]
fn a_missing_input_is_exit_4_for_every_command_that_reads_one() {
    for a in MISSING {
        let o = run(a);
        assert_eq!(o.code, 4, "{a:?}: {}", o.stderr);
        let mut j: Vec<&str> = a.to_vec();
        j.push("--json");
        let o = run(&j);
        // a command with no JSON form says so (2); the others answer with the code of the failure
        if crate::cmd::reads_json(&a[..a.len().min(2)].join(" "))
            || crate::cmd::reads_json(a[0])
            || crate::cmd::json_by_default(a[0])
            || crate::cmd::json_by_default(&a[..2].join(" "))
        {
            one_error(&o, 4, &format!("{j:?}"));
        }
    }
    for e in entries() {
        let first = e.name.split(' ').next().unwrap();
        let covered = MISSING.iter().any(|a| a[0] == first) || NO_FILE.contains(&e.name);
        assert!(
            covered,
            "{}: add a missing-input case to MISSING or list it in NO_FILE",
            e.name
        );
    }
}

fn wiff(name: &str) -> (PathBuf, String) {
    let dir =
        std::env::temp_dir().join(format!("mzlab_cli_contract_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("wiff_grande.py"))
        .arg(dir.join("in"))
        .args(["--samples", "3", "--scans", "40", "--points", "50"])
        .output()
        .expect("python3 is needed by this test");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let f = dir.join("in/grande.wiff").to_string_lossy().into_owned();
    (dir, f)
}

#[test]
fn an_unknown_selector_is_exit_5_with_error_1() {
    let (dir, f) = wiff("sample");
    for a in [
        vec!["channels", &f, "--sample", "9"],
        vec!["features", &f, "--sample", "9"],
        vec![
            "convert",
            &f,
            "--sample",
            "9",
            "--out",
            "/tmp/mzlab_contract_unused",
        ],
        vec!["samples", "select", &f, "--sample", "9"],
        vec!["channels", &f, "--channel", "sim:nothing"],
        vec!["steps", "--id", "no.such.step"],
        vec!["msn-graph", "node", "--help-not-a-flag-but-a-node", "n999"],
    ] {
        if a[0] == "msn-graph" {
            continue; // the node selector of msn-graph needs a graph file: covered by msn_graph_tests
        }
        let o = run(&a);
        assert_eq!(o.code, 5, "{a:?}: {}", o.stderr);
    }
    let o = run(&["channels", &f, "--sample", "9", "--json"]);
    assert_eq!(
        one_error(&o, 5, "channels --json")["error_key"],
        "err.sample.unknown"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_json_of_the_catalogs_starts_with_its_schema_and_every_stdout_ends_with_a_newline() {
    for a in [
        vec!["steps", "--json", "--id", "features.detect"],
        vec!["llm", "tools"],
        vec!["samples", "group", "a_1.mzML", "a_2.mzML"],
        vec!["ion-pattern", "C6H12O6", "[M+H]+"],
        vec!["elements"],
        vec!["help-md"],
        vec!["--version"],
    ] {
        let o = run(&a);
        assert_eq!(o.code, 0, "{a:?}: {}", o.stderr);
        let out = text(&o);
        assert!(out.ends_with('\n'), "{a:?}");
        if a[0] != "help-md" && a[0] != "--version" {
            let v: serde_json::Value =
                serde_json::from_str(&out).unwrap_or_else(|e| panic!("{a:?}: {e}: {out}"));
            assert!(v.is_object(), "{a:?}");
        }
    }
    for a in [
        vec!["steps", "--json", "--id", "features.detect"],
        vec!["llm", "tools"],
    ] {
        let flat: String = text(&run(&a)).split_whitespace().collect();
        assert!(flat.starts_with("{\"schema\":"), "{a:?}");
    }
}
