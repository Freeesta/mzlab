//! `Stream`: the window serves short reads with the same bytes as the source and with far fewer `read_at` calls.
use super::*;
use std::cell::Cell;
use std::rc::Rc;

struct Contato(Vec<u8>, Rc<Cell<usize>>);

impl ReadAt for Contato {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.1.set(self.1.get() + 1);
        self.0.read_at(off, buf)
    }
}

fn dati(n: usize) -> Vec<u8> {
    (0..n as u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 24) as u8)
        .collect()
}

fn flusso(d: &[u8]) -> (Stream, Rc<Cell<usize>>) {
    let c = Rc::new(Cell::new(0));
    (
        Stream::new(Box::new(Contato(d.to_vec(), c.clone())), d.len() as u64),
        c,
    )
}

#[test]
fn short_reads_share_a_window_and_match_the_source() {
    let d = dati(300_000);
    let (mut s, calls) = flusso(&d);
    let mut got = Vec::new();
    let mut b = [0u8; 7];
    while s.read(&mut b).unwrap() == b.len() {
        got.extend_from_slice(&b);
    }
    assert_eq!(got[..], d[..got.len()]);
    assert!(
        calls.get() <= d.len() / FINESTRA + 1,
        "{} calls",
        calls.get()
    );
}

#[test]
fn reads_across_the_window_seeks_and_the_end_are_exact() {
    let d = dati(200_000);
    let (mut s, calls) = flusso(&d);
    // a read that straddles the end of the first window
    s.seek(SeekFrom::Start(FINESTRA as u64 - 5)).unwrap();
    let mut b = [0u8; 10];
    s.read_exact(&mut b).unwrap();
    assert_eq!(b[..], d[FINESTRA - 5..FINESTRA + 5]);
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
    let mut big = vec![0u8; FINESTRA + 1];
    s.read_exact(&mut big).unwrap();
    assert_eq!(big[..], d[11..11 + FINESTRA + 1]);
    assert_eq!(calls.get(), before + 1);
    assert!(s.seek(SeekFrom::Current(-1_000_000)).is_err());
}
