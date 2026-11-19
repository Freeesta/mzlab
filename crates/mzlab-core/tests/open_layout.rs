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

#[test]
fn the_open_routes_read_the_chromatogram_tail_once() {
    let mut data = dad_like(150_000);
    let size = data.len() as u64;
    let p = prepare_in(&mut data, size, "dad.mzML", None, None).unwrap();
    let (idx_bytes, _) = build_mzml_to_vec(&p.table, &data, 64 << 20).unwrap();
    let open = || Index::open(idx_bytes.clone(), idx_bytes.len() as u64).unwrap();
    let sample = Sample::default();
    let mut m = Mzml::new(
        Counting {
            data: data.clone(),
            bytes: 0,
        },
        size,
    );

    // before: every route scans the tail (what `facts_from` does on its own)
    let end = open().scans.last().map_or(0, |s| s.end);
    let mut alone = 0;
    let mut want = None;
    for _ in 0..3 {
        let c = m.r.bytes;
        let chroms = m.chromatograms(end).unwrap();
        want = Some(format!(
            "{:?}",
            facts_from(&mut m, end, chroms.len() as i64, true).unwrap()
        ));
        alone += m.r.bytes - c;
    }

    // after: entry, items, channels through the shared layout of one index
    let mut idx = open();
    m.r.bytes = 0;
    let entry = file_entry(&mut idx, &mut m, "dad.mzML", size).unwrap();
    let items = open_items(&mut idx, &mut m, "dad.mzML", &sample).unwrap();
    let c = m.r.bytes;
    let chroms = idx.layout.chromatograms(&mut m, end).unwrap();
    let facts = idx
        .layout
        .facts(&mut m, end, chroms.len() as i64, true)
        .unwrap();
    let shared = m.r.bytes;
    eprintln!("tail reads: {alone} bytes before, {shared} after (entry+items+channels)");
    assert_eq!(
        m.r.bytes, c,
        "the channels read nothing the open routes had not read"
    );
    assert!(shared * 2 < alone, "{shared} bytes against {alone}");
    assert_eq!(
        Some(format!("{facts:?}")),
        want,
        "the same facts as facts_from"
    );

    // same answers as an index that never cached
    let mut fresh = open();
    let mut m2 = Mzml::new(data.clone(), size);
    assert_eq!(
        entry,
        file_entry(&mut fresh, &mut m2, "dad.mzML", size).unwrap()
    );
    let again = open_items(&mut open(), &mut m2, "dad.mzML", &sample).unwrap();
    assert_eq!(
        items.iter().map(|i| &i.info).collect::<Vec<_>>(),
        again.iter().map(|i| &i.info).collect::<Vec<_>>()
    );
}
