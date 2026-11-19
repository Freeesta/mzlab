//! The `.mzidx` keeps the chromatogram list of an mzML (`run.chromatograms`): `Index::open` answers `Layout::chromatograms` from it without a read of the
//! file, with the `Chrom` values `Mzml::chromatograms` finds. Synthetic files only: a DAD-like one and a triple-quad one with many SRM transitions.
#![allow(clippy::unwrap_used)]

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::upload::prepare_in;

struct Counting {
    data: Vec<u8>,
    bytes: u64,
}
impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, out: &mut [u8]) -> mzlab_core::Result<()> {
        self.bytes += out.len() as u64;
        self.data.read_at(off, out)
    }
}

/// Three MS1 scans and `n` SRM transitions (names with a quote and an accent, a dwell time, a collision energy), then a TIC and a pump pressure trace.
fn srm_like(n: usize) -> Vec<u8> {
    let mut s =
        String::from("<?xml version=\"1.0\"?><mzML><run id=\"r\"><spectrumList count=\"3\">\n");
    for i in 0..3 {
        s += &format!("<spectrum index=\"{i}\" id=\"scan={i}\" defaultArrayLength=\"0\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"1\"/><scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"minute\"/></scan></scanList><binaryDataArrayList count=\"0\"></binaryDataArrayList></spectrum>\n", i as f64 * 0.01);
    }
    s += &format!("</spectrumList><chromatogramList count=\"{}\">\n", n + 2);
    s += "<chromatogram index=\"0\" id=\"TIC\" defaultArrayLength=\"0\"><cvParam accession=\"MS:1000235\" name=\"total ion current chromatogram\"/></chromatogram>\n";
    for i in 0..n {
        s += &format!("<chromatogram index=\"{}\" id=\"SRM SIC Q1={}.25 Q3={}.5 name=\"Cmp &quot;{i}&quot; \u{e9}\" CE={}.5\" defaultArrayLength=\"0\"><cvParam accession=\"MS:1001473\" name=\"selected reaction monitoring chromatogram\"/><userParam name=\"MS_dwell_time\" value=\"0.{}\"/></chromatogram>\n", i + 1, 100 + i, 50 + i, 10 + i % 40, 1 + i % 9);
    }
    s += "<chromatogram index=\"999\" id=\"Pump Pressure\" defaultArrayLength=\"0\"><cvParam accession=\"MS:1003019\" name=\"pressure chromatogram\"/></chromatogram>\n";
    s += "</chromatogramList></run></mzML>\n";
    s.into_bytes()
}

fn index_of(mut data: Vec<u8>, name: &str) -> (Vec<u8>, Vec<u8>, u64) {
    let size = data.len() as u64;
    let p = prepare_in(&mut data, size, name, None, None).unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &data, 64 << 20).unwrap();
    (data, idx, size)
}

#[test]
fn the_list_of_the_index_is_the_list_of_the_file_and_costs_no_read() {
    for (name, data) in [
        ("srm_few.mzML", srm_like(3)),
        ("srm_many.mzML", srm_like(600)),
    ] {
        let (data, idx, size) = index_of(data, name);
        let index = Index::open(idx.clone(), idx.len() as u64).unwrap();
        let end = index.scans.last().map_or(0, |s| s.end);
        let mut m = Mzml::new(
            Counting {
                data: data.clone(),
                bytes: 0,
            },
            size,
        );
        let held = index.layout.chromatograms(&mut m, end).unwrap();
        assert_eq!(m.r.bytes, 0, "{name}: the index answers without a read");
        let want = Mzml::new(data, size).chromatograms(end).unwrap();
        assert!(want.len() >= 5, "{name}");
        assert_eq!(format!("{:?}", *held), format!("{want:?}"), "{name}");
        assert_eq!(
            index.run.get("n_chromatograms").and_then(|v| v.f64()),
            Some(want.len() as f64),
            "{name}"
        );
    }
}

/// The header and the meta of `idx` with the version changed (`from` -> `to` in the header) or a key of the `run` object renamed.
fn damaged(idx: &[u8], version: Option<u32>, rename: Option<(&[u8], &[u8])>) -> Vec<u8> {
    let mut out = idx.to_vec();
    if let Some(v) = version {
        out[8..12].copy_from_slice(&v.to_le_bytes());
    }
    if let Some((from, to)) = rename {
        assert_eq!(from.len(), to.len());
        let at = out.windows(from.len()).position(|w| w == from).unwrap();
        out[at..at + from.len()].copy_from_slice(to);
    }
    out
}

#[test]
fn an_index_of_an_older_version_or_without_the_list_is_refused_not_scanned() {
    let (_, idx, _) = index_of(srm_like(4), "srm_old.mzML");
    assert_eq!(u32::from_le_bytes(idx[8..12].try_into().unwrap()), 3);
    let open = |b: Vec<u8>| Index::open(b.clone(), b.len() as u64).err().map(|e| e.key);
    assert_eq!(open(idx.clone()), None);
    for old in [1, 2] {
        assert_eq!(
            open(damaged(&idx, Some(old), None)),
            Some("error.badIndex"),
            "version {old}"
        );
    }
    // a version 3 mzML index whose `run` has no list is damaged: nothing falls back to the tail
    assert_eq!(
        open(damaged(
            &idx,
            None,
            Some((b"\"chromatograms\"", b"\"chromatogramz\""))
        )),
        Some("error.badIndex")
    );
    // a newer one is still `error.newerFormat`
    assert_eq!(
        open(damaged(&idx, Some(4), None)),
        Some("error.newerFormat")
    );
}

#[test]
fn a_file_with_no_chromatograms_keeps_an_empty_list() {
    let (data, idx, size) = index_of(srm_like(0), "srm_none.mzML");
    let _ = (data, size);
    let index = Index::open(idx.clone(), idx.len() as u64).unwrap();
    let end = index.scans.last().map_or(0, |s| s.end);
    let mut m = Mzml::new(
        Counting {
            data: vec![],
            bytes: 0,
        },
        0,
    );
    // the two fixed traces (TIC, pump pressure) are there: the list is the file's, whatever it holds
    assert_eq!(index.layout.chromatograms(&mut m, end).unwrap().len(), 2);
    assert_eq!(m.r.bytes, 0);
}
