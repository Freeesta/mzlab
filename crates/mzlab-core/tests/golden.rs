//! Parity of the Rust engine with the golden data of the Python engine (`tests/golden/`, `tools/golden.py`).
//! Order: the indices of the peak limits must be identical, then areas (1e-4 relative), m/z (1e-6), entropy (1e-9).
//! A difference in an area is a difference of logic: fix the logic, never the tolerance.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::analysis::{tic, xic};
use mzlab_core::entropy::entropy_similarity;
use mzlab_core::mzml::{read, ChromKind};
use mzlab_core::peaks::{detect_peaks, Params};
use serde::Deserialize;

const TOL_AREA: f64 = 1e-4;
const TOL_MZ: f64 = 1e-6;
const TOL_ENT: f64 = 1e-9;
const MIN_REL_HEIGHT: f64 = 0.05; // as tools/golden.py

#[derive(Deserialize)]
struct GPeak {
    apex_i: usize,
    lo: usize,
    hi: usize,
    area: f64,
}
#[derive(Deserialize)]
struct GTrace {
    rt: Vec<f64>,
    y: Vec<f64>,
    peaks: Vec<GPeak>,
}
#[derive(Deserialize)]
struct GXic {
    mz: f64,
    mz_lo: f64,
    mz_hi: f64,
    #[serde(flatten)]
    t: GTrace,
}
#[derive(Deserialize)]
struct GChrom {
    id: String,
    #[serde(flatten)]
    t: GTrace,
}
#[derive(Deserialize)]
struct GSpec {
    mz: Vec<f64>,
    it: Vec<f64>,
}
#[derive(Deserialize)]
struct GPair {
    a: GSpec,
    b: GSpec,
    entropy: f64,
}
#[derive(Deserialize)]
struct Golden {
    kind: String,
    tic: Option<GTrace>,
    #[serde(default)]
    xic: Vec<GXic>,
    #[serde(default)]
    chromatograms: Vec<GChrom>,
    #[serde(default)]
    entropy_pairs: Vec<GPair>,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn close(a: f64, b: f64, rel: f64, abs: f64) -> bool {
    (a - b).abs() <= abs.max(rel * a.abs().max(b.abs()))
}

fn load(name: &str) -> Golden {
    let p = root().join("tests/golden").join(format!("{name}.json"));
    serde_json::from_reader(File::open(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
        .unwrap()
}

fn check_trace(what: &str, rt: &[f64], y: &[f64], g: &GTrace) {
    assert_eq!(rt.len(), g.rt.len(), "{what}: number of points");
    let top = g.y.iter().fold(1.0_f64, |m, v| m.max(v.abs()));
    for (i, ((a, b), (t, gt))) in y.iter().zip(&g.y).zip(rt.iter().zip(&g.rt)).enumerate() {
        assert!(
            close(*a, *b, TOL_AREA, 1e-9 * top),
            "{what}: y[{i}] {a} vs {b}"
        );
        assert!((t - gt).abs() <= 2e-6, "{what}: rt[{i}] {t} vs {gt}");
    }
    let max = y.iter().cloned().fold(0.0_f64, f64::max);
    let mut got: Vec<_> = detect_peaks(rt, y, Params::default())
        .into_iter()
        .filter(|p| p.height >= MIN_REL_HEIGHT * max)
        .collect();
    got.sort_by_key(|p| p.apex_i);
    let lim = |v: &[(usize, usize, usize)]| format!("{v:?}");
    let a: Vec<_> = got.iter().map(|p| (p.apex_i, p.lo, p.hi)).collect();
    let b: Vec<_> = g.peaks.iter().map(|p| (p.apex_i, p.lo, p.hi)).collect();
    assert_eq!(
        a,
        b,
        "{what}: peak limits differ\n rust   {}\n golden {}",
        lim(&a),
        lim(&b)
    );
    for (p, q) in got.iter().zip(&g.peaks) {
        assert!(
            close(p.area, q.area, TOL_AREA, 0.0),
            "{what}: area of the peak at {} is {} instead of {}",
            p.apex_i,
            p.area,
            q.area
        );
    }
}

fn check_file(name: &str, path: &Path) {
    let g = load(name);
    let run = read(File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap();
    match g.kind.as_str() {
        "full" => {
            let gt = g.tic.as_ref().unwrap();
            let (rt, y) = tic(&run.spectra, 1);
            check_trace(&format!("{name}/tic"), &rt, &y, gt);
            for x in &g.xic {
                assert!(
                    close(x.mz_lo, x.mz - 0.2, 0.0, TOL_MZ)
                        && close(x.mz_hi, x.mz + 0.8, 0.0, TOL_MZ)
                );
                let (rt, y) = xic(&run.spectra, 1, x.mz_lo, x.mz_hi);
                check_trace(&format!("{name}/xic {}", x.mz), &rt, &y, &x.t);
            }
        }
        "ms2" => {
            let (rt, y) = tic(&run.spectra, 2);
            check_trace(&format!("{name}/tic"), &rt, &y, g.tic.as_ref().unwrap());
            for (k, p) in g.entropy_pairs.iter().enumerate() {
                let e = entropy_similarity(&p.a.mz, &p.a.it, &p.b.mz, &p.b.it, 0.5);
                assert!(
                    (e - p.entropy).abs() <= TOL_ENT,
                    "{name}/entropy {k}: {e} vs {}",
                    p.entropy
                );
            }
        }
        "mrm" => {
            let srm: Vec<_> = run
                .chromatograms
                .iter()
                .filter(|c| c.kind == ChromKind::Srm)
                .collect();
            assert_eq!(
                srm.len(),
                g.chromatograms.len(),
                "{name}: number of SRM chromatograms"
            );
            for (c, gc) in srm.iter().zip(&g.chromatograms) {
                assert_eq!(c.id, gc.id);
                check_trace(&format!("{name}/{}", gc.id), &c.time, &c.intensity, &gc.t);
            }
        }
        k => panic!("unknown kind {k}"),
    }
}

#[test]
fn examples_match_the_python_engine() {
    let ex = root().join("mzlab/web/esempi");
    check_file("esempio_FullScan_t10", &ex.join("FullScan_t10.mzML"));
    check_file("esempio_MS2_t15", &ex.join("MS2_t15.mzML"));
    check_file("esempio_MRM_std_2.4ppm", &ex.join("MRM_std_2.4ppm.mzML"));
}

#[test]
fn synthetic_files_match_the_python_engine() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sintetici");
    let ok = Command::new("python3")
        .arg(root().join("tools/golden.py"))
        .arg("--sintetici")
        .arg(&dir)
        .output();
    match ok {
        Ok(o) if o.status.success() => {
            check_file("sintetico_FullScan", &dir.join("sintetico_FullScan.mzML"));
            check_file("sintetico_MRM", &dir.join("sintetico_MRM.mzML"));
        }
        _ => {
            if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
                panic!("python3 with numpy is needed for the synthetic files");
            }
            eprintln!("skipped: python3 with numpy is not available");
        }
    }
}
