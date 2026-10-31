//! The header of an mzML for the scan table: a single read serves the instrument configurations and the `<run ` tag text (`read_in`).

use super::*;

/// The header of the file (instrument configurations, default one): one read of up to 4 MB, whose bytes also serve the head text and the
/// `<run ` tag text when they lie inside it; the source is read again only for what falls outside.
pub(super) fn read_head<R: ReadAt>(src: &mut Source<'_, R>) -> Result<Head> {
    let size = src.size;
    let first = src.read(0, size.min(4 << 20))?;
    let head_end = find(&first, b"<run ").map(|p| p as u64);
    let head_hi = head_end.unwrap_or(200000.min(size));
    let run_lo = head_end.unwrap_or(0);
    let run_hi = (run_lo + 2000).min(size);
    let fit = |a: u64, b: u64| b <= first.len() as u64 && a <= b;
    let head_own;
    let head_bytes: &[u8] = if fit(0, head_hi) {
        &first[..head_hi as usize]
    } else {
        head_own = src.read(0, head_hi)?;
        &head_own
    };
    let run_own;
    let run_text: &[u8] = if fit(run_lo, run_hi) {
        &first[run_lo as usize..run_hi as usize]
    } else {
        run_own = src.read(run_lo, run_hi)?;
        &run_own
    };
    Ok(Head::parse(
        &String::from_utf8_lossy(head_bytes),
        &String::from_utf8_lossy(run_text),
    ))
}

/// `count` of the `<spectrumList` in `run_text` (0 when the tag or the attribute is not there).
pub(super) fn declared_count(run_text: &str) -> usize {
    let Some(p) = run_text.find("<spectrumList") else {
        return 0;
    };
    let t = &run_text[p..];
    attr(&t[..t.find('>').unwrap_or(t.len())], "count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(0)
}
