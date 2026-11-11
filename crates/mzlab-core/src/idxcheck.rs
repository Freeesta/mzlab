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

/// The sum recorded for an array of a section that has no `crc32` (an index written before the sums).
pub(crate) const ABSENT: u64 = u64::MAX;

/// Appends the 10 sums of the `crc32` object of section `s` to `out` (`ABSENT` for each when the section has none); `error.badIndex` when the
/// object is present but malformed. Flat `u64`s, not an `Option` per section: the feature modules already carry the code of a `Vec<u64>`.
pub(crate) fn crcs(s: &Value, out: &mut Vec<u64>) -> Result<()> {
    let c = s.get("crc32");
    for name in NAMES {
        out.push(match c {
            None => ABSENT,
            Some(c) => c
                .get(name)
                .and_then(Value::f64)
                .filter(|v| v.fract() == 0.0 && (0.0..=u32::MAX as f64).contains(v))
                .ok_or_else(|| Error::new("error.badIndex"))? as u64,
        });
    }
    Ok(())
}

/// Errors with `error.badIndex` when array `k` of section `si`, read whole as `bytes`, does not match the sum the section records (`crcs` of `Index`).
pub(crate) fn sum(crcs: &[u64], si: usize, k: usize, bytes: &[u8]) -> Result<()> {
    match crcs.get(si * 10 + k) {
        Some(&want) if want != ABSENT && u64::from(crc32(bytes)) != want => {
            Err(Error::new("error.badIndex"))
        }
        _ => Ok(()),
    }
}

/// IEEE CRC-32, bit by bit: no table and no flate2, so the feature modules that link this reader keep their size caps and their memory layout
/// (the arrays checked are the per-scan ones and the fence, small beside the peaks).
fn crc32(bytes: &[u8]) -> u32 {
    !bytes.iter().fold(!0u32, |c, &b| {
        (0..8).fold(c ^ u32::from(b), |c, _| {
            (c >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(c & 1))
        })
    })
}
