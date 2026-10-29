//! Searches of `mzmlhead` that must not depend on how the writer spaces its XML: tag names and the unit of a time array.

use super::{attr, find, text};

/// Start of the next element named `tag` (`b"<run"`): the name must be followed by XML white space or `>`, so `<runX` and `<binaryDataArrayList` do not count.
pub(super) fn find_tag(h: &[u8], tag: &[u8], from: usize) -> Option<usize> {
    let mut at = from;
    while let Some(p) = find(h, tag, at) {
        if h.get(p + tag.len())
            .is_some_and(|c| c.is_ascii_whitespace() || *c == b'>')
        {
            return Some(p);
        }
        at = p + 1;
    }
    None
}

/// Whether a cvParam of the chromatogram names the time array in seconds, whatever the order of its attributes and whether `value` is there.
pub(super) fn time_in_seconds(chunk: &[u8]) -> bool {
    text(chunk).split('<').any(|tag| {
        attr(tag, "name") == Some("time array") && attr(tag, "unitAccession") == Some("UO:0000010")
    })
}
