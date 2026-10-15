#![cfg(feature = "thermo")]
//! `.raw` against the mzML msconvert made from the same file (m/z 1e-6, intensity 1e-4, relative). Needs the private data
//! repository (MZLAB_DATI, or `mzlab-dati` beside this one): without it the test is skipped; no `.raw` lives in this repository.
#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use mzlab_core::{mzml, raw};

fn dati() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for up in here.ancestors().skip(1).take(3) {
        cands.push(up.join("mzlab-dati"));
        cands.push(
            up.parent()
                .map(|p| p.join("mzlab-dati"))
                .unwrap_or_default(),
        );
    }
    cands.into_iter().find(|p| p.join("HRMS").is_dir())
}

fn scan_of(id: &str) -> Option<u32> {
    id.split("scan=")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[test]
fn raw_matches_msconvert_mzml() {
    let Some(d) = dati() else {
        eprintln!("skipped: no data repository (MZLAB_DATI)");
        return;
    };
    let Some(rawp) = std::fs::read_dir(d.join("HRMS"))
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|e| std::fs::read_dir(e.path()).into_iter().flatten().flatten())
        .map(|e| e.path())
        .find(|p| {
            p.extension().is_some_and(|x| x == "raw")
                && p.with_extension("mzML").is_file()
                && !p.to_string_lossy().contains("Profile raw")
        })
    // the profile .raw has its own test (raw_profile_parity.rs)
    else {
        eprintln!("skipped: no .raw with its mzML in HRMS");
        return;
    };
    let mzp = rawp.with_extension("mzML");
    let from_raw = raw::read(BufReader::new(File::open(&rawp).unwrap())).unwrap();
    let reference = mzml::read(BufReader::new(File::open(&mzp).unwrap())).unwrap();
    assert_eq!(
        from_raw.spectra.len(),
        reference.spectra.len(),
        "number of scans"
    );
    let by_scan: std::collections::HashMap<u32, &mzml::Spectrum> = reference
        .spectra
        .iter()
        .filter_map(|s| scan_of(&s.id).map(|n| (n, s)))
        .collect();
    let (mut worst_mz, mut worst_y) = (0.0f64, 0.0f64);
    for s in &from_raw.spectra {
        let r = by_scan[&scan_of(&s.id).unwrap()];
        assert_eq!(s.level, r.level, "level of {}", s.id);
        assert_eq!(s.polarity, r.polarity, "polarity of {}", s.id);
        assert!((s.rt - r.rt).abs() < 1e-4, "rt of {}", s.id);
        assert_eq!(s.mz.len(), r.mz.len(), "peaks of {}", s.id);
        if s.level > 1 {
            assert!(
                s.precursor.is_some() == r.precursor.is_some(),
                "precursor of {}",
                s.id
            );
        }
        for i in 0..s.mz.len() {
            worst_mz = worst_mz.max((s.mz[i] - r.mz[i]).abs() / r.mz[i]);
            worst_y = worst_y
                .max(((s.intensity[i] - r.intensity[i]).abs() / r.intensity[i].max(1e-12)) as f64);
        }
    }
    eprintln!(
        "peaks compared: {}",
        from_raw.spectra.iter().map(|s| s.mz.len()).sum::<usize>()
    );
    eprintln!("worst relative error: m/z {worst_mz:.2e}, intensity {worst_y:.2e}");
    assert!(worst_mz <= 1e-6 && worst_y <= 1e-4);

    // the mzML written from the .raw reads back to the same spectra
    let mut buf = Vec::new();
    raw::to_mzml(File::open(&rawp).unwrap(), &mut buf, "example.raw").unwrap();
    let back = mzml::read(&buf[..]).unwrap();
    assert_eq!(back.spectra.len(), from_raw.spectra.len());
    assert_eq!(back.spectra[4000].mz, from_raw.spectra[4000].mz);
}

#[test]
fn not_a_raw_file_is_an_error_with_a_key() {
    let e = raw::read(std::io::Cursor::new(vec![0u8; 100])).unwrap_err();
    assert_eq!(e.key, "err.file.raw");
}

#[test]
fn the_instrument_method_text_of_a_raw_is_text() {
    let Some(d) = dati() else {
        eprintln!("skipped: no data repository (MZLAB_DATI)");
        return;
    };
    for sub in ["Esempio raw e mzML", "Profile raw"] {
        let Some(rawp) = std::fs::read_dir(d.join("HRMS").join(sub))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "raw"))
        else {
            continue;
        };
        let t = raw::instrument_method_text(File::open(&rawp).unwrap())
            .unwrap()
            .expect("a .raw with a method has its text");
        // blocks of Latin text (the LC method, the instrument summary), not a binary region read as other scripts
        assert!(
            t.contains("---- Overview ----"),
            "{sub}: the LC method overview is missing"
        );
        assert!(
            t.chars().count() < 100_000 && t.chars().all(|c| (c as u32) < 0x2100),
            "{sub}: the text looks like noise"
        );
    }
}
