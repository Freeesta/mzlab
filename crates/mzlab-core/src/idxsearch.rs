//! Search in the m/z array of an index: the element at a position, the range of a section and `np.searchsorted`, through the fence and the block cache.

use crate::error::Result;
use crate::idxread::{Index, ReadAt, BLOCK};

impl<R: ReadAt> Index<R> {
    /// Element `i` of the m/z array (as `f64`; the `f32` of a high-resolution survey is widened exactly), through the block cache.
    pub(crate) fn mz_at(&mut self, si: usize, i: u64) -> Result<f64> {
        let s = &self.sections[si];
        let (w, off) = (if s.f4 { 4 } else { 8 }, s.off[7]);
        let blk = i / BLOCK;
        let n = s.n_peaks;
        let at = self
            .cache
            .iter()
            .position(|c| c.0 == si && c.1 == 7 && c.2 == blk);
        let at = match at {
            Some(p) => p,
            None => {
                let first = blk * BLOCK;
                let cnt = (n - first).min(BLOCK) as usize;
                let mut b = vec![0u8; cnt * w];
                self.r.read_at(off + first * w as u64, &mut b)?;
                if self.cache.len() >= 8 {
                    // the least recently used block, not the first read: the blocks of a search stay while a window is read
                    if let Some(old) = (0..self.cache.len()).min_by_key(|&k| self.cache[k].4) {
                        self.cache.remove(old);
                    }
                }
                self.cache.push((si, 7, blk, b, 0));
                self.cache.len() - 1
            }
        };
        let b = self.touch_block(at);
        let k = (i - blk * BLOCK) as usize * w;
        Ok(if w == 4 {
            f32::from_le_bytes(b[k..k + 4].try_into().unwrap()) as f64
        } else {
            f64::from_le_bytes(b[k..k + 8].try_into().unwrap())
        })
    }

    /// Lowest and highest m/z of a section (`mz[0]`, `mz[-1]` of its peak table), None when it has no peaks.
    pub fn mz_range(&mut self, si: usize) -> Result<Option<(f64, f64)>> {
        let n = self.sections[si].n_peaks;
        if n == 0 {
            return Ok(None);
        }
        Ok(Some((self.mz_at(si, 0)?, self.mz_at(si, n - 1)?)))
    }

    /// How many blocks start with an m/z for which `pred` holds, from the fence of the section; `None` when the section has no usable fence
    /// (no peaks, or a `n_fence` that is not the number of blocks: the caller then searches the whole array).
    fn fence_blocks(&mut self, si: usize, pred: impl Fn(f64) -> bool) -> Result<Option<usize>> {
        if self.fences[si].is_none() {
            let s = &self.sections[si];
            let (w, n_f) = (if s.f4 { 4 } else { 8 }, s.n_fence);
            let ok = s.n_peaks > 0 && n_f as u64 == s.n_peaks.div_ceil(BLOCK) && n_f <= 1 << 24;
            let mut fence = Vec::new();
            if ok {
                let mut b = vec![0u8; n_f * w];
                self.r.read_at(s.off[6], &mut b)?;
                fence = b
                    .chunks_exact(w)
                    .map(|c| {
                        if w == 4 {
                            f64::from(f32::from_le_bytes(c.try_into().unwrap()))
                        } else {
                            f64::from_le_bytes(c.try_into().unwrap())
                        }
                    })
                    .collect();
            }
            self.fences[si] = Some(fence);
        }
        let f = self.fences[si].as_deref().unwrap_or_default();
        Ok((!f.is_empty()).then(|| f.partition_point(|&x| pred(x))))
    }

    /// `np.searchsorted(mz, v, side)` where `v` has the dtype of the array (a high-resolution survey compares in `f32`).
    pub fn search(&mut self, si: usize, v: f64, right: bool) -> Result<u64> {
        let v = if self.sections[si].f4 {
            v as f32 as f64
        } else {
            v
        };
        let n = self.sections[si].n_peaks;
        let pred = |x: f64| if right { x <= v } else { x < v };
        let (mut lo, mut hi) = (0u64, n);
        // the fence (first m/z of every block) says which one block holds the answer: one read instead of ~log2(n/16384)
        if let Some(k) = self.fence_blocks(si, pred)? {
            if k == 0 {
                return Ok(0);
            }
            (lo, hi) = ((k as u64 - 1) * BLOCK + 1, (k as u64 * BLOCK).min(n));
        }
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let x = self.mz_at(si, mid)?;
            if pred(x) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        Ok(lo)
    }
}
