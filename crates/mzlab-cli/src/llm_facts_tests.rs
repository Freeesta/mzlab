//! `llm inspect` and `llm params` on three synthetic files of `tools/dati_sintetici.py` (skipped without python3 and numpy; `MZLAB_GOLDEN_STRICT=1`
//! fails instead): the bytes are the golden files `tests/data/llm/fatti/` that `tests_e2e/e2e_llm_fatti.py` also compares the page's answers with, so
//! page and CLI agree byte for byte. Also writes the inspection G01 and its proposal that `mzlab-llm-wasm` checks its export against (both with the registry
//! `docs/schemi/steps-1.json`). `MZLAB_UPDATE_LLM=1` rewrites the golden files.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::params::G01;
use crate::execute;
use crate::llm::facts::{inspection, BUDGET};

/// The files the page test opens, in this order.
pub(crate) const DONE: [&str; 3] = ["B_FullMass-t0.mzML", "B_MRM-t0.mzML", "C_IDA-t0.mzML"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn golden(rel: &str, got: &str) {
    let path = root().join(rel);
    if std::env::var("MZLAB_UPDATE_LLM").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, got).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap_or_default(),
        got,
        "{rel}: MZLAB_UPDATE_LLM=1 rewrites it"
    );
}

/// A folder with the three files, `None` without python3 and numpy.
fn folder() -> Option<PathBuf> {
    // One directory per call: tests run in parallel, and a shared one was rewritten by the generator while another test read it.
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let all = std::env::temp_dir().join(format!("mzlab-fatti-{}-{n}", std::process::id()));
    std::fs::remove_dir_all(&all).ok();
    let ok = Command::new("python3")
        .current_dir(root())
        .arg("tools/dati_sintetici.py")
        .arg(&all)
        .output();
    if !matches!(&ok, Ok(o) if o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "the generator did not run: {ok:?}"
        );
        eprintln!("python3/numpy not available: test skipped");
        return None;
    }
    let dir = all.join("fatti");
    std::fs::create_dir_all(&dir).unwrap();
    for f in DONE {
        std::fs::copy(all.join(f), dir.join(f)).unwrap();
    }
    Some(dir)
}

fn cli(args: &[&str]) -> String {
    let o = execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    assert_eq!(o.code, 0, "{args:?}: {}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn llm_inspect_and_params_give_the_golden_bytes() {
    let dir = strict_folder();
    let d = dir.to_str().unwrap();
    let insp = cli(&["llm", "inspect", d]);
    assert_eq!(
        insp.trim_end(),
        inspection(
            &crate::llm::facts::inputs(&[d.to_string()]).ok().unwrap(),
            false,
            BUDGET
        )
        .unwrap()
    );
    assert!(insp.starts_with("{\"schema\":\"inspection/1\""), "{insp}");
    golden("tests/data/llm/fatti/inspection.json", &insp);
    let prop = cli(&["llm", "params", d]);
    assert!(
        prop.starts_with("{\"schema\":\"param-proposal/1\""),
        "{prop}"
    );
    golden("tests/data/llm/fatti/proposal.json", &prop);
    for old in ["ispezione", "proposta", "contesto", "ricerca"] {
        assert!(!insp.contains(old) && !prop.contains(old), "{old}");
    }
    // the same bytes twice, and hidden names leave no file name
    assert_eq!(cli(&["llm", "inspect", d]), insp);
    let hidden = cli(&["llm", "inspect", d, "--hide-names"]);
    assert!(DONE.iter().all(|f| !hidden.contains(f)), "{hidden}");
}

#[test]
fn llm_inspect_needs_a_folder() {
    let o = execute(&["llm".to_string(), "inspect".to_string()]);
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("usage"), "{}", o.stderr);
}

