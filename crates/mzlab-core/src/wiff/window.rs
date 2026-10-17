//! A read window over a `.wiff.scan`: the scan is read front to back, so one big read serves many small ones (the heads of the blocks, then each block)
//! and a JS reader is called once per window instead of once per block. A read outside the window moves the window; a read as large as the window goes straight through.

use crate::error::Result;
use crate::idxread::ReadAt;

const SIZE: usize = 1 << 20;

pub struct Window<'a, R: ReadAt> {
    inner: &'a mut R,
    len: u64,
    start: u64,
    buf: Vec<u8>,
}

impl<'a, R: ReadAt> Window<'a, R> {
    /// `len`: the size of the file (a window never reads past it).
    pub fn new(inner: &'a mut R, len: u64) -> Self {
        Window {
            inner,
            len,
            start: 0,
            buf: Vec::new(),
        }
    }
}

impl<R: ReadAt> ReadAt for Window<'_, R> {
    fn read_at(&mut self, off: u64, out: &mut [u8]) -> Result<()> {
        let end = off.saturating_add(out.len() as u64);
        if out.len() >= SIZE || end > self.len {
            return self.inner.read_at(off, out);
        }
        if off < self.start || end > self.start + self.buf.len() as u64 {
            let n = (self.len - off).min(SIZE as u64) as usize;
            self.buf.resize(n, 0);
            self.start = off;
            if let Err(e) = self.inner.read_at(off, &mut self.buf) {
                self.buf.clear();
                return Err(e);
            }
        }
        let at = (off - self.start) as usize;
        out.copy_from_slice(&self.buf[at..at + out.len()]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Count(Vec<u8>, usize);
    impl ReadAt for Count {
        fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
            self.1 += 1;
            self.0.read_at(off, buf)
        }
    }

    #[test]
    fn small_reads_come_from_one_big_one() {
        let data: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        let mut c = Count(data.clone(), 0);
        let mut w = Window::new(&mut c, data.len() as u64);
        let mut b = [0u8; 100];
        for off in [0u64, 500, 90_000, 1_040_000, 10, 2_999_900] {
            w.read_at(off, &mut b).unwrap();
            assert_eq!(&b[..], &data[off as usize..off as usize + 100]);
        }
        assert!(w.read_at(2_999_950, &mut b).is_err());
        assert_eq!(c.1, 3); // the first window, the one at the end, the read past the end
    }
}
