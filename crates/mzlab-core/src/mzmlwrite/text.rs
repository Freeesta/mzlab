//! The texts of the writer: escaping (the five XML characters, and the characters outside the XML 1.0 `Char` production replaced, so that no string of
//! a vendor file makes the output malformed) and the generic PSI-MS terms that a reader gives with a text of the file, written with their CV name
//! and the text in a `userParam`.

use super::{cv, Header};

/// `s` with `&`, `<`, `>` and `"` escaped and every character outside the XML 1.0 `Char` production (control characters, U+FFFE, U+FFFF) replaced by
/// U+FFFD, in one pass (a text with none of them is copied once): no string read from a vendor file can make the output malformed.
pub(crate) fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    esc_into(&mut out, s);
    out
}

/// `c` is in the XML 1.0 `Char` production (a Rust `char` is never a surrogate).
pub(crate) fn xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | ' '..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..)
}

/// `esc` appended to `out`.
pub(crate) fn esc_into(out: &mut String, s: &str) {
    if !s
        .bytes()
        .any(|b| matches!(b, b'&' | b'<' | b'>' | b'"' | 0..=0x1F | 0xEF))
    {
        out.push_str(s);
        return;
    }
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c if !xml_char(c) => out.push('\u{FFFD}'),
            c => out.push(c),
        }
    }
}

/// The PSI-MS parent terms a reader gives with a text of the file when the exact term is unknown (a model, a program, a kind of source, analyzer
/// or detector), with their CV names.
const GENERIC: [(&str, &str); 12] = [
    ("MS:1000031", "instrument model"),
    ("MS:1000483", "Thermo Fisher Scientific instrument model"),
    ("MS:1000490", "Agilent instrument model"),
    ("MS:1000121", "SCIEX instrument model"),
    ("MS:1000122", "Bruker Daltonics instrument model"),
    ("MS:1000126", "Waters instrument model"),
    ("MS:1000124", "Shimadzu instrument model"),
    ("MS:1000495", "Applied Biosystems instrument model"),
    ("MS:1000531", "software"),
    ("MS:1000008", "ionization type"),
    ("MS:1000443", "mass analyzer type"),
    ("MS:1000026", "detector type"),
];

/// The CV name of `acc` when it is a generic term (`GENERIC`).
pub fn generic_term(acc: &str) -> Option<&'static str> {
    GENERIC.iter().find(|(a, _)| *a == acc).map(|(_, n)| *n)
}

/// `name` itself when it is the CV name of a generic term.
pub fn generic_name(name: &str) -> Option<&'static str> {
    GENERIC.iter().find(|(_, n)| *n == name).map(|(_, n)| *n)
}

/// The name of the `userParam` that carries the text of a generic term: `instrument model` for every maker's model term, else the CV name.
pub fn generic_user_name(cv_name: &str) -> &str {
    if cv_name.ends_with("instrument model") {
        "instrument model"
    } else {
        cv_name
    }
}

/// A term of the header from a reader: a generic term with a text of the file keeps its CV name and the text (without a leading `NAME: `) goes in a
/// `userParam` (`generic_user_name`), which `mzmlhead`, `scantable` and the readers module read back; any other term is written as given.
pub(super) fn header_term(acc: &str, name: &str) -> String {
    match generic_term(acc) {
        Some(cv_name) if name != cv_name => {
            let text = name
                .strip_prefix(cv_name)
                .and_then(|t| t.strip_prefix(": "))
                .unwrap_or(name);
            format!(
                "{}<userParam name=\"{}\" value=\"{}\"/>",
                cv(acc, cv_name, ""),
                generic_user_name(cv_name),
                esc(text)
            )
        }
        _ => cv(acc, name, ""),
    }
}

/// The model of the instrument (`header_term`).
pub(super) fn instrument_terms(h: &Header) -> String {
    h.instrument
        .as_ref()
        .map_or(String::new(), |(acc, name)| header_term(acc, name))
}