#[test]
fn the_proposal_of_g01_is_the_one_the_wasm_export_checks() {
    let one_line: String = G01.lines().collect();
    golden(
        "crates/mzlab-llm/tests/data/inspection_g01.json",
        &(one_line.clone() + "\n"),
    );
    let steps = std::fs::read(root().join("docs/schemi/steps-1.json")).unwrap();
    let registry =
        mzlab_core::step::registry_from_value(&mzlab_core::json::parse(&steps).unwrap()).unwrap();
    let out = mzlab_llm::params_rules::propose_text(&one_line, "", &registry).unwrap();
    golden(
        "crates/mzlab-llm/tests/data/proposal_g01.json",
        &(out + "\n"),
    );
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// The synthetic folder; these tests never skip: without python3 and numpy they fail, so a green run always compared the bytes.
fn strict_folder() -> PathBuf {
    folder().expect("these tests need python3 and numpy (tools/dati_sintetici.py)")
}

fn files(dir: &Path) -> Vec<PathBuf> {
    DONE.iter().map(|f| dir.join(f)).collect()
}

#[test]
fn files_and_a_folder_give_the_same_bytes_with_one_worker_or_many() {
    let dir = strict_folder();
    let files = files(&dir);
    let by_dir = crate::llm::facts::inputs(&[dir.to_str().unwrap().to_string()])
        .ok()
        .unwrap();
    let one = crate::llm::facts::inspection_with(&files, false, BUDGET, 1).unwrap();
    assert!(one.starts_with("{\"schema\":\"inspection/1\""));
    for workers in [2, 4, 8] {
        let n = crate::llm::facts::inspection_with(&files, false, BUDGET, workers).unwrap();
        assert_eq!(one, n, "{workers} workers");
    }
    assert_eq!(
        one,
        crate::llm::facts::inspection_with(&by_dir, false, BUDGET, 4).unwrap()
    );
}

fn saved_inspection(dir: &Path) -> PathBuf {
    let mut a = args(&["llm", "inspect"]);
    a.extend(files(dir).iter().map(|f| f.to_str().unwrap().to_string()));
    let o = execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let saved = dir.join("inspection.json");
    std::fs::write(&saved, &o.stdout).unwrap();
    saved
}

#[test]
fn params_on_an_inspection_from_a_file_or_stdin_is_params_on_the_files() {
    let dir = strict_folder();
    let saved = saved_inspection(&dir);
    let want = cli(&["llm", "params", dir.to_str().unwrap()]);
    let reuse = cli(&["llm", "params", "--inspection", saved.to_str().unwrap()]);
    assert_eq!(reuse, want);
    // standard input: the bytes of `llm inspect A B` piped in
    let mut piped = std::io::Cursor::new(std::fs::read(&saved).unwrap());
    let o = crate::llm::facts::params_with(args(&["--inspection", "-"]), &mut piped);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(String::from_utf8(o.stdout).unwrap(), want);
    // the proposal logs its automatic choices with their reasons
    let doc: serde_json::Value = serde_json::from_str(&want).unwrap();
    let choices = doc["choices"].as_array().unwrap();
    let keys: Vec<&str> = choices.iter().map(|c| c["key"].as_str().unwrap()).collect();
    assert_eq!(keys, ["reference_file", "resolution_class", "answers"]);
    assert!(choices
        .iter()
        .all(|c| !c["reason"].as_str().unwrap().is_empty()));
    // a budget, hidden names or files belong elsewhere
    for extra in [vec!["--budget", "5"], vec!["--hide-names"], vec!["x.mzML"]] {
        let mut a = args(&["llm", "params", "--inspection", saved.to_str().unwrap()]);
        a.extend(extra.iter().map(|s| s.to_string()));
        let o = execute(&a);
        assert_eq!(o.code, 2, "{extra:?}: {}", o.stderr);
    }
    std::fs::remove_file(saved).ok();
}

#[test]
fn a_file_named_twice_is_refused_even_through_its_folder() {
    let dir = strict_folder();
    let d = dir.to_str().unwrap();
    let again = files(&dir)[0].to_str().unwrap().to_string();
    for a in [
        vec!["llm", "inspect", d, again.as_str()],
        vec!["llm", "params", again.as_str(), again.as_str(), "--json"],
    ] {
        let o = execute(&args(&a));
        assert_eq!(o.code, 2, "{a:?}: {}", o.stderr);
        assert!(o.stderr.contains("more than once"), "{}", o.stderr);
    }
}

#[test]
fn params_on_a_bad_or_missing_inspection_fails_with_the_contract_codes() {
    let o = execute(&args(&[
        "llm",
        "params",
        "--inspection",
        "/nonexistent/i.json",
    ]));
    assert_eq!(o.code, 4, "{}", o.stderr);
    let bad = std::env::temp_dir().join(format!("mzlab-badinsp-{}.json", std::process::id()));
    std::fs::write(&bad, "{\"schema\":\"other/1\"}").unwrap();
    let o = execute(&args(&[
        "llm",
        "params",
        "--inspection",
        bad.to_str().unwrap(),
        "--json",
    ]));
    assert_eq!(o.code, 2);
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .contains("\"schema\":\"error/1\""));
    std::fs::remove_file(bad).ok();
    // nothing to read, and a file that is not there
    assert_eq!(execute(&args(&["llm", "params"])).code, 2);
    let o = execute(&args(&["llm", "inspect", "/nonexistent/a.mzML", "--json"]));
    assert_eq!(o.code, 4);
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .contains("\"schema\":\"error/1\""));
}

