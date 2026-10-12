//! The Rust reader of `.wiff` + `.wiff.scan` gives what OpenSXRaw (the oracle, Apache-2.0) gives on the same synthetic pairs
//! (`tools/wiff_riferimento.py`): scans, levels, retention times, points (bin and count), TIC chromatogram, within 1e-9.
//! Skipped without python3 and opensxraw (`MZLAB_GOLDEN_STRICT=1` fails instead). The m/z of a QTRAP is not an oracle quantity: see `docs/agenti/wiff.md`.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::ole::Ole;
use mzlab_core::wiff::{samples, Wiff};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_{}", std::process::id()));
    let py = std::env::var("MZLAB_WIFF_PYTHON").unwrap_or_else(|_| "python3".into());
    let ok = Command::new(py).arg(root().join("tools/wiff_riferimento.py")).arg(&out).output().ok().filter(|o| o.status.success());
    if ok.is_none() {
        assert!(std::env::var("MZLAB_GOLDEN_STRICT").is_err(), "oracle not available");
        eprintln!("SKIP wiff_parity: python3 with opensxraw not available");
        return None;
    }
    let v = serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap();
    Some((out, v))
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()
}

#[test]
fn scans_and_tic_match_the_oracle() {
    let Some((dir, want)) = reference() else { return };
    for name in ["one", "two"] {
        let wiff = std::fs::read(dir.join(format!("{name}.wiff"))).unwrap();
        let mut scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
        let len = scan.len() as u64;
        let all = samples(&Ole::open(wiff.clone(), name).unwrap());
        assert_eq!(all.len(), want[name].as_object().unwrap().len(), "{name}: samples");
        for s in &all {
            let w = Wiff::open(wiff.clone(), name, Some(len), Some(s)).unwrap();
            let r = &want[name][s];
            let scans = r["scans"].as_array().unwrap();
            assert_eq!(w.scans.len(), scans.len(), "{name}/{s}: scans");
            // the oracle writes the chromatogram times as f32 seconds, the spectra times as f64 seconds
            let (t, tic) = w.tic();
            let (wt, wi) = (nums(&r["tic"]["time_sec"]), nums(&r["tic"]["intensity"]));
            for i in 0..scans.len() {
                assert!(close((t[i] as f32 * 60.0f32) as f64, wt[i]) && close(tic[i], wi[i]), "{name}/{s}[{i}]: TIC");
                assert!(close(w.scans[i].rt as f64 * 60.0, scans[i]["rt"].as_f64().unwrap()), "{name}/{s}[{i}]: RT");
                assert_eq!(w.scans[i].level as i64, scans[i]["level"].as_i64().unwrap(), "{name}/{s}[{i}]: level");
                let pts: Vec<_> = w.points(&mut scan, i).unwrap().into_iter().filter(|p| p.count > 0).collect();
                let (mz, it) = (nums(&scans[i]["mz"]), nums(&scans[i]["intensity"]));
                assert_eq!(pts.len(), mz.len(), "{name}/{s}[{i}]: points");
                for (k, p) in pts.iter().enumerate() {
                    assert!(close(p.bin as f64, mz[k]) && close(p.count as f64, it[k]), "{name}/{s}[{i}] point {k}");
                }
            }
        }
    }
}

#[test]
fn a_wiff_without_scan_or_with_several_samples_is_refused() {
    let Some((dir, _)) = reference() else { return };
    let lonely = std::fs::read(dir.join("lonely.wiff")).unwrap();
    assert_eq!(Wiff::open(lonely, "lonely.wiff", None, None).err().unwrap().key, "err.file.wiffNoScan");
    let two = std::fs::read(dir.join("two.wiff")).unwrap();
    assert_eq!(Wiff::open(two, "two.wiff", Some(1), None).err().unwrap().key, "err.file.wiff");
    assert_eq!(Wiff::open(vec![1, 2, 3], "x.wiff", Some(1), None).err().unwrap().key, "err.file.notWiff");
}
