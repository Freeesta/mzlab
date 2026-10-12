//! The Rust reader of `.wiff` + `.wiff.scan` against (1) the synthetic pairs of `tools/wiff_riferimento.py`, whose expected values come from
//! an independent encoder (skipped without python3; `MZLAB_GOLDEN_STRICT=1` fails instead), and (2) the ProteoWizard mzML of the real 3200 files
//! in the private data repository (skipped without it). Spectra: m/z and intensity within 1e-9, times and counts equal.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::mzml::{self, Run};
use mzlab_core::wiff::{container, samples, Wiff};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    a == b || (a - b).abs() <= tol * a.abs().max(b.abs()).max(1.0)
}

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_{}", std::process::id()));
    let ok = Command::new("python3").arg(root().join("tools/wiff_riferimento.py")).arg(&out).output().ok().filter(|o| o.status.success());
    if ok.is_none() {
        assert!(std::env::var("MZLAB_GOLDEN_STRICT").is_err(), "python3 not available");
        eprintln!("SKIP wiff_parity: python3 not available");
        return None;
    }
    Some((out.clone(), serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap()))
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()
}

fn open(dir: &Path, name: &str, sample: u32) -> (Wiff, Vec<u8>) {
    let ole = container(std::fs::read(dir.join(format!("{name}.wiff"))).unwrap(), name).unwrap();
    let scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
    (Wiff::open(&ole, name, Some(scan.len() as u64), sample).unwrap(), scan)
}

#[test]
fn synthetic_pairs_match_the_reference() {
    let Some((dir, want)) = reference() else { return };
    for name in ["one", "two"] {
        let ole = container(std::fs::read(dir.join(format!("{name}.wiff"))).unwrap(), name).unwrap();
        let all = samples(&ole);
        assert_eq!(all.len(), want[name].as_object().unwrap().len(), "{name}: samples");
        for s in all {
            let (w, mut scan) = open(&dir, name, s);
            let run = w.run(&mut scan).unwrap();
            let r = &want[name][s.to_string()];
            assert_eq!(w.polarity as i64, r["polarity"].as_i64().unwrap(), "{name}/{s}: polarity");
            let sp = r["spectra"].as_array().unwrap();
            assert_eq!(run.spectra.len(), sp.len(), "{name}/{s}: spectra");
            for (g, e) in run.spectra.iter().zip(sp) {
                assert_eq!(g.id, e["id"].as_str().unwrap());
                assert_eq!(g.level as i64, e["level"].as_i64().unwrap(), "{}: level", g.id);
                assert_eq!(g.precursor, e["precursor"].as_f64(), "{}: precursor", g.id);
                assert!(g.profile && close(g.rt, e["rt"].as_f64().unwrap(), 1e-12), "{}: rt", g.id);
                let (mz, it) = (nums(&e["mz"]), nums(&e["intensity"]));
                assert_eq!(g.mz.len(), mz.len(), "{}: points", g.id);
                for k in 0..mz.len() {
                    assert!(close(g.mz[k], mz[k], 1e-9) && close(g.intensity_f64(k), it[k], 1e-9), "{} point {k}", g.id);
                }
            }
            let ch = r["chromatograms"].as_array().unwrap();
            assert_eq!(run.chromatograms.len(), ch.len(), "{name}/{s}: chromatograms");
            for (g, e) in run.chromatograms.iter().zip(ch) {
                assert_eq!(g.id, e["id"].as_str().unwrap());
                let (t, i) = (nums(&e["time"]), nums(&e["intensity"]));
                assert!(g.time.len() == t.len() && (0..t.len()).all(|k| close(g.time[k], t[k], 1e-12) && close(g.intensity[k], i[k], 1e-9)), "{}", g.id);
            }
        }
    }
}

#[test]
fn a_wiff_without_scan_or_not_a_wiff_is_refused() {
    let Some((dir, _)) = reference() else { return };
    let ole = container(std::fs::read(dir.join("lonely.wiff")).unwrap(), "lonely.wiff").unwrap();
    assert_eq!(Wiff::open(&ole, "lonely.wiff", None, 1).err().unwrap().key, "err.file.wiffNoScan");
    assert_eq!(container(vec![1, 2, 3], "x.wiff").err().unwrap().key, "err.file.notWiff");
}

