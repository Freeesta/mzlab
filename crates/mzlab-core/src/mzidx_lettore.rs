//! The `.mzidx` of a vendor file set (`docs/agenti/mzidx.md`, version 2), straight from a `Lettore`, with no mzML written and read again:
//! the rows are the ones `scantable` reads from the header of each spectrum as `lettori_mzml::scrivi` would write it (built from the scan itself,
//! with no text), and the spectra given to the builder are the ones the mzML would give back. Only the first two columns of a row change:
//! the `Posizione` of the scan in the members of the set and its number `k` in `Lettore::indice`.

use crate::error::Result;
use crate::lettori::{Lettore, VoceScansione};
use crate::lettori_mzml;
use crate::mzidx::Prima;
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

/// The scan table of sample `c`: every scan is decoded once and its row is built from the scan itself (`Rows::lettore`), the row that `Rows::spectrum`
/// reads from the header text `mzmlwrite::spectrum_head` would write (the writer is only used for a scan whose extras could change that text).
/// `n_chromatograms` counts, as in the mzML, the chromatograms the writer writes (a kind it knows, at least one point).
pub fn righe(l: &mut dyn Lettore, c: usize) -> Result<Table> {
    tabella(l, c, None)
}

/// The table and pass 1 of the `.mzidx` builder in one decoding of the scans: what `righe` gives and the groups `mzidx::build_dopo` needs.
pub(crate) fn righe_prima(l: &mut dyn Lettore, c: usize, budget: usize) -> Result<(Table, Prima)> {
    let mut prima = Prima::new(budget);
    let t = tabella(l, c, Some(&mut prima))?;
    Ok((t, prima))
}

fn tabella(l: &mut dyn Lettore, c: usize, mut prima: Option<&mut Prima>) -> Result<Table> {
    let h = lettori_mzml::testata(l, c)?;
    let mut rows = match Rows::da_testata(&h) {
        Some(r) => r,
        None => Rows::from_head(&MzmlWriter::new(Vec::new(), &h)?.finish()?),
    };
    drop(h);
    let n = l.indice(c)?.len();
    for k in 0..n {
        let s = spettro(l, c, k)?;
        let extra = l.extra_mzml(&s);
        // the sum `spectrum_head` writes
        let tic: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
        if !rows.lettore(&s, &extra, tic) {
            rows.spectrum(0, 0, &mzmlwrite::spectrum_head(k, &s, &extra));
        }
        if let Some(p) = prima.as_deref_mut() {
            let pol = match s.polarity {
                1 => 1,
                -1 => -1,
                _ => 0,
            };
            p.add(s.level, pol, &s)?;
        }
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
