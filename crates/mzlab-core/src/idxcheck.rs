//! Checks the sections of an index against the size of its file when it is opened: every array must lie inside the file, with lengths that do
//! not overflow, and every key must name a section. A damaged index is `error.badIndex` here, never a panic or a huge allocation later.
//! The optional `crc32` of a section (a CRC-32 per array) is checked on every array read whole (docs/agenti/rust-mzidx.md).

use crate::error::{Error, Result};
use crate::idxread::{Section, NAMES};
use crate::json::Value;

/// The `(offset index, bytes per element, element count)` of each of the 10 arrays of a section (`NAMES` of `idxread`).
fn extents(s: &Section) -> [(usize, u64, u64); 10] {
    let w = if s.f4 { 4 } else { 8 };
    let (ns, np, nf) = (s.n_scans as u64, s.n_peaks, s.n_fence as u64);
    [
        (0, 8, ns),
        (1, 8, ns),
        (2, 8, ns),
        (3, 8, ns),
        (4, 8, ns),
        (5, 8, ns),
        (6, w, nf),
        (7, w, np),
        (8, w, np),
        (9, 4, np),
    ]
}

/// Errors with `error.badIndex` unless every array of every section ends inside `size` and every key points at a section.
pub(crate) fn check(sections: &[Section], keys: &[(String, usize)], size: u64) -> Result<()> {
    let bad = || Error::new("error.badIndex");
    if keys.iter().any(|(_, i)| *i >= sections.len()) {
        return Err(bad());
    }
    for s in sections {
        for (k, width, count) in extents(s) {
            let end = count
                .checked_mul(width)
                .and_then(|len| s.off[k].checked_add(len))
                .ok_or_else(bad)?;
            if end > size {
                return Err(bad());
            }
        }
    }
    Ok(())
}

/// Errors with `error.badIndex` when a scan id falls outside the scan table of `rows` rows (an index opened lean has none and is not checked): the routes index the table with them.
pub(crate) fn scan_ids(ids: &[i64], rows: usize) -> Result<()> {
    if rows > 0
        && ids
            .iter()
            .any(|&i| usize::try_from(i).map_or(true, |i| i >= rows))
    {
        return Err(Error::new("error.badIndex"));
    }
    Ok(())
}

/// The `crc32` object of a section: `None` when absent (an index written before the sums), `error.badIndex` when present but malformed.
pub(crate) fn crcs(s: &Value) -> Result<Option<[u32; 10]>> {
    let Some(c) = s.get("crc32") else {
        return Ok(None);
    };
    let mut o = [0u32; 10];
    for (i, name) in NAMES.iter().enumerate() {
        let v = c
            .get(name)
            .and_then(Value::f64)
            .filter(|v| v.fract() == 0.0 && (0.0..=u32::MAX as f64).contains(v));
        o[i] = v.ok_or_else(|| Error::new("error.badIndex"))? as u32;
    }
    Ok(Some(o))
}

/// Errors with `error.badIndex` when array `k` of `s`, read whole as `bytes`, does not match the sum the section records.
pub(crate) fn sum(s: &Section, k: usize, bytes: &[u8]) -> Result<()> {
    let Some(c) = s.crc else { return Ok(()) };
    let mut h = flate2::Crc::new();
    h.update(bytes);
    if h.sum() != c[k] {
        return Err(Error::new("error.badIndex"));
    }
    Ok(())
}
