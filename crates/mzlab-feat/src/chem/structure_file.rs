//! `open/1`: a structure file as the text the Draw editor (Ketcher, with Indigo inside) reads, and what was changed on the way. No format is
//! parsed. Two changes to the text are made, each reported in `changes`: UTF-16 and 8-bit text is decoded as its byte-order mark or XML
//! declaration says, and the attributes with an empty value are dropped from the start tags of a CDXML text (RDKit writes `BondLength=""` on
//! the root and Indigo then draws nothing, without an error). Binary CDX (starts with `VjCD`) is passed on as base64, unchanged. Same bytes in
//! the molkit `.wasm`, the app (`feat/grafo`, `kind: "open"`) and `mzlab-cli mol open`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out};

const CDX_MAGIC: &[u8] = b"VjCD";

/// One change made to the text of a file.
#[derive(Debug, PartialEq)]
pub enum Change {
    /// The text was decoded as this charset (not plain UTF-8).
    Charset(String),
    /// These attribute names, each once in order of first sight, were dropped for having an empty value.
    Empty(Vec<String>),
}

/// The text the editor reads and what was changed to get it.
#[derive(Debug, PartialEq)]
pub struct Opened {
    pub text: String,
    pub changes: Vec<Change>,
}

/// Drops the attributes with an empty value from every start tag of a CDXML text (a quoted value may hold `>`, `<` or the other quote; comments
/// and CDATA sections are left as they are). Other text comes back unchanged. The names are each once, in order of first sight.
pub fn tidy_cdxml(text: &str) -> (String, Vec<String>) {
    let is_cdxml = text.match_indices("<CDXML").any(|(i, m)| {
        matches!(text.as_bytes().get(i + m.len()), Some(b) if b.is_ascii_whitespace() || *b == b'>')
    });
    if !is_cdxml {
        return (text.to_string(), Vec::new());
    }
    let (mut out, mut removed, mut i) = (String::with_capacity(text.len()), Vec::new(), 0);
    while i < text.len() {
        let rest = &text[i..];
        if !rest.starts_with('<') {
            let k = rest.find('<').unwrap_or(rest.len());
            out.push_str(&rest[..k]);
            i += k;
            continue;
        }
        match markup(rest) {
            Some((n, true)) => {
                out.push_str(&strip_empty(&rest[..n], &mut removed));
                i += n;
            }
            Some((n, false)) => {
                out.push_str(&rest[..n]);
                i += n;
            }
            None => {
                out.push('<'); // not markup: the text goes on after the bracket
                i += 1;
            }
        }
    }
    (out, removed)
}

/// The comment, CDATA section (`false`) or start tag (`true`) that `s` begins with, and its length.
fn markup(s: &str) -> Option<(usize, bool)> {
    for (open, close) in [("<!--", "-->"), ("<![CDATA[", "]]>")] {
        if let Some(rest) = s.strip_prefix(open) {
            return rest
                .find(close)
                .map(|k| (open.len() + k + close.len(), false));
        }
    }
    tag_len(s).map(|n| (n, true))
}

/// The length of the start tag that `s` begins with (`<` and a letter, then quoted values and any other character but `<` and `>`), or `None`.
fn tag_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    if !b.get(1)?.is_ascii_alphabetic() {
        return None;
    }
    let mut i = 2;
    loop {
        match *b.get(i)? {
            q @ (b'"' | b'\'') => i += s[i + 1..].find(q as char)? + 2,
            b'<' => return None,
            b'>' => return Some(i + 1),
            _ => i += 1,
        }
    }
}

/// The tag without its attributes of empty value (`name=""` or `name=''`, with the white space before it); a quoted value is never read as an attribute.
fn strip_empty(tag: &str, removed: &mut Vec<String>) -> String {
    let (b, mut out, mut p, mut copied) = (tag.as_bytes(), String::new(), 0, 0);
    while p < b.len() {
        let Some((end, name, empty)) = attribute(tag, p) else {
            p += 1;
            continue;
        };
        if empty {
            out.push_str(&tag[copied..p]);
            copied = end;
            if !removed.iter().any(|n| n == name) {
                removed.push(name.to_string());
            }
        }
        p = end;
    }
    out.push_str(&tag[copied..]);
    out
}

/// The attribute that starts with the white space at `p`: where it ends, its name, and whether its value is empty.
fn attribute(tag: &str, p: usize) -> Option<(usize, &str, bool)> {
    let b = tag.as_bytes();
    let blanks = |mut i: usize| {
        while b.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        i
    };
    let start = blanks(p);
    if start == p
        || !b
            .get(start)
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
    {
        return None;
    }
    let mut end = start;
    while b
        .get(end)
        .is_some_and(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(c))
    {
        end += 1;
    }
    let eq = blanks(end);
    if b.get(eq) != Some(&b'=') {
        return None;
    }
    let open = blanks(eq + 1);
    let q = *b.get(open).filter(|c| **c == b'"' || **c == b'\'')?;
    let close = open + 1 + tag[open + 1..].find(q as char)?;
    Some((close + 1, &tag[start..end], close == open + 1))
}

