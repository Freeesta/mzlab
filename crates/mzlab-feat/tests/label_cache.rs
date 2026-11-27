//! The profile-label check is probed once per `Index` and kept on it (`Index::label`): a second ask reads the stored answer, another file or a reopened
//! index gets its own, and a lean index (no scan table yet) is never cached early. Synthetic data only.

use std::cell::Cell;
use std::rc::Rc;

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::rawinfo::LabelCheck;
use mzlab_core::upload::prepare;
use mzlab_feat::lrscreen::label_check;

/// 40 MS1 scans labelled profile: either 12 isolated ions each (centroids under a profile label: sparse) or a 0.1 Da grid of Gaussian peaks (genuine).
fn index_bytes(isolated: bool) -> Vec<u8> {
    let head = Header {
        run: "cache".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::new(Vec::new(), &head).unwrap();
    w.open_spectra(40).unwrap();
    for i in 0..40usize {
        let (mz, intensity): (Vec<f64>, Vec<f32>) = if isolated {
            (0..12)
                .map(|j| {
                    (
                        101.2 + 9.7 * j as f64 + 0.01 * i as f64,
                        5e4 / (1.0 + j as f32),
                    )
                })
                .unzip()
        } else {
            (0..300)
                .map(|b| {
                    let m = 200.0 + b as f64 * 0.1;
                    let y: f64 = [210.0, 221.0, 228.0]
                        .iter()
                        .map(|c| 6e4 * (-((m - c) * (m - c)) / (2.0 * 0.15 * 0.15)).exp())
                        .sum();
                    (m, y as f32)
                })
                .unzip()
        };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 0.05 * i as f64,
            polarity: 1,
            profile: true,
            filter: String::new(),
            precursor: None,
            mz,
            intensity,
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "cache.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

/// The bytes of an index, counting each read the index makes of them.
struct Counting {
    bytes: Vec<u8>,
    reads: Rc<Cell<usize>>,
}

impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.reads.set(self.reads.get() + 1);
        self.bytes.read_at(off, buf)
    }
}

fn whole(b: &[u8]) -> Index<Vec<u8>> {
    Index::open(b.to_vec(), b.len() as u64).unwrap()
}

#[test]
fn the_second_ask_reads_the_stored_answer_and_does_not_probe() {
    let b = index_bytes(true);
    let reads = Rc::new(Cell::new(0));
    let r = Counting {
        bytes: b.clone(),
        reads: reads.clone(),
    };
    let mut index = Index::open(r, b.len() as u64).unwrap();
    let opened = reads.get();
    let first = label_check(&mut index).expect("a file labelled profile is checked");
    assert!(reads.get() > opened, "the first ask probes the points");
    assert!(first.sparse && index.label == Some(Some(first.clone())));
    let probed = reads.get();
    assert_eq!(label_check(&mut index), Some(first.clone()));
    assert_eq!(reads.get(), probed, "the second ask reads nothing");
    // proof that the answer comes from the index and not from a second probe: a stored value that no probe would give is returned as it is
    let marker = LabelCheck {
        probed: 999,
        ..first
    };
    index.label = Some(Some(marker.clone()));
    assert_eq!(label_check(&mut index), Some(marker));
}

#[test]
fn each_file_and_each_reopened_index_gets_its_own_answer() {
    let (sparse, genuine) = (index_bytes(true), index_bytes(false));
    let (mut a, mut g) = (whole(&sparse), whole(&genuine));
    let (ca, cg) = (label_check(&mut a).unwrap(), label_check(&mut g).unwrap());
    assert!(ca.sparse && !cg.sparse, "{ca:?} {cg:?}");
    assert_eq!(label_check(&mut a), Some(ca.clone()));
    assert_eq!(
        label_check(&mut g),
        Some(cg),
        "the first file's answer is not shared"
    );
    // an index opened again, and the index over another reader of the same bytes, start unchecked and probe for themselves
    assert!(a.label.is_some());
    let mut again = whole(&sparse);
    assert!(again.label.is_none());
    assert_eq!(label_check(&mut again), Some(ca.clone()));
    let mut over = a.over(sparse.clone());
    assert!(over.label.is_none(), "`over` does not carry the answer");
    assert_eq!(label_check(&mut over), Some(ca));
}

#[test]
fn a_lean_index_is_not_cached_before_its_scan_table_loads() {
    let b = index_bytes(true);
    let mut lean = Index::open_lean(b.clone(), b.len() as u64).unwrap();
    assert_eq!(label_check(&mut lean), None, "no scan table: not profile");
    assert!(lean.label.is_none(), "that answer is not kept");
    lean.load_scans().unwrap();
    let c = label_check(&mut lean).expect("checked once the scans are there");
    assert!(c.sparse);
    assert_eq!(lean.label, Some(Some(c)));
}
