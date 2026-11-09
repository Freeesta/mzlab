//! The `.mzidx` of a vendor file set (`docs/agenti/mzidx.md`, version 2), straight from a `Lettore`, with no mzML written and read again:
//! the rows are the ones `scantable` reads from the header of each spectrum as `lettori_mzml::scrivi` would write it (built from the scan itself,
//! with no text), and the spectra given to the builder are the ones the mzML would give back. Only the first two columns of a row change:
//! the `Posizione` of the scan in the members of the set and its number `k` in `Lettore::index`.

use crate::error::Result;
use crate::lettori::{Reader, ScanEntry};
use crate::lettori_centroidi::Centroidato;
use crate::lettori_mzml;
use crate::mzidx::First;
use crate::mzidx_spill::Spill;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{self, MzmlWriter};
use crate::scantable::{Dove, Rows, Table};

/// Scan `k` of sample `c` as the mzML written by `lettori_mzml::scrivi` reads back: the arrays cut to the shorter of m/z and intensity.
pub(crate) fn spettro(l: &mut dyn Reader, c: usize, k: usize) -> Result<Spectrum> {
    taglia(l.scan(c, k)?)
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
/// reads from the header text `mzmlwrite::spectrum_head` would write (the writer is only used for a scan whose extras could change that text: precursors
/// and cvParams of `precursori`, accessions in ids, filters or userParams).
/// `n_chromatograms` counts, as in the mzML, the chromatograms the writer writes (a kind it knows, at least one point).
pub fn righe(l: &mut dyn Reader, c: usize) -> Result<Table> {
    table(l, c, None)
}

/// The table and pass 1 of the `.mzidx` builder in one decoding of the scans: what `righe` gives and the groups `mzidx::build_dopo` needs.
pub(crate) fn righe_prima<'a>(
    l: &mut dyn Reader,
    c: usize,
    budget: usize,
    spill: Option<&'a mut dyn Spill>,
) -> Result<(Table, First<'a>)> {
    let mut before = First::new(budget, spill);
    let t = table(l, c, Some(&mut before))?;
    Ok((t, before))
}

fn table(l: &mut dyn Reader, c: usize, mut before: Option<&mut First<'_>>) -> Result<Table> {
    let mut rows = righe_vuote(l, c)?;
    let n = l.index(c)?.len();
    for k in 0..n {
        let s = spettro(l, c, k)?;
        row(&mut rows, l, k, &s, before.as_deref_mut())?;
    }
    Ok(rows.finish(cromatogrammi_scritti(l, c)?))
}

/// The rows of a table with no scan yet, from the header of the reader.
fn righe_vuote(l: &mut dyn Reader, c: usize) -> Result<Rows> {
    let h = lettori_mzml::table_header(l, c)?;
    Ok(match Rows::da_testata(&h) {
        Some(r) => r,
        None => Rows::from_head(&MzmlWriter::new(Vec::new(), &h)?.finish()?),
    })
}

/// The row of scan `k` (`s`), and the scan given to the groups of the builder.
fn row(
    rows: &mut Rows,
    l: &mut dyn Reader,
    k: usize,
    s: &Spectrum,
    before: Option<&mut First<'_>>,
) -> Result<()> {
    let extra = l.extra_mzml(s);
    // the total `spectrum_head` writes: the maker's, else the sum of the points
    let tic = extra
        .tic
        .unwrap_or_else(|| (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum());
    if !rows.reader(s, &extra, tic) {
        rows.spectrum(0, 0, &mzmlwrite::spectrum_head(k, s, &extra));
    }
    if let Some(p) = before {
        // the level and polarity of the row, as `build` reads them from the table
        let (level, pol) = rows.last().unwrap_or((s.level, 0));
        if level == 1 {
            p.indovina(rows.ultima_hr());
        }
        p.add(level, pol, s)?;
    }
    Ok(())
}

fn cromatogrammi_scritti(l: &mut dyn Reader, c: usize) -> Result<usize> {
    let mut n_chromatograms = 0;
    for v in l.chromatograms(c)? {
        let t = l.chromatogram(c, &v.id)?;
        if !t.time.is_empty() {
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
    before: &mut First<'_>,
    prima_cen: &mut First<'_>,
) -> Result<(Table, Table)> {
    let mut rows = righe_vuote(l.simple(), c)?;
    let mut rows_cen = righe_vuote(l, c)?;
    let n = l.index(c)?.len();
    for k in 0..n {
        let s = taglia(l.grezza(c, k)?)?;
        row(&mut rows, l.simple(), k, &s, Some(&mut *before))?;
        drop(s);
        let s = spettro(l, c, k)?;
        row(&mut rows_cen, l, k, &s, Some(&mut *prima_cen))?;
    }
    let ncr = cromatogrammi_scritti(l.simple(), c)?;
    Ok((rows.finish(ncr), rows_cen.finish(ncr)))
}

/// The rows of version 2: `[membro, offset, lunghezza]` (or `null` when the reader gives no position) and `k` in place of `start, end`.
pub fn rows_json(t: &mut Table, entries: &[ScanEntry]) -> String {
    for (k, r) in t.rows.iter_mut().enumerate() {
        r.at = Some(Box::new(Dove {
            k,
            position: entries.get(k).and_then(|v| v.position),
        }));
    }
    t.rows_json()
}
