//! `centroidi` answers like `tools/centroidi_riferimento.py`: same number of centroids, same flags, m/z and values within 1e-9 relative, and the same
//! estimated parameters. Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::centroidi::{centroids, estimate, Buf, Mode, Params};
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
fn params(v: &Value) -> Params {
    let mode = if v[0] == "counts" {
        Mode::Counts
    } else {
        Mode::Continuous
    };
    Params {
        mode,
        unit: v[1].as_f64().unwrap(),
        step: v[2].as_f64().unwrap(),
        w: v[3].as_f64().unwrap(),
        k: v[4].as_f64().unwrap(),
        vmax: v[5].as_f64().unwrap(),
        area: v[6].as_bool().unwrap(),
    }
}

#[test]
fn centroids_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab_centroidi_{}.json", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/centroidi_riferimento.py"))
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
    let all: Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let _ = std::fs::remove_file(&out);
    let mut buf = Buf::default();
    let mut total = 0;
    for (r, row) in all["scans"].as_array().unwrap().iter().enumerate() {
        let got = centroids(
            &nums(&row["mz"]),
            &nums(&row["y"]),
            &params(&row["params"]),
            &mut buf,
        );
        let want = row["centroids"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "scan {r}: count");
        total += got.len();
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            let w = nums(w);
            assert!(
                close(g.mz, w[0])
                    && close(g.value, w[1])
                    && close(g.area, w[2])
                    && close(g.height, w[3])
                    && g.flags as f64 == w[4],
                "scan {r} centroid {i}: {g:?} against {w:?}"
            );
        }
    }
    assert!(total > 100, "{total} centroids only");
    for (r, row) in all["estimates"].as_array().unwrap().iter().enumerate() {
        let scans: Vec<(Vec<f64>, Vec<f64>)> = row["scans"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| (nums(&s["mz"]), nums(&s["y"])))
            .collect();
        let refs: Vec<(&[f64], &[f64])> = scans
            .iter()
            .map(|(a, b)| (a.as_slice(), b.as_slice()))
            .collect();
        let e = estimate(&refs, 0.7);
        let w = params(&row["params"]);
        assert!(
            e.params.mode == w.mode
                && close(e.params.unit, w.unit)
                && close(e.params.step, w.step)
                && close(e.params.w, w.w)
                && close(e.params.vmax, w.vmax)
                && e.params.area == w.area,
            "estimate {r}: {:?} against {w:?}",
            e.params
        );
        assert_eq!(
            (e.regular, e.peaks),
            (
                row["regular"].as_bool().unwrap(),
                row["peaks"].as_u64().unwrap() as usize
            ),
            "estimate {r}"
        );
    }
}
