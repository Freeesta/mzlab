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

/// `name="..."` as the LAST `name="` of a tag head (the regex `<cvParam [^>]*name="([^"]*)"` is greedy), for every `<cvParam ` of `block`.
pub(super) fn names(block: &str) -> Vec<String> {
    let mut out = vec![];
    let mut rest = block;
    while let Some(p) = rest.find("<cvParam ") {
        let tag = &rest[p + 9..];
        let end = tag.find('>').unwrap_or(tag.len());
        let head = &tag[..end];
        match head.rfind("name=\"") {
            Some(q) => {
                let v = &head[q + 6..];
                match v.find('"') {
                    Some(e) => {
                        out.push(v[..e].to_string());
                        rest = &tag[q + 6 + e + 1..];
                    }
                    None => rest = &tag[end..],
                }
            }
            None => rest = &tag[end..],
        }
    }
    out
}

/// The text of every `<kind ...>...</kind>` block in `s` (the regex `<kind .*?</kind>` with DOTALL).
pub(super) fn blocks<'a>(s: &'a str, kind: &str) -> Vec<&'a str> {
    let (open, close) = (format!("<{kind} "), format!("</{kind}>"));
    let mut out = vec![];
    let mut rest = s;
    while let Some(a) = rest.find(&open) {
        match rest[a..].find(&close) {
            Some(b) => {
                out.push(&rest[a..a + b + close.len()]);
                rest = &rest[a + b + close.len()..];
            }
            None => break,
        }
    }
    out
}
