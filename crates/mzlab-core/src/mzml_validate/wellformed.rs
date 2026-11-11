//! The strict XML 1.0 pass the validator applies to every event before any mzML check: characters outside the `Char` production (in text, names,
//! attributes, comments, references), malformed or repeated attributes, `<` in an attribute value, unknown entities, and content outside the one root.
//! quick-xml accepts all of these; a conforming XML parser stops at the first, so the validator does too.

use quick_xml::events::{BytesStart, Event};

use crate::mzmlwrite::xml_char;

/// Whether the walk has seen the root element.
#[derive(Default)]
pub(super) struct Doc {
    root_open: bool,
}

/// `None` when the event is well-formed at `depth` (the elements open before it), else the reason.
pub(super) fn check(ev: &Event, depth: usize, doc: &mut Doc) -> Option<String> {
    // the arrays are base64: printable ASCII needs no decoding
    let plain = ev
        .bytes()
        .all(|b| matches!(b, 0x20..=0x7E | b'\t' | b'\n' | b'\r'));
    if let Some(c) = (!plain)
        .then(|| ev.chars().find(|&c| !xml_char(c)))
        .flatten()
    {
        return Some(format!(
            "character U+{:04X} is not allowed in XML 1.0",
            c as u32
        ));
    }
    match ev {
        Event::Start(e) | Event::Empty(e) => {
            if depth == 0 {
                if doc.root_open {
                    return Some("a second root element".into());
                }
                doc.root_open = true;
            }
            attributes(e)
        }
        Event::Text(t) if depth == 0 && !t.trim_matches(['\t', '\n', '\r', ' ']).is_empty() => {
            Some("text outside the root element".into())
        }
        Event::CData(_) | Event::GeneralRef(_) if depth == 0 => {
            Some("content outside the root element".into())
        }
        Event::Decl(_) | Event::DocType(_) if doc.root_open => {
            Some("a declaration after the root element".into())
        }
        Event::GeneralRef(r) => reference(r),
        _ => None,
    }
}

/// Every attribute parses (quoted, not repeated), holds no `<` and only known references to allowed characters.
fn attributes(e: &BytesStart) -> Option<String> {
    for a in e.attributes().with_checks(true) {
        let a = match a {
            Ok(a) => a,
            Err(x) => return Some(format!("malformed attribute: {x}")),
        };
        if a.value.contains('<') {
            return Some(format!(
                "'<' in the value of attribute {}",
                a.key.into_inner()
            ));
        }
        let mut rest: &str = &a.value;
        while let Some(p) = rest.find('&') {
            let tail = &rest[p + 1..];
            let Some(end) = tail.find(';') else {
                return Some(format!(
                    "'&' without ';' in attribute {}",
                    a.key.into_inner()
                ));
            };
            if let Some(why) = entity(&tail[..end]) {
                return Some(why);
            }
            rest = &tail[end + 1..];
        }
    }
    None
}

fn reference(r: &quick_xml::events::BytesRef) -> Option<String> {
    entity(r)
}

/// `None` for one of the five predefined entities or a character reference to an allowed character.
fn entity(name: &str) -> Option<String> {
    if matches!(name, "lt" | "gt" | "amp" | "quot" | "apos") {
        return None;
    }
    let code = if let Some(h) = name.strip_prefix("#x") {
        u32::from_str_radix(h, 16).ok()
    } else if let Some(d) = name.strip_prefix('#') {
        d.parse::<u32>().ok()
    } else {
        return Some(format!("unknown entity &{name};"));
    };
    match code.and_then(char::from_u32) {
        Some(c) if xml_char(c) => None,
        _ => Some(format!(
            "character reference &{name}; is not an XML 1.0 character"
        )),
    }
}
