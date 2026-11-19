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
    let all = std::env::temp_dir().join(format!("mzlab-fatti-{}", std::process::id()));
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
    let Some(dir) = folder() else { return };
    let d = dir.to_str().unwrap();
    let insp = cli(&["llm", "inspect", d]);
    assert_eq!(
        insp.trim_end(),
        inspection(
            &crate::llm::facts::inputs(&[d.to_string()]).unwrap(),
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

#[test]
fn files_and_a_folder_give_the_same_bytes_with_one_worker_or_many() {
    let Some(dir) = folder() else { return };
    let files: Vec<PathBuf> = DONE.iter().map(|f| dir.join(f)).collect();
    let by_dir = crate::llm::facts::inputs(&[dir.to_str().unwrap().to_string()]).unwrap();
    let one = crate::llm::facts::inspection_with(&files, false, BUDGET, 1).unwrap();
    for workers in [2, 4, 8] {
        let n = crate::llm::facts::inspection_with(&files, false, BUDGET, workers).unwrap();
        assert_eq!(one, n, "{workers} workers");
    }
    assert_eq!(
        one,
        crate::llm::facts::inspection_with(&by_dir, false, BUDGET, 4).unwrap()
    );
}

#[test]
fn params_on_a_saved_inspection_is_params_on_the_files() {
    let Some(dir) = folder() else { return };
    let d = dir.to_str().unwrap();
    let mut a = args(&["llm", "inspect"]);
    a.extend(
        DONE.iter()
            .map(|f| dir.join(f).to_str().unwrap().to_string()),
    );
    let o = execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let saved = dir.join("inspection.json");
    std::fs::write(&saved, &o.stdout).unwrap();
    let reuse = cli(&["llm", "params", "--inspection", saved.to_str().unwrap()]);
    assert_eq!(reuse, cli(&["llm", "params", d]));
    // a budget or hidden names belong to the inspection
    for extra in [["--budget", "5"], ["--hide-names", "x"]] {
        let o = execute(&args(&[
            "llm",
            "params",
            "--inspection",
            saved.to_str().unwrap(),
            extra[0],
            extra[1],
        ]));
        assert_eq!(o.code, 2, "{extra:?}: {}", o.stderr);
    }
    std::fs::remove_file(saved).ok();
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
    // files and an inspection together, or neither
    assert_eq!(execute(&args(&["llm", "params"])).code, 2);
    assert_eq!(
        execute(&args(&["llm", "inspect", "/nonexistent/a.mzML"])).code,
        4
    );
}
