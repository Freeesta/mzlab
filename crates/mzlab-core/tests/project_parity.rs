//! `project::guess_sample` and `project::guess_conc` answer like `mzlab/project.py` on ~36000 generated file names
//! (`tools/project_riferimento.py`). Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::project::{guess_conc, guess_sample};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn names_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab_project_{}.json", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/project_riferimento.py"))
        .arg(&out)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 missing"
        );
        eprintln!("skipped: python3 not available");
        return;
    }
    let rows: Vec<Value> = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let _ = std::fs::remove_file(&out);
    let mut bad = vec![];
    for r in &rows {
        let name = r["name"].as_str().unwrap();
        let (label, time, typ) = guess_sample(name);
        let c = guess_conc(name);
        let got = (
            label,
            time,
            typ.to_string(),
            c.as_ref().map(|c| c.0),
            c.and_then(|c| c.1),
        );
        let want = (
            r["label"].as_str().unwrap().to_string(),
            r["time"].as_f64(),
            r["type"].as_str().unwrap().to_string(),
            r["conc"].as_f64(),
            r["cunit"].as_str().map(String::from),
        );
        if got != want {
            bad.push(format!("{name:?}: {got:?} against {want:?}"));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} differ, e.g.\n{}",
        bad.len(),
        rows.len(),
        bad.iter().take(15).cloned().collect::<Vec<_>>().join("\n")
    );
}
