//! `sorgente::testo_scansione` reads the header of a `<spectrum>` only (up to its arrays), not the base64 of the arrays.

use mzlab_core::error::Result;
use mzlab_core::idxread::{ReadAt, Scan};
use mzlab_core::sorgente::testo_scansione;

struct Counting {
    data: Vec<u8>,
    read: u64,
}

impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.read += buf.len() as u64;
        self.data.read_at(off, buf)
    }
}

#[test]
fn scan_text_stops_before_the_arrays() {
    let head = "<spectrum index=\"0\" id=\"scan=1\">\n<cvParam accession=\"MS:1000501\" name=\"scan window lower limit\" value=\"50\"/>\n";
    let arrays = format!("<binaryDataArrayList count=\"1\"><binaryDataArray><binary>{}</binary></binaryDataArray></binaryDataArrayList></spectrum>", "A".repeat(2_000_000));
    let data = format!("{head}{arrays}").into_bytes();
    let sc = Scan {
        start: 0,
        end: data.len() as u64,
        ..Scan::default()
    };
    let mut m = Counting { data, read: 0 };
    let text = testo_scansione(&mut m, &sc).unwrap();
    assert_eq!(text, head);
    assert!(m.read <= 16_384, "read {} bytes", m.read);
}

#[test]
fn spectrum_and_mobility_come_from_one_read() {
    use mzlab_core::sorgente::Scansioni;
    let data = b"<spectrum index=\"0\" id=\"scan=1\"><cvParam accession=\"MS:1002815\" name=\"inverse reduced ion mobility\" value=\"0.85\"/></spectrum>".to_vec();
    let sc = Scan {
        start: 0,
        end: data.len() as u64,
        ..Scan::default()
    };
    let mut a = Counting {
        data: data.clone(),
        read: 0,
    };
    let (sp, mob) = (a.spettro(&sc).unwrap(), a.mobilita(&sc).unwrap());
    let mut b = Counting { data, read: 0 };
    let both = b.spettro_mobilita(&sc).unwrap();
    assert_eq!(both.0, sp);
    assert_eq!(
        both.1.as_ref().map(|m| m.scan),
        mob.as_ref().map(|m| m.scan)
    );
    assert!(mob.is_some());
    assert_eq!(a.read, 2 * b.read, "separate calls read the spectrum twice");
}
