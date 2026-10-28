//! Reads of `mzxml` that are not one scan at a time: the window that serves the short reads of the index, and the TIC / base peak of a scan straight from its bytes.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;

use super::mzxml::{numero, TETTO};
use super::mzxml_scan as scan;

/// The window of the pass that builds the index.
const FINESTRA: u64 = 512 << 10;

/// A window over the file that serves the many short reads of the index (one per scan) from a few large ones.
#[derive(Default)]
pub(super) struct Finestra {
    da: u64,
    byte: Vec<u8>,
}

impl Finestra {
    /// The `n` bytes at `da` (the caller keeps `da + n` within the file).
    pub(super) fn leggi(
        &mut self,
        dati: &mut dyn ReadAt,
        dim: u64,
        da: u64,
        n: u64,
    ) -> Result<&[u8]> {
        let fine = da.checked_add(n).ok_or_else(Error::memory_budget)?;
        let dentro = da >= self.da && fine <= self.da + self.byte.len() as u64;
        if !dentro {
            let voluta = n.max(FINESTRA).min(dim.saturating_sub(da));
            let mut b = Vec::new();
            b.try_reserve_exact(usize::try_from(voluta).map_err(|_| Error::memory_budget())?)
                .map_err(|_| Error::memory_budget())?;
            b.resize(voluta as usize, 0);
            dati.read_at(da, &mut b)?;
            (self.da, self.byte) = (da, b);
        }
        let i = (da - self.da) as usize;
        Ok(&self.byte[i..i + n as usize])
    }
}

/// `(TIC, base peak intensity)` of the scan whose bytes are `byte`, as its spectrum would give them (a 32-bit file's values are `f32`), without building the spectrum.
pub(super) fn somme_picchi(byte: &[u8], limiti: &Limiti) -> Result<(f64, f64)> {
    let testo = String::from_utf8_lossy(byte);
    let t = scan::testa(&testo, TETTO)?;
    let Some((g, corpo)) = t.picchi else {
        return Ok((0.0, 0.0));
    };
    let conteggio = numero(t.scan, "peaksCount").map(|n| n.max(0.0) as usize);
    let p = scan::picchi(g, corpo, conteggio, limiti)?;
    let y = |&x: &f64| if p.a_64_bit { x } else { f64::from(x as f32) };
    Ok((
        p.intensita.iter().map(y).sum(),
        p.intensita.iter().map(y).fold(0.0, f64::max),
    ))
}
