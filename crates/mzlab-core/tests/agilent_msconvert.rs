//! The Agilent `.D` reader against the mzML that ProteoWizard msconvert made from the same runs (private data repository, `GC-MS/mzML/<run>.mzML` next to
//! `GC-MS/<run>.D`; skipped without it): the spectra one by one (time, level, polarity, points, m/z, intensity) and the TIC chromatogram.
//! Differences that are known and accepted: msconvert names a scan `scanId=N` (here `scan=N`, the form the program reads scan numbers from), writes m/z as
//! shorter decimals than the f32 values of the file (up to 2e-5 apart), and adds one SIM chromatogram per ion (the program builds XICs from the spectra).

use std::path::{Path, PathBuf};

use mzlab_core::agilent::{AgilentD, Files};
use mzlab_core::mzml::{self, Run};

fn dati() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().ok()?;
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI").map(PathBuf::from).into_iter().collect();
    cands.extend(root.ancestors().skip(1).take(3).map(|up| up.join("mzlab-dati")));
    cands.into_iter().find(|p| p.join("GC-MS/mzML").is_dir())
}

fn ours(folder: &Path) -> Run {
    let acq = folder.join("AcqData");
    let rd = |f: &str| std::fs::read(acq.join(f)).ok();
    let (scan, xsd, ts) = (rd("MSScan.bin").unwrap(), rd("MSScan.xsd"), rd("MSTS.xml"));
    let mut peak = std::fs::read(acq.join("MSPeak.bin")).unwrap();
    let files = Files { ms_scan: &scan, ms_scan_xsd: xsd.as_deref(), ms_peak_len: peak.len() as u64, ms_ts: ts.as_deref(), ..Default::default() };
    AgilentD::open(&folder.file_name().unwrap().to_string_lossy(), &files).unwrap().run(&mut peak).unwrap()
}

fn compare(got: &Run, want: &Run, label: &str) -> f64 {
    assert_eq!(got.spectra.len(), want.spectra.len(), "{label}: spectra");
    let mut worst_mz: f64 = 0.0;
    for (k, (g, w)) in got.spectra.iter().zip(&want.spectra).enumerate() {
        assert!((g.rt - w.rt).abs() <= 1e-9, "{label} #{k}: rt {} {}", g.rt, w.rt);
        assert_eq!((g.level, g.polarity, g.mz.len()), (w.level, w.polarity, w.mz.len()), "{label} #{k}: level/polarity/points");
        assert!(g.id.ends_with(&format!("={}", k + 1)) && w.id == format!("scanId={}", k + 1), "{label} #{k}: ids {} {}", g.id, w.id);
        for i in 0..g.mz.len() {
            worst_mz = worst_mz.max((g.mz[i] - w.mz[i]).abs());
            assert!((g.intensity_f64(i) - w.intensity_f64(i)).abs() <= 1e-9, "{label} #{k} point {i}: intensity");
        }
    }
    let (t, u) = (&got.chromatograms[0], want.chromatograms.iter().find(|c| c.id == "TIC").unwrap());
    assert_eq!(t.time.len(), u.time.len(), "{label}: TIC points");
    assert!(t.time.iter().zip(&u.time).all(|(a, b)| (a - b).abs() <= 1e-9) && t.intensity.iter().zip(&u.intensity).all(|(a, b)| (a - b).abs() <= 1e-9), "{label}: TIC");
    worst_mz
}

#[test]
fn real_runs_match_msconvert() {
    let Some(d) = dati() else {
        eprintln!("SKIP agilent_msconvert: no data repository (MZLAB_DATI)");
        return;
    };
    let mut done = 0;
    for e in std::fs::read_dir(d.join("GC-MS")).unwrap().flatten() {
        let folder = e.path();
        let Some(stem) = folder.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".D")).map(String::from) else { continue };
        let Ok(f) = std::fs::File::open(d.join("GC-MS/mzML").join(format!("{stem}.mzML"))) else { continue };
        let want = mzml::read(std::io::BufReader::new(f)).unwrap();
        let worst = compare(&ours(&folder), &want, &stem);
        assert!(worst <= 2e-5, "{stem}: m/z differs by {worst}");
        done += 1;
    }
    assert!(done > 0, "no mzML paired with a .D");
}
