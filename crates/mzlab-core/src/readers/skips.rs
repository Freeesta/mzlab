//! A spectrum whose own bytes do not decode (an `err.file.*` error of `Reader::scan`) is left out of a whole-sample read, never the sample: the loops over
//! the spectra (`readers_mzml`, `mzidx_reader`) put the row of the index with no points in its place, so the counts and ids stay, the guard
//! (`readers_finite::Finished`) counts it in `Reader::losses` with `SPECTRUM_DAMAGED`, and a sample with no spectrum read at all is refused with the first cause.

use super::{Reader, ScanEntry};
use crate::error::{Error, Result};
use crate::mzml::Spectrum;

/// Cause of the spectra left out of a read because their own data do not decode (`Skips`).
pub const SPECTRUM_DAMAGED: &str = "warn.file.damaged";

/// True for an error of one spectrum's own data (`err.file.*`), which leaves that spectrum out; any other (memory, index, cancel) stops the read.
pub fn damages_one(e: &Error) -> bool {
    e.key.starts_with("err.file.")
}

/// What a loop over the `n` spectra of a sample has read: how many decoded, and the first error of one that did not.
#[derive(Debug, Default)]
pub struct Skips {
    kept: usize,
    first: Option<Error>,
}

impl Skips {
    /// Scan `k` of sample `c` as `s` gave it, or the row of the index with no points in place of one whose data do not decode.
    pub fn take(
        &mut self,
        l: &mut dyn Reader,
        (c, k): (usize, usize),
        s: Result<Spectrum>,
    ) -> Result<Spectrum> {
        match s {
            Ok(s) => {
                self.kept += 1;
                Ok(s)
            }
            Err(e) if damages_one(&e) => {
                // a scan with no row of its own cannot be put in its place: the read stops
                let Some(s) = l.index_without_tic(c)?.get(k).map(|r| empty(k, r)) else {
                    return Err(e);
                };
                self.first.get_or_insert(e);
                Ok(s)
            }
            Err(e) => Err(e),
        }
    }
    /// The end of the loop: a sample of `n` spectra none of which decoded is refused with the first cause (nothing at all could be read).
    pub fn end(self, n: usize) -> Result<()> {
        match self.first {
            Some(e) if n > 0 && self.kept == 0 => Err(e),
            _ => Ok(()),
        }
    }
}

/// Spectrum `k` with the header of its index row and no points.
fn empty(k: usize, r: &ScanEntry) -> Spectrum {
    Spectrum {
        index: k,
        id: r.id.clone(),
        level: r.livello,
        rt: r.rt,
        polarity: r.polarita,
        filter: r.filtro.clone(),
        precursor: r.precursore,
        ..Spectrum::default()
    }
}
