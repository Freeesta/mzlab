//! `Stream`: the window serves short reads with the same bytes as the source and with far fewer `read_at` calls.
use super::*;
use crate::idxread::ReadAt;
use std::cell::Cell;
use std::io::{Read, Seek, SeekFrom};
use std::rc::Rc;

struct Contato(Vec<u8>, Rc<Cell<usize>>, Rc<Cell<usize>>);

impl ReadAt for Contato {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.1.set(self.1.get() + 1);
        self.2.set(self.2.get() + buf.len());
        self.0.read_at(off, buf)
    }
}

fn data(n: usize) -> Vec<u8> {
    (0..n as u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 24) as u8)
        .collect()
}

fn flow(d: &[u8]) -> (Stream, Rc<Cell<usize>>) {
    let (s, c, _) = byte_stream(d);
    (s, c)
}

/// The stream, its count of `read_at` calls and of the bytes they asked for.
fn byte_stream(d: &[u8]) -> (Stream, Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let (c, b) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    (
        Stream::new(
            Box::new(Contato(d.to_vec(), c.clone(), b.clone())),
            d.len() as u64,
        ),
        c,
        b,
    )
}

#[test]
fn short_reads_share_a_window_and_match_the_source() {
    let d = data(300_000);
    let (mut s, calls) = flow(&d);
    let mut got = Vec::new();
    let mut b = [0u8; 7];
    while s.read(&mut b).unwrap() == b.len() {
        got.extend_from_slice(&b);
    }
    assert_eq!(got[..], d[..got.len()]);
    // the window starts at FINESTRA_SALTO and doubles up to FINESTRA: a few more calls than whole windows
    let growth = (WINDOW / SKIP_WINDOW).ilog2() as usize;
    assert!(
        calls.get() <= d.len() / WINDOW + 1 + growth,
        "{} calls",
        calls.get()
    );
}

#[test]
fn a_skipped_profile_is_not_read() {
    // the centroid path of an Orbitrap scan: the packet header, a jump over the profile, then the centroid list
    let (packet, testa, list) = (220_000usize, 40usize, 24_000usize);
    let d = data(packet * 10);
    let (mut s, _, byte) = byte_stream(&d);
    for k in 0..10 {
        let start = k * packet;
        s.seek(SeekFrom::Start(start as u64)).unwrap();
        let mut t = vec![0u8; testa];
        s.read_exact(&mut t).unwrap();
        assert_eq!(t[..], d[start..start + testa]);
        let list_from = start + packet - list - 4;
        s.seek(SeekFrom::Start(list_from as u64)).unwrap();
        let mut count = [0u8; 4];
        s.read_exact(&mut count).unwrap();
        let mut l = vec![0u8; list];
        s.read_exact(&mut l).unwrap();
        assert_eq!(l[..], d[list_from + 4..list_from + 4 + list]);
    }
    // each scan asks for the header window, the list window and the list: never the 64 KiB windows over the profile
    let per_scan = byte.get() / 10;
    assert!(
        per_scan <= 2 * SKIP_WINDOW + list + 4,
        "{per_scan} bytes per scan"
    );
}

#[test]
fn reads_across_the_window_seeks_and_the_end_are_exact() {
    let d = data(200_000);
    let (mut s, calls) = flow(&d);
    // a read that straddles the end of the first window
    s.seek(SeekFrom::Start(WINDOW as u64 - 5)).unwrap();
    let mut b = [0u8; 10];
    s.read_exact(&mut b).unwrap();
    assert_eq!(b[..], d[WINDOW - 5..WINDOW + 5]);
    // backwards, then relative, then from the end
    s.seek(SeekFrom::Start(3)).unwrap();
    s.read_exact(&mut b).unwrap();
    assert_eq!(b[..], d[3..13]);
    s.seek(SeekFrom::Current(100)).unwrap();
    s.read_exact(&mut b).unwrap();
    assert_eq!(b[..], d[113..123]);
    s.seek(SeekFrom::End(-4)).unwrap();
    let mut t = [0u8; 10];
    assert_eq!(s.read(&mut t).unwrap(), 4);
    assert_eq!(t[..4], d[d.len() - 4..]);
    assert_eq!(s.read(&mut t).unwrap(), 0);
    // a read longer than the window goes straight to the source
    s.seek(SeekFrom::Start(11)).unwrap();
    let before = calls.get();
    let mut big = vec![0u8; WINDOW + 1];
    s.read_exact(&mut big).unwrap();
    assert_eq!(big[..], d[11..11 + WINDOW + 1]);
    assert_eq!(calls.get(), before + 1);
    assert!(s.seek(SeekFrom::Current(-1_000_000)).is_err());
}
