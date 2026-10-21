//! The JSON answers of `mzlab-cli` (`steps --json`, `flow check`, `--json` and `--progress jsonl` of `flow run`) on the synthetic plan of
//! `flow_run_tests.rs`: same ids as the exported descriptors, same codes, same files and stderr with and without `--json`, no clock in the bytes.

use std::path::Path;

use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{parse, Value};

use crate::flow_run_tests::{fixture, flow_run, folder, p, untimed};

fn json(bytes: &[u8]) -> Value {
    parse(bytes).unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(bytes)))
}

fn run_cli(args: &[&str]) -> crate::Outcome {
    crate::execute(&args.iter().map(|x| x.to_string()).collect::<Vec<_>>())
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::str).unwrap_or_default()
}

/// When `MZLAB_JSON_SAMPLES` names a folder the documents are left there for `tests/test_cli_json.py`.
fn sample(name: &str, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("MZLAB_JSON_SAMPLES") {
        std::fs::write(Path::new(&dir).join(name), bytes).unwrap();
    }
}

#[test]
fn steps_json_has_the_ids_of_the_exports_and_the_native_flags() {
    let all = run_cli(&["steps", "--json"]);
    assert_eq!(all.code, 0, "{}", all.stderr);
    let list = json(&all.stdout);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/schemi");
    let mut want = Vec::new();
    for f in [
        "steps-1.json",
        "steps-attributio-1.json",
        "steps-tabui-1.json",
        "steps-cli-1.json",
    ] {
        let exported = json(&std::fs::read(root.join(f)).unwrap());
        want.extend(exported.arr().iter().map(|s| str_of(s, "id").to_string()));
    }
    let got: Vec<String> = list
        .arr()
        .iter()
        .map(|s| str_of(s, "id").to_string())
        .collect();
    let (mut a, mut b) = (got.clone(), want);
    a.sort();
    b.sort();
    assert_eq!(a, b, "the registry equals the four exports");
    for s in list.arr() {
        let id = str_of(s, "id");
        assert_eq!(
            s.get("executable_native").and_then(Value::bool),
            Some(crate::flow::executable(id)),
            "{id}"
        );
        assert_eq!(str_of(s, "schema"), "step/1");
    }
    // the new key is the last of each step
    let text = String::from_utf8(all.stdout.clone()).unwrap();
    assert!(
        text.lines()
            .filter(|l| l.starts_with('{'))
            .all(
                |l| l.contains(",\"executable_native\":") && l.ends_with("e}")
                    || l.ends_with("e},")
            ),
        "{text}"
    );

    let one = run_cli(&["steps", "--json", "--id", "features.table"]);
    assert_eq!(one.code, 0);
    assert_eq!(str_of(&json(&one.stdout), "id"), "features.table");
    let at = run_cli(&["steps", "--id", "features.detect@1"]);
    assert!(
        at.stdout.starts_with(b"features.detect@1  native  "),
        "{}",
        String::from_utf8_lossy(&at.stdout)
    );
    let lines = run_cli(&["steps"]);
    assert_eq!(
        String::from_utf8(lines.stdout).unwrap().lines().count(),
        list.arr().len()
    );

    let unknown = run_cli(&["steps", "--json", "--id", "nope.nothing"]);
    assert_eq!(unknown.code, 2);
    let err = json(&unknown.stdout);
    assert_eq!(
        (str_of(&err, "schema"), str_of(&err, "error_key")),
        ("errore/1", "err.flow.step.unknown")
    );
    assert!(
        unknown
            .stderr
            .starts_with("error: unknown step nope.nothing"),
        "{}",
        unknown.stderr
    );
    assert_eq!(run_cli(&["steps", "--id", "nope"]).stdout, Vec::<u8>::new());
}

