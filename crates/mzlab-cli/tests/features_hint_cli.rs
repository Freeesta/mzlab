//! `features` through the real binary on a synthetic replicate series: a plain run (no `--report`) says on stderr that rows are still close and names
//! `dedup_overlap`, the CSV on stdout stays clean, and the exit code is 0. Fails, never skips, when python3/numpy is missing in CI.

use std::path::Path;
use std::process::Command;

#[test]
fn a_plain_run_names_dedup_overlap_on_stderr_when_close_rows_remain() {
    let d = std::env::temp_dir().join(format!("mzlab_hint_cli_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("lr_synthetic.py"))
        .arg(&d)
        .args(["--seed", "1", "--replicates", "4"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !made {
        assert!(
            ["MZLAB_GOLDEN_STRICT", "CI", "GITHUB_ACTIONS"]
                .iter()
                .all(|k| std::env::var_os(k).is_none()),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
        .arg("features")
        .args((0..4).map(|k| d.join(format!("rep-s{k}.mzML"))))
        .output()
        .unwrap();
    let (err, csv) = (
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );
    assert_eq!(out.status.code(), Some(0), "{err}");
    let note = err
        .lines()
        .find(|l| l.starts_with("note:") && l.contains("dedup_overlap"))
        .unwrap_or_else(|| panic!("no hint on stderr: {err}"));
    assert!(
        note.contains("--params") && note.contains("may be different ions"),
        "{note}"
    );
    assert!(!csv.contains("dedup_overlap"), "the data stays clean");
    let _ = std::fs::remove_dir_all(d);
}
