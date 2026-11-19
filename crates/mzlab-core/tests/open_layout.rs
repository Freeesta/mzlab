//! Opening an mzML (`file_entry`, `open_items`, the channels) reads the layout of the file once: the second and third route read nothing of
//! the chromatogram tail (it was a whole pass of a DAD file each). Same answers, byte for byte, with the shared cache and without it. Synthetic data.
#![allow(clippy::unwrap_used)]

use base64::Engine;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::iteminfo::facts_from;
use mzlab_core::lr::{file_entry, open_items, Sample};
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

fn b64(n: usize) -> String {
    let v: Vec<u8> = (0..n)
        .flat_map(|k| (k as f64 * 0.001).to_le_bytes())
        .collect();
    base64::engine::general_purpose::STANDARD.encode(v)
}

/// 20 MS1 scans, then a TWC trace of `n` points (the bulk of the file, as in a DAD file).
fn dad_like(n: usize) -> Vec<u8> {
    let mut s =
        String::from("<?xml version=\"1.0\"?><mzML><run id=\"r\"><spectrumList count=\"20\">\n");
    for i in 0..20 {
        s += &format!("<spectrum index=\"{i}\" id=\"scan={i}\" defaultArrayLength=\"0\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"1\"/><scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"minute\"/></scan></scanList><binaryDataArrayList count=\"0\"></binaryDataArrayList></spectrum>\n", i as f64 * 0.01);
    }
    let arr = |acc: &str| {
        format!("<binaryDataArray encodedLength=\"1\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"{acc}\" name=\"x\"/><binary>{}</binary></binaryDataArray>", b64(n))
    };
    s += &format!("</spectrumList><chromatogramList count=\"1\"><chromatogram index=\"0\" id=\"TWC\" defaultArrayLength=\"{n}\"><cvParam accession=\"MS:1000813\" name=\"x\"/><binaryDataArrayList count=\"2\">{}{}</binaryDataArrayList></chromatogram></chromatogramList></run></mzML>\n", arr("MS:1000595"), arr("MS:1000515"));
    s.into_bytes()
}

struct Fixture {
    data: Vec<u8>,
    idx: Vec<u8>,
    size: u64,
}

fn fixture() -> Fixture {
    let mut data = dad_like(150_000);
    let size = data.len() as u64;
    let p = prepare_in(&mut data, size, "dad.mzML", None, None).unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &data, 64 << 20).unwrap();
    Fixture { data, idx, size }
}

impl Fixture {
    fn open(&self) -> Index<Vec<u8>> {
        Index::open(self.idx.clone(), self.idx.len() as u64).unwrap()
    }
    /// An index with a layout of its own: what each route met before the layout was shared.
    fn alone(&self) -> Index<Vec<u8>> {
        let mut i = self.open();
        i.layout = Default::default();
        i
    }
    fn reader(&self) -> Mzml<Counting> {
        Mzml::new(
            Counting {
                data: self.data.clone(),
                bytes: 0,
            },
            self.size,
        )
    }
}

#[test]
fn the_open_routes_read_the_chromatogram_tail_once() {
    let fx = fixture();
    let sample = Sample::default();
    let mut m = fx.reader();
    let end = fx.alone().scans.last().map_or(0, |s| s.end);
    let (mut alone, mut want) = (0, None);
    for _ in 0..3 {
        let c = m.r.bytes;
        let chroms = m.chromatograms(end).unwrap();
        want = Some(format!(
            "{:?}",
            facts_from(&mut m, end, chroms.len() as i64, true).unwrap()
        ));
        alone += m.r.bytes - c;
    }
    let mut idx = fx.open();
    idx.layout = Default::default();
    m.r.bytes = 0;
    file_entry(&mut idx, &mut m, "dad.mzML", fx.size).unwrap();
    open_items(&mut idx, &mut m, "dad.mzML", &sample).unwrap();
    let c = m.r.bytes;
    let chroms = idx.layout.chromatograms(&mut m, end).unwrap();
    let facts = idx
        .layout
        .facts(&mut m, end, chroms.len() as i64, true)
        .unwrap();
    eprintln!(
        "tail reads: {alone} bytes before, {} after (entry+items+channels)",
        m.r.bytes
    );
    assert_eq!(
        m.r.bytes, c,
        "the channels read nothing the open routes had not read"
    );
    assert!(m.r.bytes * 2 < alone, "{} bytes against {alone}", m.r.bytes);
    assert_eq!(
        Some(format!("{facts:?}")),
        want,
        "the same facts as facts_from"
    );
}

#[test]
fn the_answers_are_those_of_an_index_that_never_cached() {
    let fx = fixture();
    let sample = Sample::default();
    let mut m = fx.reader();
    let mut shared = fx.open();
    shared.layout = Default::default();
    let entry = file_entry(&mut shared, &mut m, "dad.mzML", fx.size).unwrap();
    let items = open_items(&mut shared, &mut m, "dad.mzML", &sample).unwrap();
    let mut m2 = Mzml::new(fx.data.clone(), fx.size);
    assert_eq!(
        entry,
        file_entry(&mut fx.alone(), &mut m2, "dad.mzML", fx.size).unwrap()
    );
    let again = open_items(&mut fx.alone(), &mut m2, "dad.mzML", &sample).unwrap();
    assert_eq!(
        items.iter().map(|i| &i.info).collect::<Vec<_>>(),
        again.iter().map(|i| &i.info).collect::<Vec<_>>()
    );
}

#[test]
fn a_second_index_of_the_same_file_finds_the_layout_the_first_one_read() {
    let fx = fixture();
    let mut m = fx.reader();
    let first = fx.open();
    let end = first.scans.last().map_or(0, |s| s.end);
    let ch = first.layout.chromatograms(&mut m, end).unwrap();
    first
        .layout
        .facts(&mut m, end, ch.len() as i64, true)
        .unwrap();
    let before = m.r.bytes;
    // the slot of a panel next to the one of the list: another `Index::open` of the same `.mzidx`
    let second = fx.open();
    let ch = second.layout.chromatograms(&mut m, end).unwrap();
    second
        .layout
        .facts(&mut m, end, ch.len() as i64, true)
        .unwrap();
    assert_eq!(
        m.r.bytes, before,
        "the second index reads nothing of the layout"
    );
}
