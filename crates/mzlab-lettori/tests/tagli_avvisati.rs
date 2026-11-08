//! A synthetic file cut short at every length gives one keyed error, or the scans it holds whole with the lost ones counted by `missing`: never fewer
//! scans, or a scan read short, unsaid. One base per reader that holds its file in one container (Shimadzu `.qgd` and `.lcd`).
#![cfg(feature = "shimadzu")]

mod comune;

use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::mzml::Spectrum;

fn fs(name: &str, b: &[u8]) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: name.into(),
            dim: b.len() as u64,
            dati: Box::new(b.to_vec()),
            sha1: None,
        }],
    }
}

/// The scans of the first sample and its missing count, or the key of the first error.
fn read(name: &str, b: &[u8]) -> Result<(Vec<Spectrum>, usize), &'static str> {
    let mut l = mzlab_lettori::apri(fs(name, b), &Limiti::sito()).map_err(|e| e.key)?;
    if l.samples().is_empty() {
        return Ok((Vec::new(), 0));
    }
    let n = l.index(0).map_err(|e| e.key)?.len();
    let scans = (0..n)
        .map(|k| l.scan(0, k))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.key)?;
    Ok((scans, l.missing(0)))
}

fn every_cut_is_told(name: &str, file: &[u8]) {
    let (want, m) = read(name, file).unwrap();
    assert!(!want.is_empty() && m == 0, "{name}: the whole file");
    for cut in (0..file.len()).step_by(16) {
        match read(name, &file[..cut]) {
            Err(key) => assert!(key.starts_with("err."), "{name} cut at {cut}: {key}"),
            Ok((have, missing)) => {
                let whole = have.len() <= want.len()
                    && have
                        .iter()
                        .zip(&want)
                        .all(|(a, b)| a.id == b.id && a.mz == b.mz && a.intensity == b.intensity);
                assert!(
                    whole,
                    "{name} cut at {cut}: a scan read short or out of place"
                );
                assert!(
                    have.len() == want.len() || missing > 0,
                    "{name} cut at {cut}: {} of {} scans and no notice",
                    have.len(),
                    want.len()
                );
            }
        }
    }
}

#[test]
fn a_shimadzu_file_cut_short_is_told() {
    every_cut_is_told("x.qgd", &comune::shimadzu::qgd(8));
    every_cut_is_told("x.lcd", &comune::shimadzu::lcd2020(8));
}

/// A copy cut before its directory (the header points past the end, `Ole::header_cut`) gets the same `err.file.cutShort` notice as one cut after it.
#[test]
fn a_shimadzu_file_cut_before_its_directory_is_cut_short() {
    for (name, file) in [
        ("x.qgd", comune::shimadzu::qgd(8)),
        ("x.lcd", comune::shimadzu::lcd2020(8)),
    ] {
        let mut told = 0;
        for cut in (8..file.len()).step_by(16) {
            if mzlab_core::ole::Ole::header_cut(&file[..cut]).is_some() {
                assert_eq!(
                    read(name, &file[..cut]).err(),
                    Some("err.file.cutShort"),
                    "{name} cut at {cut}"
                );
                told += 1;
            }
        }
        assert!(told > 0, "{name}: no cut before the directory");
    }
}
