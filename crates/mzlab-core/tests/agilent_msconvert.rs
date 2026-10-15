//! The Agilent `.D` reader against the mzML that ProteoWizard msconvert made from the same runs (private data repository, `GC-MS/mzML/<run>.mzML` next to
//! `GC-MS/<run>.D`; skipped without it): the spectra one by one (time, level, polarity, points, m/z, intensity) and the TIC chromatogram.
//! Differences that are known and accepted: msconvert names a scan `scanId=N` (here `scan=N`, the form the program reads scan numbers from), writes m/z as
//! shorter decimals than the f32 values of the file (up to 2e-5 apart), and adds one SIM chromatogram per ion (the program builds XICs from the spectra).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};

use mzlab_core::agilent::{AgilentD, Files};
use mzlab_core::mzml::{self, Run};

fn dati() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .ok()?;
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    cands.extend(
        root.ancestors()
            .skip(1)
            .take(3)
            .map(|up| up.join("mzlab-dati")),
    );
    cands.into_iter().find(|p| p.join("GC-MS/mzML").is_dir())
}

fn ours(folder: &Path) -> Run {
    let acq = folder.join("AcqData");
    let rd = |f: &str| std::fs::read(acq.join(f)).ok();
    let (scan, xsd, ts) = (rd("MSScan.bin").unwrap(), rd("MSScan.xsd"), rd("MSTS.xml"));
    let mut peak = std::fs::read(acq.join("MSPeak.bin")).unwrap();
    let files = Files {
        ms_scan: &scan,
        ms_scan_xsd: xsd.as_deref(),
        ms_peak_len: peak.len() as u64,
        ms_ts: ts.as_deref(),
        ..Default::default()
    };
    AgilentD::open(&folder.file_name().unwrap().to_string_lossy(), &files)
        .unwrap()
        .run(&mut peak)
        .unwrap()
}

fn compare(got: &Run, want: &Run, label: &str) -> f64 {
    assert_eq!(got.spectra.len(), want.spectra.len(), "{label}: spectra");
    let mut worst_mz: f64 = 0.0;
    for (k, (g, w)) in got.spectra.iter().zip(&want.spectra).enumerate() {
        assert!(
            (g.rt - w.rt).abs() <= 1e-9,
            "{label} #{k}: rt {} {}",
            g.rt,
            w.rt
        );
        assert_eq!(
            (g.level, g.polarity, g.mz.len()),
            (w.level, w.polarity, w.mz.len()),
            "{label} #{k}: level/polarity/points"
        );
        assert!(
            g.id.ends_with(&format!("={}", k + 1)) && w.id == format!("scanId={}", k + 1),
            "{label} #{k}: ids {} {}",
            g.id,
            w.id
        );
        for i in 0..g.mz.len() {
            worst_mz = worst_mz.max((g.mz[i] - w.mz[i]).abs());
            assert!(
                (g.intensity_f64(i) - w.intensity_f64(i)).abs() <= 1e-9,
                "{label} #{k} point {i}: intensity"
            );
        }
    }
    let (t, u) = (
        &got.chromatograms[0],
        want.chromatograms.iter().find(|c| c.id == "TIC").unwrap(),
    );
    assert_eq!(t.time.len(), u.time.len(), "{label}: TIC points");
    assert!(
        t.time
            .iter()
            .zip(&u.time)
            .all(|(a, b)| (a - b).abs() <= 1e-9)
            && t.intensity
                .iter()
                .zip(&u.intensity)
                .all(|(a, b)| (a - b).abs() <= 1e-9),
        "{label}: TIC"
    );
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
        let Some(stem) = folder
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".D"))
            .map(String::from)
        else {
            continue;
        };
        let Ok(f) = std::fs::File::open(d.join("GC-MS/mzML").join(format!("{stem}.mzML"))) else {
            continue;
        };
        let want = mzml::read(std::io::BufReader::new(f)).unwrap();
        let worst = compare(&ours(&folder), &want, &stem);
        assert!(worst <= 2e-5, "{stem}: m/z differs by {worst}");
        done += 1;
    }
    assert!(done > 0, "no mzML paired with a .D");
}

