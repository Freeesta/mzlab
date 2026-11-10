//! `Index::open_lean` (the gap filling of the page) is `Index::open` without the scan table: the same keys, sections, source and arrays, no scans,
//! and the table's bytes never read. Synthetic data only.

use std::cell::Cell;
use std::rc::Rc;

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;

/// The bytes of an index, counting in the shared cell what is read of them.
struct Counted(Vec<u8>, Rc<Cell<u64>>);

impl ReadAt for Counted {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        self.1.set(self.1.get() + buf.len() as u64);
        self.0.read_at(off, buf)
    }
}

fn index() -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "lean".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(200).unwrap();
    for i in 0..200 {
        w.spectrum(&Spectrum {
            index: i,
            id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
            level: 1,
            rt: 0.05 * i as f64,
            polarity: 1,
            profile: false,
            filter: String::new(),
            precursor: None,
            mz: (0..30).map(|b| 100.0 + b as f64 * 1.5).collect(),
            intensity: (0..30)
                .map(|b| 10.0 + ((i * 7 + b * 3) % 50) as f32)
                .collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "lean.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

#[test]
fn open_lean_is_open_without_the_scan_table() {
    let b = index();
    let n = b.len() as u64;
    let read = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let mut whole = Index::open(Counted(b.clone(), read.0.clone()), n).unwrap();
    let mut lean = Index::open_lean(Counted(b, read.1.clone()), n).unwrap();
    // the scan table is the bulk of what `open` reads; `open_lean` reads the header and the meta only
    assert!(
        read.1.get() > 0 && read.1.get() * 4 < read.0.get(),
        "{read:?}"
    );
    assert!(!whole.scans.is_empty() && lean.scans.is_empty());
    assert_eq!(whole.keys, lean.keys);
    assert_eq!(
        format!("{:?}", whole.sections),
        format!("{:?}", lean.sections)
    );
    assert_eq!((&whole.source, &whole.run), (&lean.source, &lean.run));
    let mut mz = (Vec::new(), Vec::new());
    for si in 0..whole.sections.len() {
        assert_eq!(
            format!("{:?}", whole.arrays(si).unwrap()),
            format!("{:?}", lean.arrays(si).unwrap())
        );
        let peaks = whole.sections[si].n_peaks;
        whole.mz_into(si, 0, peaks, &mut mz.0).unwrap();
        lean.mz_into(si, 0, peaks, &mut mz.1).unwrap();
    }
    assert!(!mz.0.is_empty() && mz.0 == mz.1);
}