/// The text of the bytes and its charset (`""` for plain UTF-8), read as the byte-order mark or the XML declaration says.
fn decode(bytes: &[u8]) -> (String, String) {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let le = bytes[0] == 0xff;
        let units = bytes[2..].chunks_exact(2).map(|c| {
            if le {
                u16::from_le_bytes([c[0], c[1]])
            } else {
                u16::from_be_bytes([c[0], c[1]])
            }
        });
        let mut text: String = char::decode_utf16(units)
            .map(|r| r.unwrap_or('\u{fffd}'))
            .collect();
        if bytes.len() % 2 == 1 {
            text.push('\u{fffd}'); // an odd byte at the end is no character
        }
        return (text, "UTF-16".to_string());
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return (s.to_string(), String::new());
    }
    let declared = declared_charset(bytes);
    let named = declared.as_deref().unwrap_or("windows-1252");
    match named.to_ascii_lowercase().as_str() {
        "utf-8" | "utf8" => (
            String::from_utf8_lossy(bytes).into_owned(),
            named.to_string(),
        ),
        "iso-8859-1" | "iso8859-1" | "iso_8859-1" | "latin1" | "l1" | "us-ascii" | "ascii"
        | "windows-1252" | "cp1252" | "x-cp1252" => (windows_1252(bytes), named.to_string()),
        // a charset this reader has no table for: windows-1252, named as such in the notice
        _ => (windows_1252(bytes), "windows-1252".to_string()),
    }
}

/// The `encoding` of the XML declaration at the start of the bytes (first 200 read as one byte per character), as written.
fn declared_charset(bytes: &[u8]) -> Option<String> {
    let head: String = bytes[..bytes.len().min(200)]
        .iter()
        .map(|&b| char::from(b))
        .collect();
    let decl = head.trim_start().strip_prefix("<?xml")?;
    let decl = &decl[..decl.find('>').unwrap_or(decl.len())];
    let hits: Vec<usize> = decl.match_indices("encoding=").map(|(i, _)| i).collect();
    hits.into_iter().rev().find_map(|i| {
        let after = decl[i + "encoding=".len()..].strip_prefix(['"', '\''])?;
        let n = after.find(|c: char| !(c.is_ascii_alphanumeric() || "_.:-".contains(c)))?;
        (n > 0 && after[n..].starts_with(['"', '\''])).then(|| after[..n].to_string())
    })
}

/// windows-1252 (the web's reading of ISO-8859-1 and US-ASCII too): bytes 0x80-0x9F are the printable characters, never controls.
fn windows_1252(bytes: &[u8]) -> String {
    const HIGH: [char; 32] = [
        '\u{20ac}', '\u{81}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{2c6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8d}', '\u{17d}',
        '\u{8f}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}',
        '\u{2014}', '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}', '\u{153}', '\u{9d}', '\u{17e}',
        '\u{178}',
    ];
    bytes
        .iter()
        .map(|&b| {
            if (0x80..0xa0).contains(&b) {
                HIGH[usize::from(b - 0x80)]
            } else {
                char::from(b)
            }
        })
        .collect()
}

/// An XML declaration that names UTF-16 for text that is UTF-8 now names UTF-8.
fn declare_utf8(text: &str) -> String {
    let mut from = 0;
    while let Some(k) = text[from..].find("<?xml") {
        let at = from + k;
        let end = text[at..].find('>').map_or(text.len(), |e| at + e);
        if let Some(e) = text[at..end]
            .rfind("encoding=")
            .map(|e| at + e + "encoding=".len())
            && text[e..end].starts_with(['"', '\''])
        {
            let v = e + 1;
            let n = text[v..end].find(['"', '\'']).unwrap_or(end - v);
            return format!("{}UTF-8{}", &text[..v], &text[v + n..]);
        }
        from = at + 5;
    }
    text.to_string()
}

/// What the editor reads for the bytes of a structure file, and what was changed on the way.
pub fn open_bytes(bytes: &[u8]) -> Opened {
    if bytes.starts_with(CDX_MAGIC) {
        return Opened {
            text: base64(bytes),
            changes: Vec::new(),
        };
    }
    let (raw, charset) = decode(bytes);
    let (text, removed) = tidy_cdxml(raw.strip_prefix('\u{feff}').unwrap_or(&raw));
    let mut changes = Vec::new();
    if !charset.is_empty() {
        changes.push(Change::Charset(charset.clone()));
    }
    if !removed.is_empty() {
        changes.push(Change::Empty(removed));
    }
    let text = if charset.to_ascii_uppercase().starts_with("UTF-16") {
        declare_utf8(&text)
    } else {
        text
    };
    Opened { text, changes }
}

/// The `open/1` text of the bytes of a file.
pub fn open_json_bytes(bytes: &[u8]) -> String {
    let o = open_bytes(bytes);
    let mut out = Out::default();
    out.raw("{\"schema\":\"open/1\",\"text\":")
        .str(&o.text)
        .raw(",\"changes\":[");
    for (i, c) in o.changes.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" });
        match c {
            Change::Charset(c) => out.raw("{\"fix\":\"charset\",\"charset\":").str(c),
            Change::Empty(names) => out
                .raw("{\"fix\":\"empty\",\"names\":")
                .str(&names.join(", ")),
        };
        out.raw("}");
    }
    out.raw("]}");
    out.0
}

/// `open/1` of a request `{"data": "<the file, base64>"}` (the app has no binary request): the same text as `open_json_bytes`.
pub fn open_json(request: &str) -> Result<String> {
    let bad = || Error::new("error.open.request");
    let v = json::parse(request.as_bytes()).map_err(|_| bad())?;
    let bytes =
        unbase64(v.get("data").and_then(json::Value::str).ok_or_else(bad)?).ok_or_else(bad)?;
    Ok(open_json_bytes(&bytes))
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