#[test]
fn a_bad_answers_file_takes_the_declared_codes_with_one_error_document() {
    let dir = strict_folder();
    let d = dir.to_str().unwrap();
    // beside the folder, not in it: a folder is read whole
    let bad = dir.with_file_name("answers-bad.json");
    std::fs::write(&bad, "not json").unwrap();
    let missing = dir.with_file_name("answers-missing.json");
    for (answers, code) in [(&bad, 2), (&missing, 4)] {
        let o = execute(&args(&[
            "llm",
            "params",
            d,
            "--answers",
            answers.to_str().unwrap(),
            "--json",
        ]));
        assert_eq!(o.code, code, "{answers:?}: {}", o.stderr);
        let out = String::from_utf8(o.stdout).unwrap();
        assert!(out.contains("\"schema\":\"error/1\""), "{out}");
        assert!(declared("llm params").contains(&o.code));
    }
    std::fs::remove_file(bad).ok();
}

/// The exit codes a command declares in its table row.
fn declared(name: &str) -> &'static [u8] {
    crate::cmd::entries()
        .find(|e| e.name == name)
        .unwrap()
        .exits
}

#[test]
fn flow_command_and_llm_context_errors_stay_among_their_declared_codes() {
    let tmp = std::env::temp_dir().join(format!("mzlab-exits-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let bad = tmp.join("plan-bad.json");
    std::fs::write(&bad, "not json").unwrap();
    let bad = bad.to_str().unwrap();
    for (a, code) in [
        (vec!["flow", "command", bad, "--json"], 3),
        (
            vec!["flow", "command", "/nonexistent/plan.json", "--json"],
            4,
        ),
        (vec!["flow", "command", "--json"], 2),
        (vec!["llm", "context", "/nonexistent/run", "--json"], 4),
        (vec!["llm", "context", "--json"], 2),
    ] {
        let o = execute(&args(&a));
        assert_eq!(o.code, code, "{a:?}: {}", o.stderr);
        let name = format!("{} {}", a[0], a[1]);
        assert!(declared(&name).contains(&o.code), "{name}: {}", o.code);
        let out = String::from_utf8(o.stdout).unwrap();
        assert!(out.contains("\"schema\":\"error/1\""), "{a:?}: {out}");
    }
    std::fs::remove_dir_all(tmp).ok();
}
