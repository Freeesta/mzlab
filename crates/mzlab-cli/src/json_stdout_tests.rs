//! The stdout of a command run with `--json` is ONE JSON document, whatever the exit code: progress and warnings go to stderr, so a script can parse stdout
//! as it is. A focused list (the catalog does not mark which commands take `--json`), on synthetic data, every case the commands answer differently:
//! done, a warning, a refused file, a missing file and a pair that disagrees with itself.

use std::path::Path;

use crate::contract_tests::{run, text, wiff};

/// The files of one run of the list: a pair, an mzML, a folder to write in and a pair whose `.wiff.scan` outruns its index.
struct Inputs {
    pair: String,
    mzml: String,
    live: String,
    out: String,
    dir: std::path::PathBuf,
}

fn inputs() -> Inputs {
    let (dir, pair) = wiff("json_stdout");
    let mzml = dir.join("a.mzML");
    std::fs::write(&mzml, crate::tests::mzml(10.0)).unwrap();
    // a copy made while acquiring: the pair of tools/wiff_riferimento.py whose scan holds sections the index lacks
    let made = std::process::Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py"))
        .arg(dir.join("ref"))
        .output()
        .expect("python3 is needed by this test");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    Inputs {
        pair,
        mzml: mzml.display().to_string(),
        live: dir.join("ref/unindexed.wiff").display().to_string(),
        out: dir.join("out").display().to_string(),
        dir,
    }
}

/// `serde_json` takes the whole stdout, so a progress line before, between or after the document is an error.
fn one_document(a: &[&str]) {
    let o = run(a);
    let out = text(&o);
    if let Err(e) = serde_json::from_str::<serde_json::Value>(&out) {
        panic!(
            "{a:?} (exit {}): stdout is not one JSON document ({e}): {}\nstderr: {}",
            o.code,
            &out[..out.len().min(300)],
            o.stderr
        );
    }
}

#[test]
fn stdout_of_every_json_command_is_one_json_document() {
    let i = inputs();
    let (w, m, live, out) = (
        i.pair.as_str(),
        i.mzml.as_str(),
        i.live.as_str(),
        i.out.as_str(),
    );
    let dir = i.dir.display().to_string();
    let cases: Vec<Vec<&str>> = vec![
        vec!["sample", "list", w, "--json"],
        vec!["sample", "list", w, "--json", "--probe"],
        vec!["sample", "list", w, m, "--json"],
        vec!["sample", "list", w, "/no/such.wiff", "--json"],
        vec!["sample", "list", live, "--json"],
        vec!["sample", "select", w, "--sample", "1", "--json"],
        vec!["sample", "select", w, "--default", "--json"],
        vec!["file", "pair", w, "--json"],
        vec!["file", "pair", "/no/such.wiff", "--json"],
        vec!["file", "validate", m, "--json"],
        vec!["file", "inventory", w, "--json"],
        vec!["file", "list", &dir, "--json"],
        vec!["file", "convert", w, "--out", out, "--json"],
        vec!["flow", "steps", "--json"],
        vec!["flow", "defaults", "features.detect", "--json"],
        vec!["estimate", m, "--json"],
        vec!["ion", "elements", "--json"],
        vec!["dad", "peaks", m, "--json"],
        vec!["peak", "table", w, "--auto", "--json"],
        vec!["spectrum", "get", m, "--rt", "3", "--json"],
        vec![
            "spectrum",
            "get",
            m,
            "--rt",
            "3",
            "--ms-level",
            "2",
            "--json",
        ],
        vec!["spectrum", "list", m, "--json"],
        vec!["commands", "--json"],
    ];
    for a in &cases {
        one_document(a);
    }
    let _ = std::fs::remove_dir_all(&i.dir);
}
