//! Windows of the peak table of an index: the peaks `a..b` handed to a closure, and the per-scan sums and maxima of an XIC. Used by every extraction of the engine.

use crate::error::{Error, Result};
use crate::idxread::{Index, ReadAt};

impl<R: ReadAt> Index<R> {
    /// `pos`, m/z and `inten` of the peaks `a..b`, handed to `f` in pieces of at most 64 K peaks, in table order.
    pub fn each_peak(
        &mut self,
        si: usize,
        a: u64,
        b: u64,
        f: impl FnMut(usize, f64, f64),
    ) -> Result<()> {
        self.walk::<true>(si, a, b, f)
    }

    /// Like `each_peak` without the m/z: the array is not read (about 40% fewer bytes), for the sums and maxima of an XIC.
    pub fn each_inten(
        &mut self,
        si: usize,
        a: u64,
        b: u64,
        mut f: impl FnMut(usize, f64),
    ) -> Result<()> {
        self.walk::<false>(si, a, b, |p, _, y| f(p, y))
    }

    /// The reading behind `each_peak` / `each_inten`; with `MZ` false the m/z array is left alone and `f` gets 0. A `pos` outside the section's scans is a damaged index.
    fn walk<const MZ: bool>(
        &mut self,
        si: usize,
        a: u64,
        b: u64,
        mut f: impl FnMut(usize, f64, f64),
    ) -> Result<()> {
        const STEP: u64 = 1 << 16;
        let (off, f4, n_scans, n_peaks) = {
            let s = &self.sections[si];
            (s.off, s.f4, s.n_scans as u64, s.n_peaks)
        };
        // a window past the table holds no peaks (the bytes there belong to other arrays)
        let b = b.min(n_peaks);
        let w = if f4 { 4 } else { 8 } as u64;
        // one work buffer for the whole call, sized for the first piece: intensities, then m/z, then positions
        let first = b.saturating_sub(a).min(STEP) as usize;
        let mut work = Vec::new();
        work.try_reserve_exact(first * (2 * w as usize + 4))
            .map_err(|_| Error::memory_budget())?;
        let mut i = a;
        {
            while i < b {
                let c = (b - i).min(STEP) as usize;
                let cw = c * w as usize;
                if work.len() < cw * 2 + c * 4 {
                    work.resize(cw * 2 + c * 4, 0);
                }
                let (ib, rest) = work.split_at_mut(cw);
                let (mb, pb) = rest.split_at_mut(cw);
                self.r.read_at(off[8] + i * w, ib)?;
                if MZ {
                    self.r.read_at(off[7] + i * w, mb)?;
                }
                let pb = &mut pb[..c * 4];
                self.r.read_at(off[9] + i * 4, pb)?;
                let (ib, mb, pb) = (&ib[..cw], &mb[..cw], &pb[..c * 4]);
                // the three arrays walked in step by iterators: no index arithmetic or bounds check per peak
                let mut ps = pb.chunks_exact(4);
                if f4 {
                    let (mut is, mut ms) = (ib.chunks_exact(4), mb.chunks_exact(4));
                    for _ in 0..c {
                        let pos = pos_of(ps.next().unwrap(), n_scans)?;
                        let y = f64::from(f32::from_le_bytes(is.next().unwrap().try_into().unwrap()));
                        let mz = if MZ { f64::from(f32::from_le_bytes(ms.next().unwrap().try_into().unwrap())) } else { 0.0 };
                        f(pos, mz, y);
                    }
                } else {
                    let (mut is, mut ms) = (ib.chunks_exact(8), mb.chunks_exact(8));
                    for _ in 0..c {
                        let pos = pos_of(ps.next().unwrap(), n_scans)?;
                        let y = f64::from_le_bytes(is.next().unwrap().try_into().unwrap());
                        let mz = if MZ { f64::from_le_bytes(ms.next().unwrap().try_into().unwrap()) } else { 0.0 };
                        f(pos, mz, y);
                    }
                }
                i += c as u64;
            }
        }
        Ok(())
    }

    /// Summed intensity of the peaks `a..b` in every scan of the section (`np.bincount(pos, weights=inten, minlength=n)`).
    pub fn sum_window(&mut self, si: usize, a: u64, b: u64) -> Result<Vec<f64>> {
        let n = self.sections[si].n_scans;
        let mut out = vec![0.0; n];
        self.each_inten(si, a, b, |p, y| {
            if p < n {
                out[p] += y;
            }
        })?;
        Ok(out)
    }

    /// Highest intensity of the peaks `a..b` in every scan (0 where there are none).
    pub fn max_window(&mut self, si: usize, a: u64, b: u64) -> Result<Vec<f64>> {
        let n = self.sections[si].n_scans;
        let mut out = vec![0.0; n];
        self.each_inten(si, a, b, |p, y| {
            if p < n && y > out[p] {
                out[p] = y;
            }
        })?;
        Ok(out)
    }
}

/// The scan of a peak from its 4 bytes; a position outside the section's scans is a damaged index.
#[inline(always)]
fn pos_of(b: &[u8], n_scans: u64) -> Result<usize> {
    let pos = u32::from_le_bytes(b.try_into().unwrap());
    if u64::from(pos) >= n_scans {
        return Err(Error::new("error.badIndex"));
    }
    Ok(pos as usize)
}