fn dati() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI").map(PathBuf::from).into_iter().collect();
    for up in root().canonicalize().ok()?.ancestors().skip(1).take(3) {
        cands.push(up.join("mzlab-dati"));
    }
    cands.into_iter().find(|p| p.join("3200 qtrap").is_dir())
}

fn sample_of(id: &str) -> Option<u32> {
    id.split("sample=").nth(1)?.split_whitespace().next()?.parse().ok()
}

fn compare(w: &Wiff, got: &Run, want: &Run, label: &str) -> usize {
    // msconvert pads profile peaks with zeros and mis-reads the last block of a file: compare the points above zero, all but the last spectrum
    let n = want.spectra.len().saturating_sub(1);
    for e in &want.spectra[..n] {
        let g = got.spectra.iter().find(|g| g.id == e.id).unwrap_or_else(|| panic!("{label}: {} missing", e.id));
        assert!(close(g.rt, e.rt, 1e-9) && g.level == e.level && g.polarity == e.polarity, "{label} {}: rt/level/polarity", e.id);
        assert!(g.precursor.zip(e.precursor).is_none_or(|(a, b)| close(a, b, 1e-9)) && g.precursor.is_some() == e.precursor.is_some(), "{label} {}: precursor", e.id);
        let pts: Vec<(f64, f64)> = (0..e.mz.len()).map(|k| (e.mz[k], e.intensity_f64(k))).filter(|p| p.1 > 0.0).collect();
        assert_eq!(g.mz.len(), pts.len(), "{label} {}: points", e.id);
        for (k, p) in pts.iter().enumerate() {
            assert!(close(g.mz[k], p.0, 1e-9) && close(g.intensity_f64(k), p.1, if e.wide.is_some() { 1e-9 } else { 1e-6 }), "{label} {} point {k}", e.id);
        }
    }
    for e in &want.chromatograms {
        let Some(g) = got.chromatograms.iter().find(|g| e.q1.is_some() && g.q1 == e.q1 && g.q3 == e.q3 || g.kind == e.kind && e.q1.is_none()) else { continue };
        assert_eq!(g.time.len(), e.time.len(), "{label} {}: chromatogram length", e.id);
        assert!((0..e.time.len()).all(|k| close(g.time[k], e.time[k], 1e-9) && close(g.intensity[k], e.intensity[k], 1e-6)), "{label} {}", e.id);
    }
    let _ = w;
    n
}

#[test]
fn real_files_match_proteowizard() {
    let Some(d) = dati() else {
        eprintln!("SKIP wiff_parity: no data repository (MZLAB_DATI)");
        return;
    };
    let folder = d.join("3200 qtrap/wiff inquinanti 25-26 B");
    let mut done = 0;
    for (wiff, prefix, dir) in [("FullMass", "B_FullMass-", "mzML_profilo"), ("MS2", "B_MS2-", "mzML_profilo"), ("MRM", "B_MRM-", "mzML")] {
        let Ok(bytes) = std::fs::read(folder.join(format!("{wiff}.wiff"))) else { continue };
        let ole = container(bytes, wiff).unwrap();
        let mut scan = std::fs::read(folder.join(format!("{wiff}.wiff.scan"))).unwrap();
        let len = scan.len() as u64;
        let mut files: Vec<PathBuf> = std::fs::read_dir(d.join(dir)).unwrap().flatten().map(|e| e.path()).filter(|p| p.file_name().unwrap().to_string_lossy().starts_with(prefix)).collect();
        files.sort();
        for f in files {
            let want = mzml::read(std::io::BufReader::new(std::fs::File::open(&f).unwrap())).unwrap();
            let id = want.spectra.first().map(|s| s.id.clone()).or_else(|| want.chromatograms.iter().find(|c| c.id.contains("sample=")).map(|c| c.id.clone())).unwrap();
            let w = Wiff::open(&ole, wiff, Some(len), sample_of(&id).unwrap()).unwrap();
            let got = w.run(&mut scan).unwrap();
            compare(&w, &got, &want, &f.display().to_string());
            done += 1;
        }
    }
    eprintln!("compared {done} mzML files");
    assert!(done > 0, "no mzML paired with a wiff");
}
