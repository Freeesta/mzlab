//! The Bruker timsTOF reader against the real folders and the mzML that msconvert (ProteoWizard, `combineIonMobilitySpectra`) made of them (private data repository, `MZLAB_DATI`; skipped without it).
#![cfg(feature = "bruker")]

use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::mzml::{self, Item, Spectrum};
use std::path::{Path, PathBuf};

fn dati() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    let qui = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for up in qui.ancestors().skip(1).take(3) {
        c.push(up.join("mzlab-dati"));
    }
    if let Some(h) = std::env::var_os("HOME") {
        c.push(PathBuf::from(h).join("mzlab-dati"));
    }
    c.into_iter()
        .find(|p| p.join("dati_formati/INDICE.md").exists())
}

/// The files of a real `.d` folder (not its subfolders) as a `FileSet`.
fn set_of(dir: &Path) -> FileSet {
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let membri = std::fs::read_dir(dir)
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
            }
        })
        .collect();
    FileSet { membri }
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

fn somma(s: &Spectrum) -> f64 {
    (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum()
}

/// Every point of `rif` has a point of `s` within `ppm`, and the sums are equal.
fn confronta(s: &Spectrum, rif: &Spectrum, ppm: f64, etichetta: &str) {
    assert!(
        (somma(s) - somma(rif)).abs() < 0.5,
        "{etichetta}: sum {} vs {}",
        somma(s),
        somma(rif)
    );
    let mut peggiore = 0.0f64;
    for &m in &rif.mz {
        let j = s.mz.partition_point(|&x| x < m);
        let vicino = [j.checked_sub(1), Some(j)]
            .into_iter()
            .flatten()
            .filter_map(|j| s.mz.get(j))
            .map(|&x| ((x - m) / m).abs() * 1e6)
            .fold(f64::INFINITY, f64::min);
        peggiore = peggiore.max(vicino);
    }
    assert!(peggiore <= ppm, "{etichetta}: worst point {peggiore} ppm");
    eprintln!(
        "{etichetta}: {} points, worst {peggiore:.2} ppm",
        s.mz.len()
    );
}

fn confronta_cartella(cartella: &str, mzml_rif: &str) {
    let Some(d) = dati() else {
        eprintln!("SKIP bruker {cartella}: no data repository (MZLAB_DATI)");
        return;
    };
    let base = d.join("dati_formati/lettori_open_test/pwiz/Bruker");
    let mut l = mzlab_lettori::apri_nudo(set_of(&base.join(cartella)), &Limiti::desktop()).unwrap();
    let rif = spettri_mzml(&base.join(mzml_rif));
    assert_eq!(l.indice(0).unwrap().len(), rif.len());
    for (k, r) in rif.iter().enumerate() {
        let s = match l.scansione(0, k) {
            Ok(s) => s,
            // the copy of the test data keeps only the first 400 kB of `analysis.tdf_bin`: the MS1 frame at its end is cut
            Err(e) => {
                assert!(
                    k + 1 == rif.len() && format!("{e:?}").contains("block length"),
                    "{cartella} {k}: {e:?}"
                );
                eprintln!("{cartella} #{k}: block cut by the file ({e:?})");
                continue;
            }
        };
        assert_eq!(s.level, r.level, "{cartella} {k}");
        assert!((s.rt - r.rt).abs() < 1e-6, "{cartella} {k}: rt");
        confronta(&s, r, 5.0, &format!("{cartella} #{k} {}", s.id));
    }
}

#[test]
fn dda_pasef_come_msconvert() {
    confronta_cartella(
        "Hela_QC_PASEF_Slot1-first-6-frames.d",
        "Hela_QC_PASEF_Slot1-first-6-frames-combineIMS.mzML",
    );
}

#[test]
fn mrm_come_msconvert() {
    confronta_cartella("ThyroglobMRM000003.d", "ThyroglobMRM000003-combineIMS.mzML");
}

/// Opens the folder `rel` (under `dati_formati/`) and reads every spectrum (at most `max`): no error, m/z increasing and finite, the TIC of the index equal to the sum of the points
/// for a survey scan. Returns (spectra, levels seen).
fn percorri(rel: &str, max: usize) -> Option<(usize, Vec<u8>)> {
    let d = dati()?;
    let dir = d.join("dati_formati").join(rel);
    if !dir.exists() {
        eprintln!("SKIP bruker {rel}: not in the data repository");
        return None;
    }
    let mut l = mzlab_lettori::apri_nudo(set_of(&dir), &Limiti::desktop()).unwrap();
    let ix = l.indice(0).unwrap().to_vec();
    let mut livelli = Vec::new();
    for k in (0..ix.len()).take(max) {
        let s = l.scansione(0, k).unwrap();
        assert!(
            s.mz.windows(2).all(|w| w[0] < w[1]) && s.mz.iter().all(|m| m.is_finite() && *m > 0.0),
            "{rel} #{k}: m/z order"
        );
        assert_eq!(s.mz.len(), s.intensity.len());
        if !livelli.contains(&s.level) {
            livelli.push(s.level);
        }
        // a survey scan carries the sum of the frame table undone from its normalisation to 100 ms (the rounding of the table is 1 count); the others are summed from their points
        let tol = if s.level == 1 {
            2.0 + ix[k].tic * 1e-6
        } else {
            1.0 + ix[k].tic * 1e-9
        };
        assert!(
            (somma(&s) - ix[k].tic).abs() <= tol,
            "{rel} #{k}: TIC {} vs {}",
            ix[k].tic,
            somma(&s)
        );
    }
    eprintln!("{rel}: {} spectra, levels {livelli:?}", ix.len());
    Some((ix.len(), livelli))
}

#[test]
fn diapasef_pulita() {
    let r = percorri(
        "bruker_timstof_dia/asalt_mzio_hela_dia_slice/hela_dia_iildlisespik_rt21_60_21_78.d",
        400,
    );
    if let Some((n, liv)) = r {
        assert!(n > 0 && liv.contains(&1) && liv.contains(&2));
    }
}

#[test]
fn cartelle_sintetiche_di_timsrust() {
    // the files of timsrust's tests have no MzCalibration: they open, the spectra give an error with a key
    let Some(d) = dati() else { return };
    for f in ["test.d", "dia_test.d"] {
        let dir = d
            .join("dati_formati/bruker_timstof_lettori/timsrust_tests")
            .join(f);
        if !dir.exists() {
            continue;
        }
        let mut l = mzlab_lettori::apri_nudo(set_of(&dir), &Limiti::desktop()).unwrap();
        assert!(!l.indice(0).unwrap().is_empty());
        assert_eq!(
            l.scansione(0, 0).err().map(|e| e.key),
            Some("err.dam.unreadable")
        );
    }
}
