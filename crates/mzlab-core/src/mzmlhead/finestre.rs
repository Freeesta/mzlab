//! `Mzml::chromatograms` read in windows: the chromatograms of an MRM file are the whole file, so the list is searched window by window
//! instead of in one buffer as large as the file (which also capped such files at `MAX_REGION`).

use super::tags::find_tag;
use super::{chrom_of, find, part, text, Chrom, Mzml};
use crate::error::Result;
use crate::idxread::ReadAt;

/// Bytes read at a time; a window grows (doubling) only while one chromatogram does not fit in it.
pub(super) const WINDOW: u64 = 4 << 20;

/// The header of every `<chromatogram>` from `scans_end` on, as the search over `scans_end..size` in one buffer finds them.
pub(super) fn chromatograms<R: ReadAt>(m: &mut Mzml<R>, scans_end: u64) -> Result<Vec<Chrom>> {
    let mut out: Vec<Chrom> = vec![];
    let (mut at, mut len) = (scans_end, WINDOW);
    while at < m.size {
        let end = at.saturating_add(len).min(m.size);
        let region = m.read(at, end)?;
        let mut pos = 0usize;
        let mut cut = None;
        while let Some(a) = find_tag(&region, b"<chromatogram", pos) {
            let Some(b) = find(&region, b"</chromatogram>", a) else {
                cut = Some(a);
                break;
            };
            let c = find(part(&region, 0, b), b"<binaryDataArrayList", a);
            let h = text(part(&region, a, c.unwrap_or(b)));
            out.push(chrom_of(&h, out.len(), at + a as u64, at + (b + 15) as u64));
            pos = b + 1;
        }
        if end >= m.size {
            break;
        }
        match cut {
            // one chromatogram longer than the window: the same start, a window twice as long
            Some(0) => len = len.saturating_mul(2),
            Some(a) => (at, len) = (at + a as u64, WINDOW),
            // far enough back that an opening tag cut by the end of this window is seen whole in the next
            None => (at, len) = ((at + pos as u64).max(end - 13), WINDOW),
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "finestre_tests.rs"]
mod tests;
