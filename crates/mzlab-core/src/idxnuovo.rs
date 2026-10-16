//! Header of an index written by a newer program (`error.newerFormat`, docs/agenti/formato-nuovo.md).

/// Highest header version this reader knows (1 = mzML, 2 = vendor file).
pub const VERSION_MAX: u32 = 2;

/// The version of an index written by a newer program (a newer magic `MZIDX00N`, or a known magic with a header version above `VERSION_MAX`); `None` when the header is one this reader knows or is no index.
pub fn newer_version(h: &[u8]) -> Option<u32> {
    let magic = h.get(..8)?;
    if &magic[..5] != b"MZIDX" || !magic[5..].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let n: u32 = std::str::from_utf8(&magic[5..]).ok()?.parse().ok()?;
    if n > 1 {
        return Some(n);
    }
    let v = u32::from_le_bytes(h.get(8..12)?.try_into().ok()?);
    (v > VERSION_MAX).then_some(v)
}
