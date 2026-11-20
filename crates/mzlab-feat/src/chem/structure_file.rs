//! `open/1`: a structure file as the text the Draw editor (Ketcher, with Indigo inside) reads, and what was changed on the way. No format is
//! parsed. These changes to the text are made, each reported in `changes`: UTF-16 and 8-bit text is decoded as its byte-order mark or XML
//! declaration says, and the attributes with an empty value are dropped from the start tags of a CDXML text (RDKit writes `BondLength=""` on
//! the root and Indigo then draws nothing, without an error). Binary CDX (starts with `VjCD`) is passed on as base64, unchanged. Same bytes in
//! the molkit `.wasm`, the app (`feat/grafo`, `kind: "open"`) and `mzlab-cli mol open`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out};
use std::collections::BTreeSet;

#[path = "structure_charset.rs"]
mod charset;
use charset::{declare_utf8, decode};

const CDX_MAGIC: &[u8] = b"VjCD";

/// One change made to the text of a file.
#[derive(Debug, PartialEq)]
pub enum Change {
    /// The text was decoded as this charset (not plain UTF-8).
    Charset(String),
    /// This many characters were not valid in the charset of the file and are replaced by U+FFFD.
    Replaced(usize),
    /// These attribute names, each once in order of first sight, were dropped for having an empty value.
    Empty(Vec<String>),
}

/// The text the editor reads and what was changed to get it.
#[derive(Debug, PartialEq)]
pub struct Opened {
    pub text: String,
    pub changes: Vec<Change>,
}

/// Drops the attributes with an empty value from every start tag of a CDXML text, one pass and linear in the length: the root element must be
/// `CDXML` (any other text comes back unchanged); comments, CDATA sections, processing instructions and the DOCTYPE with its internal subset are
/// copied as they are; a quoted value may hold `>`, `<` or the other quote and is never edited. Text that never closes a comment, CDATA section,
/// declaration or quote is malformed: the rest is copied unchanged. The names are each once, in order of first sight.
pub fn tidy_cdxml(text: &str) -> (String, Vec<String>) {
    let (mut out, mut removed, mut i, mut root) =
        (String::with_capacity(text.len()), Vec::new(), 0, false);
    let mut seen = BTreeSet::new(); // the names in `removed`, for a lookup that stays fast with many distinct names
    while i < text.len() {
        let rest = &text[i..];
        let k = rest.find('<').unwrap_or(rest.len());
        out.push_str(&rest[..k]);
        i += k;
        let rest = &text[i..];
        if rest.is_empty() {
            break;
        }
        match markup(rest) {
            Markup::Skip(n) => {
                out.push_str(&rest[..n]);
                i += n;
            }
            Markup::Tag(n) => {
                let tag = &rest[..n];
                if !root {
                    root = true;
                    if !is_root(tag) {
                        out.push_str(rest);
                        return (out, removed);
                    }
                }
                out.push_str(&strip_empty(tag, &mut removed, &mut seen));
                i += n;
            }
            Markup::Text => {
                out.push('<'); // not markup: the text goes on after the bracket
                i += 1;
            }
            Markup::Unclosed => {
                out.push_str(rest);
                break;
            }
        }
    }
    (out, removed)
}

/// What a `<` begins.
enum Markup {
    /// A comment, CDATA section, processing instruction or declaration, of this length: copied.
    Skip(usize),
    /// A start tag of this length.
    Tag(usize),
    /// Just a bracket in the text.
    Text,
    /// A construct that never closes: the rest of the text is copied as it is.
    Unclosed,
}

fn skip_to(s: &str, from: usize, close: &str) -> Markup {
    s[from..]
        .find(close)
        .map_or(Markup::Unclosed, |k| Markup::Skip(from + k + close.len()))
}

fn markup(s: &str) -> Markup {
    if s.starts_with("<!--") {
        return skip_to(s, 4, "-->");
    }
    if s.starts_with("<![CDATA[") {
        return skip_to(s, 9, "]]>");
    }
    if s.starts_with("<?") {
        return skip_to(s, 2, "?>");
    }
    if s.starts_with("<!") {
        return declaration(s);
    }
    tag_len(s)
}