#[test]
fn flow_check_says_what_is_wrong_without_the_input_files() {
    let (dir, _) = fixture("check");
    let plan = dir.join("plan.json");
    // the inputs are not even beside the plan: the check does not look for them
    std::fs::remove_dir_all(dir.join("data")).unwrap();
    let ok = run_cli(&["flow", "check", p(&plan), "--json"]);
    assert_eq!(ok.code, 0, "{}", ok.stderr);
    let v = json(&ok.stdout);
    assert_eq!(
        (str_of(&v, "schema"), v.get("valid").and_then(Value::bool)),
        ("flowcheck/1", Some(true))
    );
    assert_eq!(run_cli(&["flow", "check", p(&plan)]).code, 0);

    let bad = run_cli(&[
        "flow",
        "check",
        p(&plan),
        "--set",
        "det.min_height=-5",
        "--json",
    ]);
    assert_eq!(bad.code, 3, "{}", bad.stderr);
    let v = json(&bad.stdout);
    assert_eq!(v.get("valid").and_then(Value::bool), Some(false));
    let first = v.get("problems").unwrap().at(0).unwrap();
    assert_eq!(str_of(first, "instance"), "det");
    assert_eq!(str_of(first, "error_key"), "err.flow.param.range");
    assert_eq!(str_of(first.get("params").unwrap(), "param"), "min_height");
    assert!(
        bad.stderr.contains("det:err.flow.param.range"),
        "{}",
        bad.stderr
    );
    sample("flowcheck.json", &bad.stdout);

    let set = run_cli(&[
        "flow",
        "check",
        p(&plan),
        "--set",
        "det.min_height",
        "--json",
    ]);
    assert_eq!(set.code, 2);
    assert_eq!(str_of(&json(&set.stdout), "error_key"), "err.cli.set");
    assert_eq!(run_cli(&["flow", "check", p(&plan), "--bogus"]).code, 2);
    assert_eq!(
        run_cli(&["flow", "check", p(&dir.join("none.json"))]).code,
        4
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn flow_run_json_gives_flowrun_with_the_hashes_of_the_files_and_changes_nothing_else() {
    let (dir, _) = fixture("json");
    let (plan, data) = (dir.join("plan.json"), dir.join("data"));
    let (a, b) = (dir.join("a"), dir.join("b"));
    let args = |out: &Path, extra: &[&str]| {
        let mut v = vec![
            p(&plan).to_string(),
            "--in".into(),
            p(&data).into(),
            "--out".into(),
            p(out).into(),
        ];
        v.extend(extra.iter().map(|x| x.to_string()));
        v
    };
    let plain = flow_run(&args(&a, &[]).iter().map(String::as_str).collect::<Vec<_>>());
    let with = flow_run(
        &args(&b, &["--json"])
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    assert_eq!((plain.code, with.code), (0, 0), "{}", with.stderr);
    assert!(plain.stdout.is_empty());
    assert_eq!(
        plain.stderr.replace(p(&a), "OUT").lines().last(),
        with.stderr.replace(p(&b), "OUT").lines().last()
    );
    assert_eq!(
        folder(&a),
        folder(&b),
        "same files and prov.json, times apart"
    );

    let v = json(&with.stdout);
    assert_eq!(
        (str_of(&v, "schema"), v.get("code").and_then(Value::f64)),
        ("flowrun/1", Some(0.0))
    );
    assert_eq!(
        str_of(&v, "plan_sha256"),
        sha256_hex(&std::fs::read(b.join("plan.json")).unwrap())
    );
    let steps = v.get("steps").unwrap().arr();
    assert_eq!(steps.len(), 4);
    let mut files = 0;
    for s in steps {
        assert_eq!(str_of(s, "status"), "ok");
        for o in s.get("outputs").unwrap().arr() {
            let bytes = std::fs::read(b.join(str_of(o, "file"))).unwrap();
            assert_eq!(
                str_of(o, "sha256"),
                sha256_hex(&bytes),
                "{}",
                str_of(o, "file")
            );
            assert_eq!(
                o.get("bytes").and_then(Value::f64),
                Some(bytes.len() as f64)
            );
            files += 1;
        }
    }
    assert!(files >= 4, "{files} files");
    sample("flowrun.json", &with.stdout);

    // the same bytes the second time, the clock apart; the folder reused: every step cached
    std::fs::remove_dir_all(&b).unwrap();
    let again = flow_run(
        &args(&b, &["--json"])
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        untimed(&String::from_utf8(again.stdout).unwrap()),
        untimed(&String::from_utf8(with.stdout).unwrap())
    );
    let cached = flow_run(
        &args(&b, &["--json"])
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    let v = json(&cached.stdout);
    assert!(v
        .get("steps")
        .unwrap()
        .arr()
        .iter()
        .all(|s| str_of(s, "status") == "cached"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dry_run_json_writes_nothing_and_errors_come_as_errore() {
    let (dir, _) = fixture("dry");
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let dry = flow_run(&[
        p(&plan),
        "--in",
        p(&data),
        "--out",
        p(&out),
        "--dry-run",
        "--json",
    ]);
    assert_eq!(dry.code, 0, "{}", dry.stderr);
    assert!(!out.exists(), "nothing is written");
    let v = json(&dry.stdout);
    assert_eq!(str_of(&v, "schema"), "flowdry/1");
    assert_eq!(v.get("inputs").unwrap().arr().len(), 3);
    let steps = v.get("steps").unwrap().arr();
    assert_eq!(str_of(&steps[0], "step"), "index.mzidx@1");
    assert_eq!(str_of(&steps[3], "instance"), "gap");
    assert_eq!(str_of(&steps[3], "params_sha256").len(), 64);
    sample("flowdry.json", &dry.stdout);

    // a malformed --set (2) and a missing input (4): the code and stderr of the text form, the document on stdout
    let text = |args: &[&str]| flow_run(args);
    let set = ["--set", "gap"];
    let bad = flow_run(&[
        p(&plan),
        "--in",
        p(&data),
        "--out",
        p(&out),
        set[0],
        set[1],
        "--json",
    ]);
    let same = text(&[p(&plan), "--in", p(&data), "--out", p(&out), set[0], set[1]]);
    assert_eq!((bad.code, same.code), (2, 2));
    assert_eq!(bad.stderr, same.stderr);
    let e = json(&bad.stdout);
    assert_eq!(
        (str_of(&e, "schema"), str_of(&e, "error_key")),
        ("errore/1", "err.cli.set")
    );
    assert_eq!(e.get("code").and_then(Value::f64), Some(2.0));
    assert!(!str_of(&e, "hint").is_empty());
    sample("errore.json", &bad.stdout);

    let missing = flow_run(&[p(&plan), "--in", p(&dir), "--out", p(&out), "--json"]);
    assert_eq!(missing.code, 4, "{}", missing.stderr);
    let e = json(&missing.stdout);
    assert_eq!(str_of(&e, "error_key"), "err.cli.input.missing");
    assert_eq!(str_of(e.get("params").unwrap(), "input"), "s_t000.mzML");

    let usage = flow_run(&[p(&plan), "--bogus", "--json"]);
    assert_eq!(usage.code, 2);
    assert_eq!(str_of(&json(&usage.stdout), "error_key"), "err.cli.usage");

    // a plan that is not valid: the problems ride in errore/1, code 3
    let invalid = flow_run(&[
        p(&plan),
        "--in",
        p(&data),
        "--out",
        p(&out),
        "--set",
        "det.min_height=-5",
        "--json",
    ]);
    assert_eq!(invalid.code, 3);
    let e = json(&invalid.stdout);
    assert_eq!(str_of(&e, "error_key"), "err.flow.check");
    assert_eq!(
        str_of(e.get("problems").unwrap().at(0).unwrap(), "error_key"),
        "err.flow.param.range"
    );
    assert!(!out.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The run of `progress_jsonl_is_one_json_line_per_job`, in a child process of the test binary, so that its stderr can be read.
#[test]
fn progress_child() {
    let Some(dir) = std::env::var_os("MZLAB_JSONL_CHILD") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let o = flow_run(&[
        p(&plan),
        "--in",
        p(&data),
        "--out",
        p(&out),
        "--progress",
        "jsonl",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
}

#[test]
fn progress_jsonl_is_one_json_line_per_job() {
    let (dir, _) = fixture("jsonl");
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "flow_json_tests::progress_child",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env("MZLAB_JSONL_CHILD", &dir)
        .output()
        .unwrap();
    let err = String::from_utf8(child.stderr).unwrap();
    assert!(child.status.success(), "{err}");
    let jobs: Vec<Value> = err
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| json(l.as_bytes()))
        .collect();
    assert_eq!(
        jobs.len(),
        8,
        "3 indexes, 3 detections, align, gapfill: {err}"
    );
    for j in &jobs {
        assert_eq!(j.get("of").and_then(Value::f64), Some(4.0));
        assert_eq!(str_of(j, "status"), "ok");
        assert!(j.get("ms").and_then(Value::f64).is_some());
    }
    assert!(
        jobs.iter()
            .any(|j| j.get("item").and_then(Value::f64) == Some(3.0)),
        "per-file steps number their items"
    );
    assert!(jobs.last().is_some_and(
        |j| str_of(j, "instance") == "gap" && j.get("item").is_some_and(Value::is_null)
    ));
    assert!(!err.contains("[1/4]"), "{err}");
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out2"));
    assert_eq!(
        flow_run(&[
            p(&plan),
            "--in",
            p(&data),
            "--out",
            p(&out),
            "--progress",
            "no"
        ])
        .code,
        2
    );
    let _ = std::fs::remove_dir_all(&dir);
}
