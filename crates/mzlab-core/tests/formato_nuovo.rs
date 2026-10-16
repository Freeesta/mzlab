//! An index written by a newer program is refused with the typed error `error.newerFormat` (never `error.badIndex`, which means damaged): the caller leaves it as it is.
//! A known version still opens. Synthetic data only.

use mzlab_core::idxnuovo::newer_version;
use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;

fn index() -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "nuovo".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(5).unwrap();
    for i in 0..5 {
        w.spectrum(&Spectrum {
            index: i,
            id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
            level: 1,
            rt: 0.05 * i as f64,
            polarity: 1,
            profile: false,
            filter: String::new(),
            precursor: None,
            mz: vec![100.0, 150.0, 200.0],
            intensity: vec![10.0, 20.0, 30.0],
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "nuovo.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

fn key(idx: Vec<u8>) -> Option<(String, Vec<(String, String)>)> {
    let n = idx.len() as u64;
    Index::open(idx, n).err().map(|e| {
        (
            e.key.to_string(),
            e.params
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        )
    })
}

#[test]
fn a_known_version_opens() {
    let idx = index();
    assert_eq!(newer_version(&idx), None);
    assert!(key(idx).is_none());
}

#[test]
fn a_header_version_above_the_known_one_is_newer_and_not_written() {
    let mut idx = index();
    idx[8..12].copy_from_slice(&3u32.to_le_bytes());
    assert_eq!(newer_version(&idx), Some(3));
    let (k, p) = key(idx).expect("refused");
    assert_eq!(k, "error.newerFormat");
    assert!(
        p.contains(&("kind".into(), "mzidx".into())) && p.contains(&("version".into(), "3".into())),
        "{p:?}"
    );
}

#[test]
fn a_newer_magic_is_newer_whatever_follows() {
    let mut idx = index();
    idx[..8].copy_from_slice(b"MZIDX002");
    idx.truncate(100); // the layout of a newer format is unknown: no size check comes first
    assert_eq!(newer_version(&idx), Some(2));
    assert_eq!(key(idx).unwrap().0, "error.newerFormat");
}

#[test]
fn a_damaged_index_stays_damaged() {
    let mut idx = index();
    idx[0] = b'X';
    assert_eq!(newer_version(&idx), None);
    assert_eq!(key(idx).unwrap().0, "error.badIndex");
    let mut idx = index();
    idx.truncate(idx.len() - 1);
    assert_eq!(key(idx).unwrap().0, "error.badIndex");
}