/// A `<!...>` declaration (the DOCTYPE with an internal subset in brackets, quoted literals and comments inside it).
fn declaration(s: &str) -> Markup {
    let b = s.as_bytes();
    let (mut i, mut depth) = (2, 0usize);
    while i < b.len() {
        match b[i] {
            q @ (b'"' | b'\'') => match s[i + 1..].find(q as char) {
                Some(k) => i += k + 2,
                None => return Markup::Unclosed,
            },
            b'[' => {
                depth += 1;
                i += 1;
            }
            b']' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            b'<' if s[i..].starts_with("<!--") => match s[i + 4..].find("-->") {
                Some(k) => i += k + 7,
                None => return Markup::Unclosed,
            },
            b'>' if depth == 0 => return Markup::Skip(i + 1),
            _ => i += 1,
        }
    }
    Markup::Unclosed
}

/// The start tag that `s` begins with (`<` and a letter, then quoted values and any other character but `<` and `>`).
fn tag_len(s: &str) -> Markup {
    let b = s.as_bytes();
    if !b.get(1).is_some_and(u8::is_ascii_alphabetic) {
        return Markup::Text;
    }
    let mut i = 2;
    while let Some(&c) = b.get(i) {
        match c {
            q @ (b'"' | b'\'') => match s[i + 1..].find(q as char) {
                Some(k) => i += k + 2,
                None => return Markup::Unclosed, // no closing quote anywhere after: nothing later is well formed
            },
            b'<' => return Markup::Text,
            b'>' => return Markup::Tag(i + 1),
            _ => i += 1,
        }
    }
    Markup::Text
}

pub(super) const NAME_CHARS: &[u8] = b"_.:-";

pub(super) fn name_end(b: &[u8], mut i: usize) -> usize {
    while b
        .get(i)
        .is_some_and(|c| c.is_ascii_alphanumeric() || NAME_CHARS.contains(c))
    {
        i += 1;
    }
    i
}

pub(super) fn blanks(b: &[u8], mut i: usize) -> usize {
    while b.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
}

/// Whether the start tag is the `CDXML` root.
fn is_root(tag: &str) -> bool {
    let b = tag.as_bytes();
    let e = name_end(b, 1);
    &tag[1..e] == "CDXML"
}

/// The tag without its attributes of empty value (`name=""` or `name=''`, with the white space before it). The tag is read left to right in
/// tokens, a quoted string is skipped whole wherever it stands, so a quoted value is never read as attributes.
fn strip_empty(tag: &str, removed: &mut Vec<String>, seen: &mut BTreeSet<String>) -> String {
    let b = tag.as_bytes();
    let (mut out, mut copied, mut p) = (String::new(), 0, name_end(b, 1));
    while p < b.len() {
        let start = blanks(b, p);
        match b.get(start) {
            None | Some(b'>') => break,
            Some(&q @ (b'"' | b'\'')) => {
                // a quoted string that is no attribute's value
                p = tag[start + 1..]
                    .find(q as char)
                    .map_or(b.len(), |k| start + k + 2);
                continue;
            }
            _ => {}
        }
        let end = name_end(b, start);
        if end == start {
            p = start + 1; // any other character
            continue;
        }
        let eq = blanks(b, end);
        if b.get(eq) != Some(&b'=') || start == p {
            p = eq.max(end);
            continue;
        }
        let open = blanks(b, eq + 1);
        let Some(&q) = b.get(open).filter(|c| **c == b'"' || **c == b'\'') else {
            p = open;
            continue;
        };
        let close = open + 1 + tag[open + 1..].find(q as char).unwrap_or(b.len());
        if close == open + 1 {
            out.push_str(&tag[copied..p]);
            // the next token must not fuse with the name before the dropped attribute (`a=""x=""`)
            if b.get(close + 1)
                .is_some_and(|c| !c.is_ascii_whitespace() && *c != b'>' && *c != b'/')
            {
                out.push(' ');
            }
            copied = close + 1;
            let name = &tag[start..end];
            if seen.insert(name.to_string()) {
                removed.push(name.to_string());
            }
        }
        p = close + 1;
    }
    out.push_str(&tag[copied..]);
    out
}

/// The largest structure file read (8 MiB): the engine runs on the page thread of the site, and no drawing file comes near it.
pub const MAX_BYTES: usize = 8 << 20;

/// Refuses a file of this size (`error.open.tooBig`, the real size and the limit in the error): the one test the engine, the CLI (from the file
/// metadata) and the site (from `file.size`, before the file is read) all run.
pub fn check_size(size: u64) -> Result<()> {
    if size > MAX_BYTES as u64 {
        return Err(Error::new("error.open.tooBig")
            .with("size", size)
            .with("limit", MAX_BYTES));
    }
    Ok(())
}

