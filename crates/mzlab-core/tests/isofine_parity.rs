//! `isofine::isotope_fine_route` against `tools/isotopi_fini_riferimento.py` (the Python `fine_pattern` without a cut): same peaks, m/z within 1e-9
//! and intensities within 1e-6 relative. Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::isofine::isotope_fine_route;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn isotope_fine_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab-isofine-{}.json", std::process::id()));
    let ok = Command::new("python3")
        .current_dir(root())
        .arg("tools/isotopi_fini_riferimento.py")
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
    for r in &rows {
        let query = r["query"].as_str().unwrap();
        let got: Vec<Vec<f64>> = serde_json::from_str(&isotope_fine_route(query).unwrap()).unwrap();
        let want = r["peaks"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{query}: number of peaks");
        for (g, w) in got.iter().zip(want) {
            let (wm, wi) = (w[0].as_f64().unwrap(), w[1].as_f64().unwrap());
            assert!(
                (g[0] - wm).abs() <= 1e-9 * wm.abs().max(1.0),
                "{query}: m/z {} against {wm}",
                g[0]
            );
            assert!(
                (g[1] - wi).abs() <= 1e-6 * wi.max(1e-3),
                "{query}: % {} against {wi}",
                g[1]
            );
        }
    }
    assert!(
        isotope_fine_route("?n=Xx:3").is_err(),
        "an element without isotope data is an error with a key"
    );
}
