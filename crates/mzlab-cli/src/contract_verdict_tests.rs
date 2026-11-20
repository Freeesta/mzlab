//! The contract tests of the codes: each failure code is one `error/1`, the verdict commands keep their answer at exit 1 or 3, any other command
//! wraps its document. Split from `contract_tests.rs` for the size limit; the helpers are shared.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::contract_tests::{one_error, run, text, wiff};
use crate::cmd::entries;

/// A file written into a fresh folder of its own.
fn scratch(name: &str, file: &str, bytes: &[u8]) -> (PathBuf, String) {
    let dir =
        std::env::temp_dir().join(format!("mzlab_cli_contract_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(file), bytes).unwrap();
    let path = dir.join(file).to_string_lossy().into_owned();
    (dir, path)
}

#[test]
fn the_table_of_verdict_commands_names_entries_and_each_keeps_its_document_at_exit_1() {
    for v in crate::contract::VERDICT {
        assert!(entries().any(|e| e.name == *v), "{v} is not a command");
    }
    assert!(crate::contract::VERDICT.contains(&"llm eval"));
    // a pair with no companion, an mzML with an error, a list with a wrong row: exit 1 is the answer, the document stays on stdout
    let (d1, lone) = scratch("verdict_pair", "lone.wiff", b"x");
    let (d2, bad) = scratch("verdict_validate", "bad.mzML", b"<mzML>");
    let (d3, list) = scratch(
        "verdict_reflist",
        "l.csv",
        b"name,formula,adduct\nx,NOTAFORMULA,[M+H]+\n",
    );
    for (a, key) in [
        (vec!["file", "pair", &lone, "--json"], "\"wiffpair/1\""),
        (vec!["file", "validate", &bad, "--json"], "\"valid\":false"),
        (vec!["db", "ref-list", &list], "\"reflist/1\""),
    ] {
        let o = run(&a);
        assert_eq!(o.code, 1, "{a:?}: {}", o.stderr);
        let out = text(&o);
        assert!(
            out.contains(key) && !out.contains("\"schema\":\"error/1\""),
            "{a:?}: {out}"
        );
        assert!(out.ends_with('\n'), "{a:?}");
    }
    for d in [d1, d2, d3] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn every_failure_code_is_one_error_document_under_json() {
    let (dir, junk) = scratch("codes", "plan.json", b"not json");
    let example =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi/HRMS_t000.mzML");
    let example = example.to_string_lossy().into_owned();
    let (wdir, wiff_file) = wiff("codes_wiff");
    for (code, a) in [
        (1, vec!["dad", "spectrum", &example, "--rt", "1", "--json"]),
        (2, vec!["flow", "steps", "--bogus", "--json"]),
        (3, vec!["flow", "check", &junk, "--json"]),
        (4, vec!["file", "channels", "/nonexistent/x.mzML", "--json"]),
        (
            5,
            vec!["file", "channels", &wiff_file, "--sample", "9", "--json"],
        ),
        (
            6,
            vec![
                "ion",
                "elements",
                "--out",
                "/nonexistent/dir/e.json",
                "--json",
            ],
        ),
    ] {
        let v = one_error(&run(&a), code, &format!("{a:?}"));
        assert_eq!(
            crate::exit::code_of_key(v["error_key"].as_str().unwrap()),
            code,
            "{a:?}: the key and the code agree"
        );
    }
    for d in [dir, wdir] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn wiff_and_samples_list_keep_their_document_at_exit_1_and_3_and_a_non_verdict_command_wraps_it() {
    // file inventory exits 3 (a sample refused), file verify exits 1 (a difference): the wiff.list/1 and wiff.verify/1 documents stay on stdout
    let out =
        std::env::temp_dir().join(format!("mzlab_cli_contract_wiffref_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    let made = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .expect("python3 is needed by this test");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let f = |n: &str| out.join(n).to_string_lossy().into_owned();
    for (a, code, schema) in [
        (
            vec!["file", "inventory", &f("periods.wiff")],
            3,
            "wiff.list/1",
        ),
        (
            vec![
                "file",
                "verify",
                &f("one.wiff"),
                "--against",
                &f("two.wiff"),
            ],
            1,
            "wiff.verify/1",
        ),
    ] {
        let o = run(&a);
        assert_eq!(o.code, code, "{a:?}: {}", o.stderr);
        let t = text(&o);
        assert!(
            t.contains(schema) && !t.contains("\"schema\":\"error/1\""),
            "{a:?}: {t}"
        );
    }
    let _ = std::fs::remove_dir_all(out);
    // sample list at exit 1: a .wiff.scan shorter than its index says, asked with --probe
    let (dir, wiff_file) = wiff("verdict_samples");
    let cut = dir.join("cut");
    std::fs::create_dir_all(&cut).unwrap();
    std::fs::copy(&wiff_file, cut.join("grande.wiff")).unwrap();
    let data = std::fs::read(format!("{wiff_file}.scan")).unwrap();
    std::fs::write(cut.join("grande.wiff.scan"), &data[..data.len() - 500]).unwrap();
    let o = run(&[
        "sample",
        "list",
        cut.join("grande.wiff").to_str().unwrap(),
        "--json",
        "--probe",
    ]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    let t = text(&o);
    assert!(
        t.contains("\"samples_list/1\"")
            && t.contains("\"short\"")
            && !t.contains("\"schema\":\"error/1\""),
        "{t}"
    );
    // the other `samples` words are not verdicts: a failure of `sample select` is one error/1
    assert_eq!(
        run(&["sample", "select", &wiff_file, "--sample", "9", "--json"]).code,
        5
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_non_verdict_command_at_exit_3_is_one_error_with_its_document_as_report() {
    // flow check of a plan with a value out of range: exit 3; the flowcheck/1 document is not left on stdout beside an error, it rides in `report`
    let (dir, _) = crate::flow_run_tests::fixture("contract_wrap");
    let plan = dir.join("plan.json").to_string_lossy().into_owned();
    let o = run(&[
        "flow",
        "check",
        &plan,
        "--set",
        "det.min_height=-5",
        "--json",
    ]);
    let v = one_error(&o, 3, "flow check");
    assert_eq!(v["report"]["schema"], "flowcheck/1");
    assert_eq!(v["report"]["valid"], false);
    assert_eq!(v["error_key"], "err.flow.check");
    assert!(v["problems"].as_array().is_some_and(|p| !p.is_empty()));
    let _ = std::fs::remove_dir_all(dir);
}

/// The commands that gained a `--json` form take it (they do not answer `err.cli.json.unsupported`) and fail as one `error/1` with their own code.
#[test]
fn flow_command_takes_json() {
    for (a, code) in [
        (
            vec!["flow", "command", "/nonexistent/plan.json", "--json"],
            4,
        ),
        (vec!["llm", "eval", "--bogus", "--json"], 2),
    ] {
        let v = one_error(&run(&a), code, &a.join(" "));
        assert_ne!(v["error_key"], "err.cli.json.unsupported", "{a:?}");
    }
}
