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
pub(crate) const FATTI: [&str; 3] = ["B_FullMass-t0.mzML", "B_MRM-t0.mzML", "C_IDA-t0.mzML"];

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
    for f in FATTI {
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
    assert_eq!(insp.trim_end(), inspection(&dir, false, BUDGET).unwrap());
    assert!(insp.starts_with("{\"schema\":\"ispezione/1\""), "{insp}");
    golden("tests/data/llm/fatti/ispezione.json", &insp);
    let prop = cli(&["llm", "params", d]);
    assert!(
        prop.starts_with("{\"schema\":\"proposta-param/1\""),
        "{prop}"
    );
    golden("tests/data/llm/fatti/proposta.json", &prop);
    // the same bytes twice, and hidden names leave no file name
    assert_eq!(cli(&["llm", "inspect", d]), insp);
    let hidden = cli(&["llm", "inspect", d, "--hide-names"]);
    assert!(FATTI.iter().all(|f| !hidden.contains(f)), "{hidden}");
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
        "crates/mzlab-llm/tests/data/ispezione_g01.json",
        &(one_line.clone() + "\n"),
    );
    let steps = std::fs::read(root().join("docs/schemi/steps-1.json")).unwrap();
    let registry =
        mzlab_core::step::registry_from_value(&mzlab_core::json::parse(&steps).unwrap()).unwrap();
    let out = mzlab_llm::params_rules::propose_text(&one_line, "", &registry).unwrap();
    golden(
        "crates/mzlab-llm/tests/data/proposta_g01.json",
        &(out + "\n"),
    );
}
