//! `trace::detect` against the frozen answers of the former Python detection (`tests/golden/feat/detect.json`) on the synthetic files of
//! `tools/feat_sintetici.py`: same number of features, same apex and edge scans, m/z within 1e-6 and areas within 1e-4 (relative), same flags. Also
//! with a small slab (tiny memory budget). Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_feat::table::{FileFeatures, FLAG_BACKGROUND, FLAG_FT_LOBE};
use mzlab_feat::trace::{detect, Params};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_detect(idx: &Path, p: &Params) -> FileFeatures {
    let f = std::fs::File::open(idx).unwrap();
    let size = f.metadata().unwrap().len();
    let mut ix = Index::open(f, size).unwrap();
    detect(&mut ix, p).unwrap()
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn rel(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * a.abs().max(b.abs()).max(1e-300)
}

fn check(name: &str, got: &FileFeatures, want: &Value) {
    let n = want["n"].as_u64().unwrap() as usize;
    assert_eq!(got.len(), n, "{name}: number of features");
    let (mz, area) = (nums(&want["mz"]), nums(&want["area"]));
    let (apex, lo, hi) = (
        nums(&want["apex"]),
        nums(&want["scan_lo"]),
        nums(&want["scan_hi"]),
    );
    let (bg, art) = (nums(&want["background"]), nums(&want["artefact"]));
    let (rt, left, right) = (nums(&want["rt"]), nums(&want["left"]), nums(&want["right"]));
    let npts = nums(&want["npts"]);
    for i in 0..n {
        assert!(
            rel(got.mz[i], mz[i], 1e-6),
            "{name} #{i}: m/z {} against {}",
            got.mz[i],
            mz[i]
        );
        assert_eq!(got.apex[i] as f64, apex[i], "{name} #{i}: apex");
        assert_eq!(got.scan_lo[i] as f64, lo[i], "{name} #{i}: first scan");
        assert_eq!(got.scan_hi[i] as f64, hi[i], "{name} #{i}: last scan");
        assert!(
            rel(got.area[i], area[i], 1e-4),
            "{name} #{i}: area {} against {}",
            got.area[i],
            area[i]
        );
        assert!(rel(got.rt[i] as f64, rt[i], 1e-6), "{name} #{i}: rt");
        assert!(
            rel(got.left[i] as f64, left[i], 1e-6) && rel(got.right[i] as f64, right[i], 1e-6),
            "{name} #{i}: edges"
        );
        assert_eq!(got.npts[i] as f64, npts[i], "{name} #{i}: points");
        assert_eq!(
            got.flags[i] & FLAG_BACKGROUND != 0,
            bg[i] > 0.0,
            "{name} #{i}: background"
        );
        assert_eq!(
            got.flags[i] & FLAG_FT_LOBE != 0,
            art[i] > 0.0,
            "{name} #{i}: side lobe"
        );
    }
}

#[test]
fn detect_like_the_golden_file() {
    let out = std::env::temp_dir().join(format!("mzlab_feat_{}", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/feat_sintetici.py"))
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
    let golden: Value = serde_json::from_slice(
        &std::fs::read(root().join("tests/golden/feat/detect.json")).unwrap(),
    )
    .unwrap();
    let mut total = 0;
    let mut lobes = 0;
    let mut bg = 0;
    for name in ["deboli", "intensi", "fondo", "vicini", "vuoto"] {
        let want = &golden[name];
        let mut p = Params::default();
        if let Some(l) = want["params"]["long_trace"].as_u64() {
            p.long_trace = l as u32;
        }
        let idx = out.join(format!("{name}.mzidx"));
        let big = run_detect(&idx, &p);
        check(name, &big, want);
        // a slab of a thousand peaks: the same table
        let small = run_detect(
            &idx,
            &Params {
                budget: 1,
                ..p.clone()
            },
        );
        assert_eq!(big, small, "{name}: small slabs");
        assert_eq!(
            big.to_bytes(),
            run_detect(&idx, &p).to_bytes(),
            "{name}: second run"
        );
        total += big.len();
        lobes += big.flags.iter().filter(|f| *f & FLAG_FT_LOBE != 0).count();
        bg += big
            .flags
            .iter()
            .filter(|f| *f & FLAG_BACKGROUND != 0)
            .count();
    }
    let _ = std::fs::remove_dir_all(&out);
    assert!(total >= 50, "the synthetic files are too poor: {total}");
    assert!(lobes >= 1 && bg >= 1, "lobes {lobes}, background {bg}");
}
