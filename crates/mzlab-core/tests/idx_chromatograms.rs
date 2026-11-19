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

#[test]
fn an_index_without_the_list_scans_the_tail_as_before() {
    let (data, idx, size) = index_of(srm_like(4), "srm_old.mzML");
    // the meta of an index written before the list was kept: the key cut out of the `run` object, the lengths in the header moved with it
    let find = |from: usize, pat: &[u8]| {
        from + idx[from..]
            .windows(pat.len())
            .position(|w| w == pat)
            .unwrap()
    };
    let from = find(0, b",\"chromatograms\":[");
    let to = find(from, b"]]},\"keys\"") + 2;
    let cut = to - from;
    let mut old = [&idx[..from], &idx[to..]].concat();
    let total = old.len() as u64;
    old[24..32].copy_from_slice(
        &(u64::from_le_bytes(idx[24..32].try_into().unwrap()) - cut as u64).to_le_bytes(),
    );
    old[48..56].copy_from_slice(&total.to_le_bytes());
    let index = Index::open(old.clone(), total).unwrap();
    let end = index.scans.last().map_or(0, |s| s.end);
    let mut m = Mzml::new(
        Counting {
            data: data.clone(),
            bytes: 0,
        },
        size,
    );
    let got = index.layout.chromatograms(&mut m, end).unwrap();
    assert!(m.r.bytes > 0, "no list in the index: the file is read");
    assert_eq!(
        format!("{:?}", *got),
        format!("{:?}", Mzml::new(data, size).chromatograms(end).unwrap())
    );
}
