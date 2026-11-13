//! A part of an Agilent `.D` that cannot be read is left out and said, never the run: an MRM visit whose points do not decode leaves the SRM chromatograms
//! and is counted in `metodo/1.avvisi`, a side member that cannot be read (here `MSTS.xml`) is named there. Synthetic folders only.

#![allow(clippy::disallowed_methods)]

mod agilent_synth;

use agilent_synth::{encode, sample};
use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::{self, FileSet, Limits, Member};

/// A member whose bytes cannot be read: it declares more than it holds.
struct Short(Vec<u8>);

impl ReadAt for Short {
    fn read_at(&mut self, at: u64, buf: &mut [u8]) -> mzlab_core::error::Result<()> {
        let end = at as usize + buf.len();
        let b = self.0.get(at as usize..end).ok_or_else(|| {
            mzlab_core::error::Error::new("err.file.agilent").with("detail", "short member")
        })?;
        buf.copy_from_slice(b);
        Ok(())
    }
}

fn member(n: &str, b: Vec<u8>) -> Member {
    Member {
        nome: format!("x.D/AcqData/{n}"),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    }
}

#[test]
fn a_side_member_that_cannot_be_read_is_named_and_the_run_read() {
    let (scan, peak) = encode(&sample());
    let ts = Member {
        nome: "x.D/AcqData/MSTS.xml".into(),
        dim: 100,
        dati: Box::new(Short(b"<TimeSegments/>".to_vec())),
        sha1: None,
    };
    let fs = FileSet {
        membri: vec![member("MSScan.bin", scan), member("MSPeak.bin", peak), ts],
    };
    let mut l = readers::apri(fs, &Limits::sito()).expect("the run opens");
    assert_eq!(l.index(0).unwrap().len(), 3);
    let doc = l.method(0).unwrap();
    assert!(
        doc.contains("meta.danneggiato") && doc.contains("AcqData/MSTS.xml: could not be read"),
        "{doc}"
    );
}

#[cfg(feature = "estesi")]
#[test]
fn an_mrm_visit_that_does_not_decode_leaves_the_srm_chromatograms_and_is_counted() {
    // scans 2-4 are MRM visits (3 has no points); the first intensity of visit 2 is not a number
    let mut s = sample();
    for x in s.iter_mut().skip(1) {
        x.kind = 256;
        x.level = 2;
    }
    let (scan, mut peak) = encode(&s);
    let at = 68 + 48 + 24;
    peak[at..at + 8].copy_from_slice(&f64::NAN.to_le_bytes());
    let fs = FileSet {
        membri: vec![member("MSScan.bin", scan), member("MSPeak.bin", peak)],
    };
    let mut l = readers::apri(fs, &Limits::sito()).expect("the run opens");
    let srm: Vec<_> = l
        .chromatograms(0)
        .expect("the chromatograms are listed")
        .into_iter()
        .filter(|c| c.id.starts_with("SRM SIC "))
        .collect();
    // the other visit still makes its three transitions
    assert_eq!(srm.len(), 3);
    for c in &srm {
        let t = l.chromatogram(0, &c.id).unwrap();
        assert_eq!(t.time.len(), 1, "{}", c.id);
    }
    let doc = l.method(0).unwrap();
    assert!(
        doc.contains("1 MRM visits whose points do not decode"),
        "{doc}"
    );
}
