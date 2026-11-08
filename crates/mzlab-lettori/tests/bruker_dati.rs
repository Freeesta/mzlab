//! The Bruker timsTOF reader against the real folders and the mzML that msconvert (ProteoWizard, `combineIonMobilitySpectra`) made of them (private data repository, `MZLAB_DATI`; skipped without it).
#![cfg(feature = "bruker")]

use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::mzml::{self, Item, Spectrum};
use std::path::Path;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

/// The files of a real `.d` folder (not its subfolders) as a `FileSet`.
fn set_of(dir: &Path) -> FileSet {
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let members = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .map(|p| {
            let f = std::fs::File::open(&p).unwrap();
            Membro {
                nome: format!("{name}/{}", p.file_name().unwrap().to_string_lossy()),
                dim: f.metadata().unwrap().len(),
                dati: Box::new(f),
                sha1: None,
            }
        })
        .collect();
    FileSet { membri: members }
}

fn spettri_mzml(p: &Path) -> Vec<Spectrum> {
    let mut out = Vec::new();
    mzml::parse(
        std::io::BufReader::new(std::fs::File::open(p).unwrap()),
        |it| {
            if let Item::Spectrum(s) = it {
                out.push(s);
            }
        },
    )
    .unwrap();
    out
}

fn summed(s: &Spectrum) -> f64 {
    (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum()
}

/// Every point of `rif` has a point of `s` within `ppm`, and the sums are equal.
fn compare(s: &Spectrum, reference: &Spectrum, ppm: f64, label: &str) {
    assert!(
        (summed(s) - summed(reference)).abs() < 0.5,
        "{label}: sum {} vs {}",
        summed(s),
        summed(reference)
    );
    let mut peggiore = 0.0f64;
    for &m in &reference.mz {
        let j = s.mz.partition_point(|&x| x < m);
        let near = [j.checked_sub(1), Some(j)]
            .into_iter()
            .flatten()
            .filter_map(|j| s.mz.get(j))
            .map(|&x| ((x - m) / m).abs() * 1e6)
            .fold(f64::INFINITY, f64::min);
        peggiore = peggiore.max(near);
    }
    assert!(peggiore <= ppm, "{label}: worst point {peggiore} ppm");
    eprintln!("{label}: {} points, worst {peggiore:.2} ppm", s.mz.len());
}

fn compare_folder(folder: &str, reference_mzml: &str) {
    let Some(base) = dati_percorsi::percorso("dati/formati/aperti/lettori_open_test/pwiz/Bruker")
    else {
        eprintln!("SKIP bruker {folder}: no data repository (MZLAB_DATI)");
        return;
    };
    let mut l = mzlab_lettori::apri_nudo(set_of(&base.join(folder)), &Limiti::desktop()).unwrap();
    let reference = spettri_mzml(&base.join(reference_mzml));
    // the copy of the test data keeps only the first 400 kB of `analysis.tdf_bin`: the spectra of the frames past its end leave the index and are counted
    let n = l.index(0).unwrap().len();
    assert_eq!(n + l.missing(0), reference.len(), "{folder}");
    if n < reference.len() {
        eprintln!(
            "{folder}: {} spectra past the end of the cut tdf_bin",
            reference.len() - n
        );
    }
    for (k, r) in reference.iter().enumerate().take(n) {
        let s = l.scan(0, k).unwrap();
        assert_eq!(s.level, r.level, "{folder} {k}");
        assert!((s.rt - r.rt).abs() < 1e-6, "{folder} {k}: rt");
        compare(&s, r, 5.0, &format!("{folder} #{k} {}", s.id));
    }
}

#[test]
fn dda_pasef_come_msconvert() {
    compare_folder(
        "Hela_QC_PASEF_Slot1-first-6-frames.d",
        "Hela_QC_PASEF_Slot1-first-6-frames-combineIMS.mzML",
    );
}

#[test]
fn mrm_come_msconvert() {
    compare_folder("ThyroglobMRM000003.d", "ThyroglobMRM000003-combineIMS.mzML");
}

/// Opens the folder `rel` (under `dati/formati/`) and reads every spectrum (at most `max`): no error, m/z increasing and finite, the TIC of the index equal to the sum of the points
/// for a survey scan. Returns (spectra, levels seen).
fn percorri(rel: &str, max: usize) -> Option<(usize, Vec<u8>)> {
    let Some(dir) = dati_percorsi::percorso(&format!("dati/formati/{rel}")) else {
        eprintln!("SKIP bruker {rel}: not in the data repository");
        return None;
    };
    let mut l = mzlab_lettori::apri_nudo(set_of(&dir), &Limiti::desktop()).unwrap();
    let ix = l.index(0).unwrap().to_vec();
    let mut levels = Vec::new();
    for k in (0..ix.len()).take(max) {
        let s = l.scan(0, k).unwrap();
        assert!(
            s.mz.windows(2).all(|w| w[0] < w[1]) && s.mz.iter().all(|m| m.is_finite() && *m > 0.0),
            "{rel} #{k}: m/z order"
        );
        assert_eq!(s.mz.len(), s.intensity.len());
        if !levels.contains(&s.level) {
            levels.push(s.level);
        }
        // a survey scan carries the sum of the frame table undone from its normalisation to 100 ms (the rounding of the table is 1 count); the others are summed from their points
        let tol = if s.level == 1 {
            2.0 + ix[k].tic * 1e-6
        } else {
            1.0 + ix[k].tic * 1e-9
        };
        assert!(
            (summed(&s) - ix[k].tic).abs() <= tol,
            "{rel} #{k}: TIC {} vs {}",
            ix[k].tic,
            summed(&s)
        );
    }
    eprintln!("{rel}: {} spectra, levels {levels:?}", ix.len());
    Some((ix.len(), levels))
}

#[test]
fn clean_diapasef() {
    let r = percorri(
        "bruker/timstof_dia/asalt_mzio_hela_dia_slice/hela_dia_iildlisespik_rt21_60_21_78.d",
        400,
    );
    if let Some((n, liv)) = r {
        assert!(n > 0 && liv.contains(&1) && liv.contains(&2));
    }
}

#[test]
fn synthetic_timsrust_folders() {
    // the files of timsrust's tests have no MzCalibration: they open, the spectra give an error with a key
    let Some(d) = dati_percorsi::percorso("dati/formati/bruker/timstof_lettori/timsrust_tests")
    else {
        return;
    };
    for f in ["test.d", "dia_test.d"] {
        let dir = d.join(f);
        if !dir.exists() {
            continue;
        }
        let mut l = mzlab_lettori::apri_nudo(set_of(&dir), &Limiti::desktop()).unwrap();
        assert!(!l.index(0).unwrap().is_empty());
        assert_eq!(
            l.scan(0, 0).err().map(|e| e.key),
            Some("err.dam.unreadable")
        );
    }
}
