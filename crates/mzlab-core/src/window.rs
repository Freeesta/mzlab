//! Window cache over a `ReadAt` source.

use crate::error::Result;
use crate::idxread::ReadAt;

/// Window cache for readers whose every call is costly (a synchronous `Blob` read in the browser takes about a millisecond whatever its length):
/// a small read is served from the last window of `WINDOW` bytes, a miss reads the next window from the requested offset. A read longer than
/// the window goes straight to the source.
pub struct Windowed<'a, R: ReadAt> {
    r: &'a mut R,
    size: u64,
    at: u64,
    buf: Vec<u8>,
}

const WINDOW: usize = 1 << 20;

impl<'a, R: ReadAt> Windowed<'a, R> {
    pub fn new(r: &'a mut R, size: u64) -> Self {
        Windowed {
            r,
            size,
            at: 0,
            buf: Vec::new(),
        }
    }
}

impl<R: ReadAt> ReadAt for Windowed<'_, R> {
    fn read_at(&mut self, off: u64, out: &mut [u8]) -> Result<()> {
        let end = off + out.len() as u64;
        if out.len() > WINDOW || end > self.size {
            return self.r.read_at(off, out);
        }
        if off < self.at || end > self.at + self.buf.len() as u64 {
            let n = (self.size - off).min(WINDOW as u64) as usize;
            self.buf.resize(n, 0);
            self.at = off;
            if let Err(e) = self.r.read_at(off, &mut self.buf) {
                self.buf.clear();
                return Err(e);
            }
        }
        let a = (off - self.at) as usize;
        out.copy_from_slice(&self.buf[a..a + out.len()]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Counting(Vec<u8>, usize);
    impl ReadAt for Counting {
        fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
            self.1 += 1;
            self.0.read_at(off, buf)
        }
    }

    #[test]
    fn small_reads_share_a_window_and_match_the_source() {
        let data: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        let mut src = Counting(data.clone(), 0);
        let mut w = Windowed::new(&mut src, data.len() as u64);
        for off in (0..900_000u64).step_by(3000) {
            let mut b = [0u8; 100];
            w.read_at(off, &mut b).unwrap();
            assert_eq!(&b[..], &data[off as usize..off as usize + 100]);
        }
        let mut tail = [0u8; 50];
        w.read_at(2_999_950, &mut tail).unwrap();
        assert_eq!(&tail[..], &data[2_999_950..]);
        let mut big = vec![0u8; WINDOW + 1];
        w.read_at(10, &mut big).unwrap();
        assert_eq!(&big[..], &data[10..10 + WINDOW + 1]);
        assert!(w.read_at(2_999_990, &mut [0u8; 20]).is_err());
        drop(w);
        assert_eq!(
            src.1, 4,
            "one window, the tail, the long read and the failing one"
        );
    }
}
