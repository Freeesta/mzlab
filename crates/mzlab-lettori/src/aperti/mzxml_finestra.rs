//! Reads of `mzxml` that are not one scan at a time: the window that serves the short reads of the index, and the TIC / base peak of a scan straight from its bytes.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;

use super::mzxml::{number, CAP};
use super::mzxml_scan as scan;

/// The window of the pass that builds the index.
const WINDOW: u64 = 512 << 10;

/// A window over the file that serves the many short reads of the index (one per scan) from a few large ones.
#[derive(Default)]
pub(super) struct Window {
    da: u64,
    byte: Vec<u8>,
}

impl Window {
    /// The `n` bytes at `da` (the caller keeps `da + n` within the file).
    pub(super) fn read(
        &mut self,
        data: &mut dyn ReadAt,
        dim: u64,
        da: u64,
        n: u64,
    ) -> Result<&[u8]> {
        let fine = da.checked_add(n).ok_or_else(Error::memory_budget)?;
        let inside = da >= self.da && fine <= self.da + self.byte.len() as u64;
        if !inside {
            let voluta = n.max(WINDOW).min(dim.saturating_sub(da));
            let mut b = Vec::new();
            b.try_reserve_exact(usize::try_from(voluta).map_err(|_| Error::memory_budget())?)
                .map_err(|_| Error::memory_budget())?;
            b.resize(voluta as usize, 0);
            data.read_at(da, &mut b)?;
            (self.da, self.byte) = (da, b);
        }
        let i = (da - self.da) as usize;
        Ok(&self.byte[i..i + n as usize])
    }
}

/// `(TIC, base peak intensity)` of the scan whose bytes are `byte`, as its spectrum would give them (a 32-bit file's values are `f32`), without building the spectrum.
pub(super) fn peak_sums(byte: &[u8], limits: &Limiti) -> Result<(f64, f64)> {
    let text = String::from_utf8_lossy(byte);
    let t = scan::testa(&text, CAP)?;
    let Some((g, corpo)) = t.peaks else {
        return Ok((0.0, 0.0));
    };
    let conteggio = number(t.scan, "peaksCount").map(|n| n.max(0.0) as usize);
    let p = scan::peaks(g, corpo, conteggio, limits)?;
    let y = |&x: &f64| if p.a_64_bit { x } else { f64::from(x as f32) };
    Ok((
        p.intensities.iter().map(y).sum(),
        p.intensities.iter().map(y).fold(0.0, f64::max),
    ))
}
