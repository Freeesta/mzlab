//! The XIC of the index as doubles for `api/xicbin`: `[rt(n), y(n)]` with the numbers `Routes::xic` writes in its JSON. Used by the wasm engine and the desktop.

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::routes::Routes;

impl<R: ReadAt> Routes<R> {
    /// Scans of one level (the length of each half of the buffer `xic_into` fills).
    pub fn xic_len(&self, level: u8) -> usize {
        self.idx
            .section_of(level, self.pol)
            .map_or(0, |si| self.idx.sections[si].n_scans)
    }

    /// The XIC of one level into `out` as `[rt(n), y(n)]`, rounded like `xic` (RT to 4 decimals, y to 1) so the numbers
    /// are those `JSON.parse(xic)` gives. Returns n (0 if the section is missing or `out` is shorter than `2 * n`).
    pub fn xic_into(&mut self, mz: f64, tol: f64, level: u8, out: &mut [f64]) -> Result<usize> {
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok(0);
        };
        let n = self.idx.sections[si].n_scans;
        if out.len() < 2 * n {
            return Ok(0);
        }
        let (lo, hi) = (mz - tol, mz + tol);
        let a = self.idx.search(si, lo, false)?;
        let b = self.idx.search(si, hi, true)?;
        let y = if b <= a {
            vec![0.0; n]
        } else {
            self.idx.sum_window(si, a, b)?
        };
        let rt = &self.idx.arrays(si)?.rt;
        for i in 0..n {
            out[i] = crate::json::round(rt[i], 4);
            out[n + i] = crate::json::round(y[i], 1);
        }
        Ok(n)
    }
}
