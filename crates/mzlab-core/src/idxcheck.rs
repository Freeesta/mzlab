//! Checks the sections of an index against the size of its file when it is opened: every array must lie inside the file, with lengths that do
//! not overflow, and every key must name a section. A damaged index is `error.badIndex` here, never a panic or a huge allocation later.

use crate::error::{Error, Result};
use crate::idxread::Section;

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
