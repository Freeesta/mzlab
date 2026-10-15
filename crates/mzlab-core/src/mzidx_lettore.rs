//! The `.mzidx` of a vendor file set (`docs/agenti/mzidx.md`, version 2), straight from a `Lettore`, with no mzML written and read again:
//! the rows are the ones `scantable` reads from the header of each spectrum as `lettori_mzml::scrivi` would write it (built from the scan itself,
//! with no text), and the spectra given to the builder are the ones the mzML would give back. Only the first two columns of a row change:
//! the `Posizione` of the scan in the members of the set and its number `k` in `Lettore::indice`.

use crate::error::Result;
use crate::lettori::{Lettore, VoceScansione};
use crate::lettori_centroidi::Centroidato;
use crate::lettori_mzml;
use crate::mzidx::Prima;
use crate::mzidx_spill::Spill;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{self, MzmlWriter};
use crate::scantable::{Rows, Table};

/// Scan `k` of sample `c` as the mzML written by `lettori_mzml::scrivi` reads back: the arrays cut to the shorter of m/z and intensity.
pub(crate) fn spettro(l: &mut dyn Lettore, c: usize, k: usize) -> Result<Spectrum> {
    taglia(l.scansione(c, k)?)
}

/// The arrays of `s` cut to the shorter of m/z and intensity.
pub(crate) fn taglia(mut s: Spectrum) -> Result<Spectrum> {
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
pub(crate) fn righe_prima<'a>(
    l: &mut dyn Lettore,
    c: usize,
    budget: usize,
    spill: Option<&'a mut dyn Spill>,
) -> Result<(Table, Prima<'a>)> {
    let mut prima = Prima::new(budget, spill);
    let t = tabella(l, c, Some(&mut prima))?;
    Ok((t, prima))
}

fn tabella(l: &mut dyn Lettore, c: usize, mut prima: Option<&mut Prima<'_>>) -> Result<Table> {
    let mut rows = righe_vuote(l, c)?;
    let n = l.indice(c)?.len();
    for k in 0..n {
        let s = spettro(l, c, k)?;
        riga(&mut rows, l, k, &s, prima.as_deref_mut())?;
    }
    Ok(rows.finish(cromatogrammi_scritti(l, c)?))
}

/// The rows of a table with no scan yet, from the header of the reader.
fn righe_vuote(l: &mut dyn Lettore, c: usize) -> Result<Rows> {
    let h = lettori_mzml::testata(l, c)?;
    Ok(match Rows::da_testata(&h) {
        Some(r) => r,
        None => Rows::from_head(&MzmlWriter::new(Vec::new(), &h)?.finish()?),
    })
}

/// The row of scan `k` (`s`), and the scan given to the groups of the builder.
fn riga(
    rows: &mut Rows,
    l: &mut dyn Lettore,
    k: usize,
    s: &Spectrum,
    prima: Option<&mut Prima<'_>>,
) -> Result<()> {
    let extra = l.extra_mzml(s);
    // the sum `spectrum_head` writes
    let tic: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
    if !rows.lettore(s, &extra, tic) {
        rows.spectrum(0, 0, &mzmlwrite::spectrum_head(k, s, &extra));
    }
    if let Some(p) = prima {
        // the level and polarity of the row, as `build` reads them from the table
        let (level, pol) = rows.ultima().unwrap_or((s.level, 0));
        p.add(level, pol, s)?;
    }
    Ok(())
}

fn cromatogrammi_scritti(l: &mut dyn Lettore, c: usize) -> Result<usize> {
    let mut n_chromatograms = 0;
    for v in l.cromatogrammi(c)? {
        let t = l.cromatogramma(c, &v.id)?;
        if !t.time.is_empty() && mzmlwrite::chromatogram_kind(&t).is_some() {
            n_chromatograms += 1;
        }
    }
    Ok(n_chromatograms)
}

/// The two tables of a sample (the scans as the file has them, and as centroids) and pass 1 of the two `.mzidx` builders, from ONE decoding of the scans:
/// every scan is decoded by the reader beneath `l` and `l` centroids that same scan.
pub(crate) fn righe_prima_doppie(
    l: &mut Centroidato,
    c: usize,
    prima: &mut Prima<'_>,
    prima_cen: &mut Prima<'_>,
) -> Result<(Table, Table)> {
    let mut rows = righe_vuote(l.semplice(), c)?;
    let mut rows_cen = righe_vuote(l, c)?;
    let n = l.indice(c)?.len();
    for k in 0..n {
        let s = taglia(l.grezza(c, k)?)?;
        riga(&mut rows, l.semplice(), k, &s, Some(&mut *prima))?;
        drop(s);
        let s = spettro(l, c, k)?;
        riga(&mut rows_cen, l, k, &s, Some(&mut *prima_cen))?;
    }
    let ncr = cromatogrammi_scritti(l.semplice(), c)?;
    Ok((rows.finish(ncr), rows_cen.finish(ncr)))
}

/// The rows of version 2: `[membro, offset, lunghezza]` (or `null` when the reader gives no position) and `k` in place of `start, end`.
pub fn rows_json(t: &mut Table, voci: &[VoceScansione]) -> String {
    for (k, r) in t.rows.iter_mut().enumerate() {
        r.k = Some(k);
        r.posizione = voci.get(k).and_then(|v| v.posizione);
    }
    t.rows_json()
}
