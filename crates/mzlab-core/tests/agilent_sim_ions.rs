//! Agilent SIM through the `Lettore`: an ion of the method with no points is an empty SIC (not an error), and the SIM scans are read once for all the ions.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod agilent_synth;
mod conv_aiuto;

use agilent_synth::*;
use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{self, FileSet, Limiti};
use mzlab_core::mzml;
use std::cell::Cell;
use std::rc::Rc;

/// `MSPeak.bin` that counts its reads.
struct Counting(Vec<u8>, Rc<Cell<usize>>);

impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.1.set(self.1.get() + 1);
        self.0.read_at(off, buf)
    }
}

/// A method with one SIM group of three ions; the run only ever sees 57 and 71.
const METHOD: &str = "MS Information\nAcquisition Mode: SIM\nGroup 1 Group ID: g\nGroup Start Time: 0.0\n(57.0,10) (71.0,10) (99.0,10)\n";

fn utf16(s: &str) -> Vec<u8> {
    let mut b = vec![0xff, 0xfe];
    b.extend(s.encode_utf16().flat_map(u16::to_le_bytes));
    b
}

fn sim(t: f64, k: f64) -> Scan {
    Scan {
        kind: 2,
        pts: vec![(57.0, 10.0 * k), (71.0, 20.0 * k)],
        time: t,
        ..sample().remove(0)
    }
}

fn set(reads: &Rc<Cell<usize>>) -> FileSet {
    let (idx, peak) = encode(&[sim(1.0, 1.0), sim(1.1, 2.0), sim(1.2, 3.0)]);
    let m = |nome: &str, b: &[u8]| conv_aiuto::membro(nome, b);
    let mut peak_m = m("x.D/AcqData/MSPeak.bin", &peak);
    peak_m.dati = Box::new(Counting(peak, reads.clone()));
    FileSet {
        membri: vec![
            m("x.D/AcqData/MSScan.bin", &idx),
            peak_m,
            m("x.D/AcqData/acqmeth.txt", &utf16(METHOD)),
        ],
    }
}

#[test]
fn an_ion_with_no_points_is_an_empty_sic() {
    let reads = Rc::new(Cell::new(0));
    let mut l = lettori::apri(set(&reads), &Limiti::sito()).unwrap();
    let c = l.cromatogramma(0, "SIC 99.00").unwrap();
    assert!(c.time.is_empty() && c.intensity.is_empty());
    assert_eq!(
        l.cromatogramma(0, "SIC 57.00").unwrap().intensity,
        [10.0, 20.0, 30.0]
    );
}

#[test]
fn the_mzml_of_a_run_with_a_missing_ion_converts_and_leaves_that_trace_out() {
    let reads = Rc::new(Cell::new(0));
    let back = mzml::read(&conv_aiuto::mzml(set(&reads))[..]).unwrap();
    let ids: Vec<_> = back.chromatograms.iter().map(|c| c.id.as_str()).collect();
    // the writer leaves out empty traces: only the ion with no points is missing
    assert_eq!(ids, ["TIC", "BPC", "SIC 57.00", "SIC 71.00"]);
}

#[test]
fn the_sim_scans_are_read_once_for_all_the_ions() {
    let reads = Rc::new(Cell::new(0));
    let mut l = lettori::apri(set(&reads), &Limiti::sito()).unwrap();
    l.cromatogramma(0, "SIC 57.00").unwrap();
    let after_first = reads.get();
    assert!(after_first > 0);
    l.cromatogramma(0, "SIC 71.00").unwrap();
    l.cromatogramma(0, "SIC 99.00").unwrap();
    assert_eq!(
        reads.get(),
        after_first,
        "the other ions re-read MSPeak.bin"
    );
}
