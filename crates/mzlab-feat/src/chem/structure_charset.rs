//! Text of a structure file: the bytes decoded as the byte-order mark or the XML declaration says (UTF-16 with or without a mark, UTF-8, windows-1252),
//! the characters that had to be replaced counted, and a UTF-16 declaration rewritten as UTF-8. Part of `structure_file.rs` (`open/1`).

use super::{blanks, name_end, NAME_CHARS};

/// The text of the bytes, its charset (`""` for plain UTF-8) and how many characters had to be replaced by U+FFFD because the bytes were not valid
/// in that charset. Read as the byte-order mark or the XML declaration says; UTF-16 without a mark is told by its NUL bytes.
pub(super) fn decode(bytes: &[u8]) -> (String, String, usize) {
    let bom = if bytes.starts_with(&[0xff, 0xfe]) {
        Some(true)
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        Some(false)
    } else {
        None
    };
    if let Some(le) = bom {
        let (text, bad) = utf16(&bytes[2..], le);
        return (text, "UTF-16".to_string(), bad);
    }
    if let Some(le) = nul_utf16(bytes) {
        let (text, bad) = utf16(bytes, le);
        return (text, format!("UTF-16{}", if le { "LE" } else { "BE" }), bad);
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return (s.to_string(), String::new(), 0);
    }
    let declared = declared_charset(bytes);
    let named = declared.as_deref().unwrap_or("windows-1252");
    match named.to_ascii_lowercase().as_str() {
        "utf-8" | "utf8" => {
            let bad = bytes
                .utf8_chunks()
                .filter(|c| !c.invalid().is_empty())
                .count();
            (
                String::from_utf8_lossy(bytes).into_owned(),
                named.to_string(),
                bad,
            )
        }
        "iso-8859-1" | "iso8859-1" | "iso_8859-1" | "latin1" | "l1" | "us-ascii" | "ascii"
        | "windows-1252" | "cp1252" | "x-cp1252" => (windows_1252(bytes), named.to_string(), 0),
        // a charset this reader has no table for: windows-1252, named as such in the notice
        _ => (windows_1252(bytes), "windows-1252".to_string(), 0),
    }
}

/// UTF-16 units (little or big endian) as text, and the number of characters replaced (an unpaired surrogate, an odd byte at the end).
fn utf16(bytes: &[u8], le: bool) -> (String, usize) {
    let units = bytes.chunks_exact(2).map(|c| {
        if le {
            u16::from_le_bytes([c[0], c[1]])
        } else {
            u16::from_be_bytes([c[0], c[1]])
        }
    });
    let mut bad = usize::from(bytes.len() % 2 == 1);
    let mut text: String = char::decode_utf16(units)
        .map(|r| {
            r.unwrap_or_else(|_| {
                bad += 1;
                '\u{fffd}'
            })
        })
        .collect();
    if bytes.len() % 2 == 1 {
        text.push('\u{fffd}'); // an odd byte at the end is no character
    }
    (text, bad)
}

/// `Some(little endian)` for text with no byte-order mark that is UTF-16 of mostly Latin text: in the first 512 bytes at least 9 in 10 of the bytes
/// in one place of each pair (the high byte) are NUL and at most 1 in 10 of the others. A head that does not meet this stays undetected.
fn nul_utf16(bytes: &[u8]) -> Option<bool> {
    let head = &bytes[..bytes.len().min(512) & !1];
    if head.len() < 4 {
        return None;
    }
    let n = head.len() / 2;
    let nuls = |par: usize| {
        head.iter()
            .skip(par)
            .step_by(2)
            .filter(|b| **b == 0)
            .count()
    };
    let (even, odd) = (nuls(0), nuls(1));
    if odd * 10 >= n * 9 && even * 10 <= n {
        Some(true)
    } else if even * 10 >= n * 9 && odd * 10 <= n {
        Some(false)
    } else {
        None
    }
}

/// The value of the `encoding` of an XML declaration that `text` starts with (after white space), as a range of `text`.
fn encoding_value(text: &str) -> Option<std::ops::Range<usize>> {
    let lead = text.len() - text.trim_start().len();
    let b = text.as_bytes();
    if !text[lead..].starts_with("<?xml") {
        return None;
    }
    let mut p = lead + 5;
    loop {
        let start = blanks(b, p);
        let end = name_end(b, start);
        let eq = blanks(b, end);
        if end == start || b.get(eq) != Some(&b'=') {
            return None;
        }
        let open = blanks(b, eq + 1);
        let q = *b.get(open).filter(|c| **c == b'"' || **c == b'\'')?;
        let close = open + 1 + text[open + 1..].find(q as char)?;
        if &text[start..end] == "encoding" {
            return Some(open + 1..close);
        }
        p = close + 1;
    }
}

/// The `encoding` of the XML declaration at the start of the bytes (first 200 read as one byte per character), as written.
fn declared_charset(bytes: &[u8]) -> Option<String> {
    let head: String = bytes[..bytes.len().min(200)]
        .iter()
        .map(|&b| char::from(b))
        .collect();
    let r = encoding_value(&head)?;
    let v = &head[r];
    (!v.is_empty()
        && v.bytes()
            .all(|c| c.is_ascii_alphanumeric() || NAME_CHARS.contains(&c)))
    .then(|| v.to_string())
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

/// The XML declaration at the very start of the text that names UTF-16 for text that is UTF-8 now names UTF-8 (one anywhere else is text).
pub(super) fn declare_utf8(text: &str) -> String {
    match encoding_value(text) {
        Some(r) => format!("{}UTF-8{}", &text[..r.start], &text[r.end..]),
        None => text.to_string(),
    }
}
