//! A `.raw` scan that has a centroid list comes out as centroids (`Spectrum::profile` false, every peak of the list kept), also when the acquisition is
//! profile ("FTMS + p"); a profile scan without a list comes out as its profile. Skipped without `MZLAB_DATI` (private data repository).

#![allow(clippy::disallowed_methods)] // the file names are sorted with the standard sort

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use opentfraw::mzml::extract_spectrum;
use opentfraw::ScanMode;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn raws() -> Vec<PathBuf> {
    let Some(dir) = dati_percorsi::percorso("dati/formati") else {
        return Vec::new();
    };
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("raw")))
        .filter(|p| p.metadata().is_ok_and(|m| m.len() < 400_000_000))
        .collect();
    v.sort();
    v
}

#[test]
fn centroid_lists_are_not_marked_as_profile() {
    let files = raws();
    if files.is_empty() {
        eprintln!("SKIP raw_centroidi: no MZLAB_DATI/dati/formati/*.raw");
        return;
    }
    for path in files {
        let mut src = BufReader::new(File::open(&path).unwrap());
        let raw = opentfraw::RawFileReader::open(&mut src).unwrap();
        let n = raw.num_scans.min(200);
        for idx in 0..n {
            let Some(rec) = extract_spectrum(&raw, &mut src, idx, false) else {
                continue;
            };
            let list = raw
                .read_peaks_only(&mut src, rec.scan_number)
                .map(|p| p.len())
                .unwrap_or(0);
            if list > 0 {
                assert_eq!(
                    rec.scan_mode,
                    Some(ScanMode::Centroid),
                    "{path:?} scan {idx}"
                );
                assert_eq!(
                    rec.mz.len(),
                    list,
                    "{path:?} scan {idx}: every centroid kept"
                );
            }
        }
    }
}
