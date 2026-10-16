//! `subpeaks` parity test against TP_Mine/py/tpmine/hr/spectra.py reference answers.
//! Generates synthetic chromatographic traces via `tools/subpeaks_riferimento.py` and verifies exact match of apex retention times within 1e-9.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::subpeaks::subpeaks;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| x.as_f64()).collect())
        .unwrap_or_default()
}

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

#[test]
fn subpeaks_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab_subpeaks_{}.json", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/subpeaks_riferimento.py"))
        .arg(&out)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let data = match std::fs::read(&out) {
        Ok(d) => d,
        Err(e) => panic!("failed to read generated reference file: {e}"),
    };
    let _ = std::fs::remove_file(&out);
    let rows: Vec<Value> = match serde_json::from_slice(&data) {
        Ok(r) => r,
        Err(e) => panic!("failed to parse json reference: {e}"),
    };

    assert!(
        rows.len() >= 40,
        "expected at least 40 test cases, got {}",
        rows.len()
    );

    for (r, row) in rows.iter().enumerate() {
        let rt = nums(&row["rt"]);
        let y = nums(&row["y"]);
        let min_rel = row["min_rel"].as_f64().unwrap_or(0.15);
        let smooth_min = row["smooth_min"].as_f64().unwrap_or(0.03);
        let min_sep = row["min_sep"].as_f64().unwrap_or(0.08);
        let want = nums(&row["expected"]);

        let got = subpeaks(&rt, &y, min_rel, smooth_min, min_sep);
        let case_name = row["case"].as_str().unwrap_or("unknown");

        assert_eq!(
            got.len(),
            want.len(),
            "case {r} ({case_name}): count mismatch (got {got:?}, want {want:?})"
        );
        for (i, (&g, &w)) in got.iter().zip(want.iter()).enumerate() {
            assert!(
                close(g, w),
                "case {r} ({case_name}) apex {i}: got {g} want {w} (diff: {})",
                (g - w).abs()
            );
        }
    }
}
