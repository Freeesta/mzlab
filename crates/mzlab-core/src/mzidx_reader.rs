//! The `.mzidx` of a vendor file set (`docs/agenti/mzidx.md`, version 2), straight from a `Lettore`, with no mzML written and read again:
//! the rows are the ones `scantable` reads from the header of each spectrum as `readers_mzml::scrivi` would write it (built from the scan itself,
//! with no text), and the spectra given to the builder are the ones the mzML would give back. Only the first two columns of a row change:
//! the `Posizione` of the scan in the members of the set and its number `k` in `Lettore::index`.

use crate::error::Result;
use crate::mzidx::First;
use crate::mzidx_spill::Spill;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{self, MzmlWriter};
use crate::readers::{Reader, ScanEntry, Skips};
use crate::readers_centroids::Centroidato;
use crate::readers_mzml;
use crate::scantable::{Rows, Table, Where};

/// Scan `k` of sample `c` as the mzML written by `readers_mzml::scrivi` reads back: the arrays cut to the shorter of m/z and intensity; a scan whose
/// data do not decode has no points, as the mzML writes it (`readers::Skips`).
pub(crate) fn spettro(l: &mut dyn Reader, c: usize, k: usize) -> Result<Spectrum> {
    let s = l.scan(c, k);
    cut(Skips::default().take(l, (c, k), s)?)
}

/// The arrays of `s` cut to the shorter of m/z and intensity.
pub(crate) fn cut(mut s: Spectrum) -> Result<Spectrum> {
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

/// The table and pass 1 of the `.mzidx` builder in one decoding of the scans: what `righe` gives and the groups `mzidx::build_after` needs.
pub(crate) fn rows_before<'a>(
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
    let mut rows = empty_rows(l, c)?;
    let n = l.index(c)?.len();
    let mut skips = Skips::default();
    for k in 0..n {
        let s = l.scan(c, k);
        let s = cut(skips.take(l, (c, k), s)?)?;
        row(&mut rows, l, k, &s, before.as_deref_mut())?;
    }
    skips.end(n)?;
    Ok(rows.finish(chromatograms_written(l, c)?))
}

/// The rows of a table with no scan yet, from the header of the reader.
fn empty_rows(l: &mut dyn Reader, c: usize) -> Result<Rows> {
    let h = readers_mzml::table_header(l, c)?;
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
            p.guess(rows.last_hr());
        }
        p.add(level, pol, s)?;
    }
    Ok(())
}

fn chromatograms_written(l: &mut dyn Reader, c: usize) -> Result<usize> {
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
pub(crate) fn rows_before_double(
    l: &mut Centroidato,
    c: usize,
    before: &mut First<'_>,
    first_cen: &mut First<'_>,
) -> Result<(Table, Table)> {
    let mut rows = empty_rows(l.simple(), c)?;
    let mut rows_cen = empty_rows(l, c)?;
    let n = l.index(c)?.len();
    let mut skips = Skips::default();
    for k in 0..n {
        let s = l.grezza(c, k);
        let s = cut(skips.take(l.simple(), (c, k), s)?)?;
        row(&mut rows, l.simple(), k, &s, Some(&mut *before))?;
        drop(s);
        let s = spettro(l, c, k)?;
        row(&mut rows_cen, l, k, &s, Some(&mut *first_cen))?;
    }
    skips.end(n)?;
    let ncr = chromatograms_written(l.simple(), c)?;
    Ok((rows.finish(ncr), rows_cen.finish(ncr)))
}

/// The rows of version 2: `[membro, offset, lunghezza]` (or `null` when the reader gives no position) and `k` in place of `start, end`.
pub fn rows_json(t: &mut Table, entries: &[ScanEntry]) -> String {
    for (k, r) in t.rows.iter_mut().enumerate() {
        r.at = Some(Box::new(Where {
            k,
            position: entries.get(k).and_then(|v| v.position),
        }));
    }
    t.rows_json()
}