fn lc_dir() -> Option<PathBuf> {
    dati()
        .map(|d| d.join("dati_formati/agilent_D"))
        .or_else(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let mut c: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
                .map(PathBuf::from)
                .into_iter()
                .collect();
            c.extend(
                root.ancestors()
                    .skip(1)
                    .take(3)
                    .map(|u| u.join("mzlab-dati")),
            );
            c.into_iter()
                .map(|p| p.join("dati_formati/agilent_D"))
                .find(|p| p.is_dir())
        })
}

/// The profile spectra of a Q-TOF run (`MSProfile.bin`: flight times, and the mass calibration of each scan) against the profile mzML of msconvert. The two are not
/// the same points: msconvert writes a thinned profile (37 000 of 293 000 samples, zeros dropped) with intensities it interpolated. What must agree: the scans (time,
/// level, polarity), and for the strongest points of msconvert the nearest sample of ours, within 12 ppm in m/z and 10 % in height.
#[cfg(feature = "estesi")]
#[test]
fn qtof_profile_run_matches_msconvert() {
    let Some(root) = lc_dir() else {
        eprintln!("SKIP agilent_msconvert: no data repository (MZLAB_DATI)");
        return;
    };
    let mut done = 0;
    for dir in std::fs::read_dir(&root).unwrap().flatten() {
        let Some(run) = std::fs::read_dir(dir.path()).ok().and_then(|it| {
            it.flatten().map(|e| e.path()).find(|p| {
                p.extension().is_some_and(|e| e == "d") && p.join("AcqData/MSProfile.bin").exists()
            })
        }) else {
            continue;
        };
        let Ok(f) = std::fs::File::open(run.with_extension("mzML")) else {
            continue;
        };
        let want = mzml::read(std::io::BufReader::new(f)).unwrap();
        let acq = run.join("AcqData");
        let rd = |f: &str| std::fs::read(acq.join(f)).ok();
        let (scan, xsd, ts) = (rd("MSScan.bin").unwrap(), rd("MSScan.xsd"), rd("MSTS.xml"));
        let mut peak = std::fs::read(acq.join("MSProfile.bin")).unwrap();
        let files = Files {
            ms_scan: &scan,
            ms_scan_xsd: xsd.as_deref(),
            ms_peak_len: peak.len() as u64,
            ms_ts: ts.as_deref(),
            ..Default::default()
        };
        let got = AgilentD::open(&run.file_name().unwrap().to_string_lossy(), &files)
            .unwrap()
            .with_calibration(
                rd("MSMassCal.bin").as_deref(),
                rd("DefaultMassCal.xml").as_deref(),
            )
            .run(&mut peak)
            .unwrap();
        assert_eq!(got.spectra.len(), want.spectra.len(), "spectra");
        let mut worst_ppm = 0.0f64;
        for (k, (g, w)) in got.spectra.iter().zip(&want.spectra).enumerate() {
            assert!((g.rt - w.rt).abs() <= 1e-9, "#{k}: rt");
            assert_eq!((g.level, g.polarity), (w.level, w.polarity), "#{k}");
            assert!(g.mz.windows(2).all(|p| p[0] <= p[1]), "#{k}: m/z order");
            let mut idx: Vec<usize> = (0..w.mz.len()).collect();
            idx.sort_by(|&a, &b| w.intensity_f64(b).total_cmp(&w.intensity_f64(a)));
            for &i in idx.iter().take(6) {
                let j = (0..g.mz.len())
                    .min_by(|&a, &b| {
                        (g.mz[a] - w.mz[i])
                            .abs()
                            .total_cmp(&(g.mz[b] - w.mz[i]).abs())
                    })
                    .unwrap();
                let ppm = (g.mz[j] - w.mz[i]).abs() / w.mz[i] * 1e6;
                worst_ppm = worst_ppm.max(ppm);
                assert!(
                    ppm <= 12.0,
                    "#{k}: {} is {ppm} ppm from our nearest",
                    w.mz[i]
                );
                // the sample nearest to msconvert's apex is on the same peak: its height is within 10 % of it
                let b = w.intensity_f64(i);
                let near = (j.saturating_sub(8)..(j + 9).min(g.mz.len()))
                    .map(|q| g.intensity_f64(q))
                    .fold(0.0, f64::max);
                assert!(near >= 0.9 * b, "#{k}: height {near} against {b}");
            }
        }
        eprintln!(
            "{}: {} spectra, strongest points within {worst_ppm:.1} ppm",
            run.display(),
            got.spectra.len()
        );
        done += 1;
    }
    assert!(done > 0, "no profile run with an mzML");
}
