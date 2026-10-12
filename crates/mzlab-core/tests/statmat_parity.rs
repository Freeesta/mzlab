//! `statmat` against numpy (`tools/statmat_riferimento.py`): singular values and eigenvalues to 1e-12 (relative to the largest), the signs
//! of the 2x2 eigenvectors exactly. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::Path;
use std::process::Command;

use mzlab_core::statmat::{eigh, svd};
use serde_json::Value;

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn close(got: f64, want: f64, scale: f64) -> bool {
    (got - want).abs() <= 1e-12 * scale.max(1e-300)
}

#[test]
fn statmat_like_numpy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("mzlab-statmat-{}.json", std::process::id()));
    let run = Command::new("python3")
        .current_dir(&root)
        .arg("tools/statmat_riferimento.py")
        .arg(&out)
        .output();
    if !matches!(&run, Ok(o) if o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "the reference did not run: {run:?}"
        );
        eprintln!("python3/numpy not available: test skipped");
        return;
    }
    let j: Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    for c in j["svd"].as_array().unwrap() {
        let (m, n) = (
            c["m"].as_u64().unwrap() as usize,
            c["n"].as_u64().unwrap() as usize,
        );
        let want = nums(&c["s"]);
        let got = svd(&nums(&c["a"]), m, n).s;
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(&want) {
            assert!(close(*g, *w, want[0]), "{m}x{n}: {g} vs {w}");
        }
    }
    for c in j["eigh2"].as_array().unwrap() {
        let (a, b, d) = (
            c["a"].as_f64().unwrap(),
            c["b"].as_f64().unwrap(),
            c["c"].as_f64().unwrap(),
        );
        let (w, v) = eigh(&[a, b, b, d], 2);
        let (want_w, want_v) = (nums(&c["w"]), nums(&c["v"]));
        let scale = want_w[1].abs().max(want_w[0].abs());
        for i in 0..2 {
            assert!(
                close(w[i], want_w[i], scale),
                "w {a} {b} {d}: {w:?} vs {want_w:?}"
            );
        }
        for (g, x) in v.iter().zip(&want_v).skip(1).step_by(2) {
            // the eigenvector of the larger value (column 1), sign included
            assert!((g - x).abs() < 1e-9, "v {a} {b} {d}: {v:?} vs {want_v:?}");
        }
    }
    for c in j["eigh"].as_array().unwrap() {
        let n = c["n"].as_u64().unwrap() as usize;
        let want = nums(&c["w"]);
        let (w, _) = eigh(&nums(&c["a"]), n);
        for (g, x) in w.iter().zip(&want) {
            assert!(
                close(*g, *x, want[n - 1].abs().max(want[0].abs())),
                "eigh {n}: {g} vs {x}"
            );
        }
    }
}
