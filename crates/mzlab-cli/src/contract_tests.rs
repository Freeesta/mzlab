//! The agent contract on every command of the table: `--help`/`-h`, unknown options, `--json`, exit codes, `error/1`, and a newline at the end of stdout.
//! python3 builds the synthetic `.wiff` (`tools/wiff_grande.py`); without it the test fails, it never skips.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cmd::entries;
use crate::{execute, Outcome};

fn run(a: &[&str]) -> Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn text(o: &Outcome) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// Every command path of the table, and every subcommand word its usage line lists (`mzlab-cli samples vars|group|list|select`).
fn paths() -> Vec<Vec<String>> {
    let mut v: Vec<Vec<String>> = Vec::new();
    for e in entries() {
        let base: Vec<String> = e.name.split(' ').map(String::from).collect();
        v.push(base.clone());
        let marker = format!("mzlab-cli {} ", e.name);
        for line in e.help.split("\n       ") {
            let Some(rest) = line.strip_prefix(&marker) else { continue };
            let word = rest.split(' ').next().unwrap_or("");
            if word.contains('|') && word.chars().all(|c| c.is_ascii_lowercase() || c == '|' || c == '-') {
                for w in word.split('|') {
                    let mut p = base.clone();
                    p.push(w.to_string());
                    v.push(p);
                }
            }
        }
    }
    v
}

#[test]
fn help_and_h_exit_0_with_a_text_on_stdout_and_nothing_on_stderr() {
    for p in paths() {
        for flag in ["--help", "-h"] {
            let mut a: Vec<&str> = p.iter().map(String::as_str).collect();
            a.push(flag);
            let o = run(&a);
            assert_eq!(o.code, 0, "{a:?}: {}", o.stderr);
            assert!(!o.stdout.is_empty() && o.stderr.is_empty(), "{a:?}: {}", o.stderr);
            assert!(text(&o).ends_with('\n'), "{a:?}");
        }
    }
    for a in [&["--help"][..], &["-h"]] {
        let o = run(a);
        assert!(text(&o).contains("Start here:") && o.code == 0, "{a:?}");
    }
    assert!(!text(&run(&["features", "--help"])).contains("Start here:"));
}

#[test]
fn an_unknown_option_is_exit_2_with_one_error_document_under_json() {
    for e in entries().filter(|e| e.name != "help-md" && e.name != "elements" && e.name != "background") {
        let mut a: Vec<&str> = e.name.split(' ').collect();
        a.extend(["--bogus", "--json"]);
        let o = run(&a);
        assert_eq!(o.code, 2, "{a:?}: {}", o.stderr);
        let out = text(&o);
        assert_eq!(out.matches("\"schema\":\"error/1\"").count(), 1, "{a:?}: {out}");
        assert!(out.ends_with('\n') && out.lines().count() == 1, "{a:?}");
        let v: serde_json::Value = serde_json::from_str(out.trim_end()).unwrap();
        assert_eq!(v["code"], 2, "{a:?}");
        assert!(v["error_key"].as_str().unwrap().starts_with("err."), "{a:?}");
        assert!(!v["message"].as_str().unwrap().starts_with("error:"), "{a:?}");
        assert_eq!(v["command"], e.name, "{a:?}");
    }
}

#[test]
fn a_command_without_a_json_form_says_so_and_never_reports_an_unknown_option() {
    let o = run(&["features", "x.mzML", "--json"]);
    assert_eq!(o.code, 2);
    assert!(text(&o).contains("err.cli.json.unsupported"), "{}", text(&o));
    assert!(!o.stderr.contains("unknown option"), "{}", o.stderr);
}

#[test]
fn a_missing_input_file_is_exit_4() {
    for a in [
        vec!["channels", "/nonexistent/x.mzML"],
        vec!["features", "/nonexistent/x.mzML"],
        vec!["items", "/nonexistent/x.mzML"],
        vec!["validate", "/nonexistent/x.mzML"],
        vec!["flow", "check", "/nonexistent/plan.json"],
        vec!["samples", "select", "/nonexistent/x.mzML", "--sample", "1"],
    ] {
        let o = run(&a);
        assert_eq!(o.code, 4, "{a:?}: {}", o.stderr);
    }
    let o = run(&["channels", "/nonexistent/x.mzML", "--json"]);
    assert!(text(&o).contains("\"code\":4"), "{}", text(&o));
}

fn wiff(name: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_contract_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("wiff_grande.py"))
        .arg(dir.join("in"))
        .args(["--samples", "3", "--scans", "40", "--points", "50"])
        .output()
        .expect("python3 is needed by this test");
    assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
    let f = dir.join("in/grande.wiff").to_string_lossy().into_owned();
    (dir, f)
}

#[test]
fn an_unknown_sample_is_exit_5_with_error_1() {
    let (dir, f) = wiff("sample");
    for a in [
        vec!["channels", &f, "--sample", "9", "--json"],
        vec!["features", &f, "--sample", "9"],
        vec!["samples", "select", &f, "--sample", "9", "--json"],
    ] {
        let o = run(&a);
        assert_eq!(o.code, 5, "{a:?}: {}", o.stderr);
    }
    let o = run(&["channels", &f, "--sample", "9", "--json"]);
    assert!(text(&o).contains("\"error_key\":\"err.sample.unknown\""), "{}", text(&o));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_json_of_the_catalogs_starts_with_its_schema_and_every_stdout_ends_with_a_newline() {
    for a in [
        vec!["steps", "--json", "--id", "features.detect"],
        vec!["llm", "tools"],
        vec!["samples", "group", "a_1.mzML", "a_2.mzML"],
        vec!["ion-pattern", "C6H12O6", "[M+H]+"],
    ] {
        let o = run(&a);
        assert_eq!(o.code, 0, "{a:?}: {}", o.stderr);
        let out = text(&o);
        assert!(out.ends_with('\n'), "{a:?}");
        let first = out.lines().next().unwrap();
        let v: serde_json::Value = serde_json::from_str(first).unwrap_or_else(|e| panic!("{a:?}: {e}: {first}"));
        let _ = v;
    }
    for a in [vec!["steps", "--json", "--id", "features.detect"], vec!["llm", "tools"]] {
        assert!(text(&run(&a)).starts_with("{\"schema\":"), "{a:?}");
    }
}