/// `{"limit": L}` when a file of this size is read, else the `error.open.tooBig` of `check_size`.
pub fn size_json(size: u64) -> Result<String> {
    check_size(size)?;
    Ok(format!("{{\"limit\":{MAX_BYTES}}}"))
}

/// What the editor reads for the bytes of a structure file, and what was changed on the way; a file over `MAX_BYTES` is refused
/// (`error.open.tooBig`, the size and the limit in the error), never cut.
pub fn open_bytes(bytes: &[u8]) -> Result<Opened> {
    check_size(bytes.len() as u64)?;
    if bytes.starts_with(CDX_MAGIC) {
        return Ok(Opened {
            text: base64(bytes),
            changes: Vec::new(),
        });
    }
    let (raw, charset, bad) = decode(bytes);
    let (text, removed) = tidy_cdxml(raw.strip_prefix('\u{feff}').unwrap_or(&raw));
    let mut changes = Vec::new();
    if !charset.is_empty() {
        changes.push(Change::Charset(charset.clone()));
    }
    if bad > 0 {
        changes.push(Change::Replaced(bad));
    }
    if !removed.is_empty() {
        changes.push(Change::Empty(removed));
    }
    let text = if charset.to_ascii_uppercase().starts_with("UTF-16") {
        declare_utf8(&text)
    } else {
        text
    };
    Ok(Opened { text, changes })
}

impl Opened {
    /// The `open/1` document.
    pub fn json(&self) -> String {
        let mut out = Out::default();
        out.raw("{\"schema\":\"open/1\",\"text\":")
            .str(&self.text)
            .raw(",\"changes\":[");
        for (i, c) in self.changes.iter().enumerate() {
            out.raw(if i > 0 { "," } else { "" });
            match c {
                Change::Charset(c) => out.raw("{\"fix\":\"charset\",\"charset\":").str(c),
                Change::Replaced(n) => out
                    .raw("{\"fix\":\"replaced\",\"count\":")
                    .raw(&n.to_string()),
                Change::Empty(names) => {
                    out.raw("{\"fix\":\"empty\",\"names\":[");
                    for (k, n) in names.iter().enumerate() {
                        out.raw(if k > 0 { "," } else { "" }).str(n);
                    }
                    out.raw("]")
                }
            };
            out.raw("}");
        }
        out.raw("]}");
        out.0
    }
}

/// The `open/1` text of the bytes of a file.
pub fn open_json_bytes(bytes: &[u8]) -> Result<String> {
    open_bytes(bytes).map(|o| o.json())
}

/// `open/1` of a request `{"data": "<the file, base64>"}` (the app has no binary request): the same text as `open_json_bytes`. A request
/// `{"size": N}` asks only whether a file of N bytes is read: `{"limit": L}` when it is, else the `error.open.tooBig` of `check_size`.
pub fn open_json(request: &str) -> Result<String> {
    let bad = || Error::new("error.open.request");
    let v = json::parse(request.as_bytes()).map_err(|_| bad())?;
    if let Some(n) = v.get("size").and_then(json::Value::f64) {
        return if n >= 0.0 {
            size_json(n as u64)
        } else {
            Err(bad())
        };
    }
    let bytes =
        unbase64(v.get("data").and_then(json::Value::str).ok_or_else(bad)?).ok_or_else(bad)?;
    open_json_bytes(&bytes)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding.
fn base64(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for k in 0..4 {
            s.push(if k <= c.len() {
                char::from(B64[(n >> (18 - 6 * k) & 63) as usize])
            } else {
                '='
            });
        }
    }
    s
}

/// Standard base64 (padding required, no white space), or `None`.
fn unbase64(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    for (i, c) in s.as_bytes().chunks(4).enumerate() {
        let last = (i + 1) * 4 == s.len();
        let pad = c.iter().rev().take_while(|b| **b == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return None;
        }
        let mut n = 0u32;
        for &b in &c[..4 - pad] {
            n = n << 6 | B64.iter().position(|x| *x == b)? as u32;
        }
        n <<= 6 * pad as u32;
        out.extend_from_slice(&n.to_be_bytes()[1..4 - pad]);
    }
    Some(out)
}

#[cfg(test)]
#[path = "structure_file_tests.rs"]
mod tests;
