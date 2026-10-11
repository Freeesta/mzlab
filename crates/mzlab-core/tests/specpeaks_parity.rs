//! `specpeaks` answers like `mzlab/peaks.py` (`tools/specpeaks_riferimento.py`): same peaks (count), m/z and heights within 1e-9
//! relative. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::specpeaks::{merge_unit, pad_zeros, profile_peaks};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}
fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

#[test]
fn peaks_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab_specpeaks_{}.json", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/specpeaks_riferimento.py"))
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
    let rows: Vec<Value> = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let _ = std::fs::remove_file(&out);
    let mut npeaks = 0;
    for (r, row) in rows.iter().enumerate() {
        let (mz, y) = (nums(&row["mz"]), nums(&row["y"]));
        let (gm, gy) = match row["kind"].as_str().unwrap() {
            "profile" => profile_peaks(&mz, &y, row["min_rel"].as_f64().unwrap()),
            "pad" => pad_zeros(&mz, &y, row["step"].as_f64().unwrap()),
            _ => merge_unit(&mz, &y),
        };
        let (wm, wy) = (nums(&row["peaks"][0]), nums(&row["peaks"][1]));
        assert_eq!(gm.len(), wm.len(), "row {r} {}: count", row["kind"]);
        npeaks += gm.len();
        for i in 0..gm.len() {
            assert!(
                close(gm[i], wm[i]) && close(gy[i], wy[i]),
                "row {r} {} point {i}: ({}, {}) against ({}, {})",
                row["kind"],
                gm[i],
                gy[i],
                wm[i],
                wy[i]
            );
        }
    }
    assert!(npeaks > 500, "the generated spectra are too poor: {npeaks}");
}
