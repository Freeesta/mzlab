//! Windows of the peak table of an index: the peaks `a..b` handed to a closure, and the per-scan sums and maxima of an XIC. Used by every extraction of the engine.

use crate::error::{Error, Result};
use crate::idxread::{index_bytes, Index, ReadAt};

impl<R: ReadAt> Index<R> {
    /// `pos`, m/z and `inten` of the peaks `a..b`, handed to `f` in pieces of at most 64 K peaks, in table order.
    pub fn each_peak(
        &mut self,
        si: usize,
        a: u64,
        b: u64,
        f: impl FnMut(usize, f64, f64),
    ) -> Result<()> {
        self.walk::<true, true>(si, a, b, f)
    }

    /// Like `each_peak` without the m/z: the array is not read (about 40% fewer bytes), for the sums and maxima of an XIC.
    pub fn each_inten(
        &mut self,
        si: usize,
        a: u64,
        b: u64,
        mut f: impl FnMut(usize, f64),
    ) -> Result<()> {
        self.walk::<false, true>(si, a, b, |p, _, y| f(p, y))
    }

    /// Like `each_peak` without the intensity: the array is not read (about 40% fewer bytes), for the m/z spacing of profile scans.
    pub fn each_mz(
        &mut self,
        si: usize,
        a: u64,
        b: u64,
        mut f: impl FnMut(usize, f64),
    ) -> Result<()> {
        self.walk::<true, false>(si, a, b, |p, mz, _| f(p, mz))
    }

    /// Like `each_peak` over the whole section, but only for scans with `lut[pos] != u32::MAX`, passing `(lut[pos], mz, inten)` to `f`.
    /// Reads `pos` first for each block and skips reading `mz` and `inten` on blocks where no peak belongs to the selected scans.
    pub fn each_peak_for(
        &mut self,
        si: usize,
        lut: &[u32],
        mut f: impl FnMut(usize, f64, f64),
    ) -> Result<()> {
        const STEP: u64 = 1 << 14;
        let (off, f4, n_scans, b) = {
            let s = &self.sections[si];
            (s.off, s.f4, s.n_scans as u64, s.n_peaks)
        };
        if lut.len() < n_scans as usize {
            return Err(Error::new("error.badIndex"));
        }
        let w = if f4 { 4 } else { 8 } as u64;
        let first = b.min(STEP) as usize;
        let mut work = Vec::new();
        work.try_reserve_exact(first * (2 * w as usize + 4))
            .map_err(|_| Error::memory_budget())?;
        let mut i = 0;
        while i < b {
            let c = (b - i).min(STEP) as usize;
            let cw = c * w as usize;
            if work.len() < cw * 2 + c * 4 {
                work.resize(cw * 2 + c * 4, 0);
            }
            let (ib, rest) = work.split_at_mut(cw);
            let (mb, pb) = rest.split_at_mut(cw);
            let pb = &mut pb[..c * 4];
            index_bytes(&mut self.r, off[9] + i * 4, pb)?;
            if let Some((lo, hi)) = block_span(pb, n_scans, lut)? {
                let span = (hi - lo + 1) * w as usize;
                let base = (i + lo as u64) * w;
                index_bytes(&mut self.r, off[8] + base, &mut ib[..span])?;
                index_bytes(&mut self.r, off[7] + base, &mut mb[..span])?;
                decode_selected(
                    &pb[lo * 4..(hi + 1) * 4],
                    &ib[..span],
                    &mb[..span],
                    f4,
                    lut,
                    &mut f,
                );
            }
            i += c as u64;
        }
        Ok(())
    }

