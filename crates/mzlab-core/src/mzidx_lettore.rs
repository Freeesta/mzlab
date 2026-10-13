//! The `.mzidx` of a vendor file set (`docs/agenti/mzidx.md`, version 2), straight from a `Lettore`, with no mzML written and read again:
//! the rows are the ones `scantable` reads from the header of each spectrum as `lettori_mzml::scrivi` would write it (the same text, so the
//! same parser), and the spectra given to the builder are the ones the mzML would give back. Only the first two columns of a row change:
//! the `Posizione` of the scan in the members of the set and its number `k` in `Lettore::indice`.

use crate::error::Result;
use crate::lettori::{Lettore, VoceScansione};
use crate::lettori_mzml;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{self, MzmlWriter};
use crate::scantable::{Rows, Table};

/// Scan `k` of sample `c` as the mzML written by `lettori_mzml::scrivi` reads back: the arrays cut to the shorter of m/z and intensity.
pub(crate) fn spettro(l: &mut dyn Lettore, c: usize, k: usize) -> Result<Spectrum> {
    let mut s = l.scansione(c, k)?;
    let n =
        s.mz.len()
            .min(s.wide.as_ref().map_or(s.intensity.len(), Vec::len));
    s.mz.truncate(n);
    s.intensity.truncate(n);
    if let Some(w) = s.wide.as_mut() {
        w.truncate(n);
    }
    Ok(s)
}

/// The scan table of sample `c`: every scan is decoded once, for the header text the writer would give it (TIC, base peak, profile).
/// `n_chromatograms` counts, as in the mzML, the chromatograms the writer writes (a kind it knows, at least one point).
pub fn righe(l: &mut dyn Lettore, c: usize) -> Result<Table> {
    let head = MzmlWriter::new(Vec::new(), &lettori_mzml::testata(l, c)?)?.finish()?;
    let mut rows = Rows::from_head(&head);
    drop(head);
    let n = l.indice(c)?.len();
    for k in 0..n {
        let s = l.scansione(c, k)?;
        rows.spectrum(0, 0, &mzmlwrite::spectrum_head(k, &s, &l.extra_mzml(&s)));
    }
    let mut n_chromatograms = 0;
    for v in l.cromatogrammi(c)? {
        let t = l.cromatogramma(c, &v.id)?;
        if !t.time.is_empty() && mzmlwrite::chromatogram_kind(&t).is_some() {
            n_chromatograms += 1;
        }
    }
    Ok(rows.finish(n_chromatograms))
}

/// The rows of version 2: `[membro, offset, lunghezza]` (or `null` when the reader gives no position) and `k` in place of `start, end`.
pub fn rows_json(t: &mut Table, voci: &[VoceScansione]) -> String {
    for (k, r) in t.rows.iter_mut().enumerate() {
        r.k = Some(k);
        r.posizione = voci.get(k).and_then(|v| v.posizione);
    }
    t.rows_json()
}