    /// The reading behind `each_peak` / `each_inten` / `each_mz`; unused arrays are left alone and `f` gets 0 for them. A `pos` outside the section's scans is a damaged index.
    fn walk<const MZ: bool, const INTEN: bool>(
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
        while i < b {
            let c = (b - i).min(STEP) as usize;
            let cw = c * w as usize;
            if work.len() < cw * 2 + c * 4 {
                work.resize(cw * 2 + c * 4, 0);
            }
            let (ib, rest) = work.split_at_mut(cw);
            let (mb, pb) = rest.split_at_mut(cw);
            if INTEN {
                index_bytes(&mut self.r, off[8] + i * w, ib)?;
            }
            if MZ {
                index_bytes(&mut self.r, off[7] + i * w, mb)?;
            }
            let pb = &mut pb[..c * 4];
            index_bytes(&mut self.r, off[9] + i * 4, pb)?;
            decode_block::<MZ, INTEN>(&pb[..c * 4], &ib[..cw], &mb[..cw], c, f4, n_scans, &mut f)?;
            i += c as u64;
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

/// First and last peak indices in `pb` whose scan is selected in `lut` (`lut[pos] != u32::MAX`).
fn block_span(pb: &[u8], n_scans: u64, lut: &[u32]) -> Result<Option<(usize, usize)>> {
    let (mut lo, mut hi) = (usize::MAX, 0usize);
    let n = pb.len() / 4;
    for j in 0..n {
        let pos = pos_of(&pb[j * 4..j * 4 + 4], n_scans)?;
        if lut[pos] != u32::MAX {
            if lo == usize::MAX {
                lo = j;
            }
            hi = j;
        }
    }
    Ok((lo != usize::MAX).then_some((lo, hi)))
}

/// Decodes and hands to `f` only the peaks in `pb` whose scan is selected in `lut`.
fn decode_selected(
    pb: &[u8],
    ib: &[u8],
    mb: &[u8],
    f4: bool,
    lut: &[u32],
    f: &mut impl FnMut(usize, f64, f64),
) {
    let n = pb.len() / 4;
    if f4 {
        for j in 0..n {
            let p4 = j * 4;
            let k = lut[u32::from_le_bytes(pb[p4..p4 + 4].try_into().unwrap()) as usize];
            if k != u32::MAX {
                let y = f64::from(f32::from_le_bytes(ib[p4..p4 + 4].try_into().unwrap()));
                let mz = f64::from(f32::from_le_bytes(mb[p4..p4 + 4].try_into().unwrap()));
                f(k as usize, mz, y);
            }
        }
    } else {
        for j in 0..n {
            let p4 = j * 4;
            let k = lut[u32::from_le_bytes(pb[p4..p4 + 4].try_into().unwrap()) as usize];
            if k != u32::MAX {
                let at = j * 8;
                let y = f64::from_le_bytes(ib[at..at + 8].try_into().unwrap());
                let mz = f64::from_le_bytes(mb[at..at + 8].try_into().unwrap());
                f(k as usize, mz, y);
            }
        }
    }
}

/// Decodes one block of `c` peaks for `walk`.
fn decode_block<const MZ: bool, const INTEN: bool>(
    pb: &[u8],
    ib: &[u8],
    mb: &[u8],
    c: usize,
    f4: bool,
    n_scans: u64,
    f: &mut impl FnMut(usize, f64, f64),
) -> Result<()> {
    let mut ps = pb.chunks_exact(4);
    if f4 {
        let (mut is, mut ms) = (ib.chunks_exact(4), mb.chunks_exact(4));
        for _ in 0..c {
            let pos = pos_of(ps.next().unwrap(), n_scans)?;
            let y = if INTEN {
                f64::from(f32::from_le_bytes(is.next().unwrap().try_into().unwrap()))
            } else {
                0.0
            };
            let mz = if MZ {
                f64::from(f32::from_le_bytes(ms.next().unwrap().try_into().unwrap()))
            } else {
                0.0
            };
            f(pos, mz, y);
        }
    } else {
        let (mut is, mut ms) = (ib.chunks_exact(8), mb.chunks_exact(8));
        for _ in 0..c {
            let pos = pos_of(ps.next().unwrap(), n_scans)?;
            let y = if INTEN {
                f64::from_le_bytes(is.next().unwrap().try_into().unwrap())
            } else {
                0.0
            };
            let mz = if MZ {
                f64::from_le_bytes(ms.next().unwrap().try_into().unwrap())
            } else {
                0.0
            };
            f(pos, mz, y);
        }
    }
    Ok(())
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
