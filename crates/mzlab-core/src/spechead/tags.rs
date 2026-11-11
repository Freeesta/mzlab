//! A small XML tag scanner for the header text of an mzML (cut anywhere, so not a document): element names without their namespace prefix,
//! attributes in any order and with either quote, the five entities and character references, comments, processing instructions, CDATA.
//! Malformed or cut markup ends the scan; the strict check is the validator's (`mzml_validate`).

use std::borrow::Cow;

/// An element tag: `Open` `<a>`, `Empty` `<a/>`, `Close` `</a>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Open,
    Empty,
    Close,
}

/// One tag with its local name and attributes (local keys, unescaped values; `xmlns` declarations left out).
#[derive(Debug)]
pub struct Tag<'a> {
    pub name: &'a str,
    pub attrs: Vec<(&'a str, Cow<'a, str>)>,
    pub kind: Kind,
}

impl Tag<'_> {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_ref())
    }
}

/// A tag, or character data (text between tags, unescaped, or the content of a CDATA section).
#[derive(Debug)]
pub enum Item<'a> {
    Tag(Tag<'a>),
    Text(Cow<'a, str>),
}

/// The name after its namespace prefix (`m:spectrum` -> `spectrum`).
pub fn local(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, l)| l)
}

/// The items of `s` in order.
pub struct Items<'a> {
    s: &'a str,
    at: usize,
}

pub fn items(s: &str) -> Items<'_> {
    Items { s, at: 0 }
}

impl<'a> Iterator for Items<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Item<'a>> {
        loop {
            let s = self.s;
            let rest = s.get(self.at..)?;
            if rest.is_empty() {
                return None;
            }
            let Some(lt) = rest.find('<') else {
                self.at = s.len();
                return Some(Item::Text(unescape(rest, false)));
            };
            if lt > 0 {
                let t = &rest[..lt];
                self.at += lt;
                return Some(Item::Text(unescape(t, false)));
            }
            let (skip, end) = if rest.starts_with("<!--") {
                (0, "-->")
            } else if let Some(c) = rest.strip_prefix("<![CDATA[") {
                let e = c.find("]]>")?;
                self.at += 9 + e + 3;
                return Some(Item::Text(Cow::Borrowed(&c[..e])));
            } else if rest.starts_with("<?") {
                (0, "?>")
            } else if rest.starts_with("<!") {
                (0, ">")
            } else {
                let (tag, used) = tag(rest)?;
                self.at += used;
                return Some(Item::Tag(tag));
            };
            let e = rest[skip..].find(end)?;
            self.at += skip + e + end.len();
        }
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// The tag at the start of `s` (which begins with `<`) and the bytes it takes; `None` when it is cut or malformed.
fn tag(s: &str) -> Option<(Tag<'_>, usize)> {
    let b = s.as_bytes();
    let close = b.get(1) == Some(&b'/');
    let mut i = if close { 2 } else { 1 };
    let n0 = i;
    while i < b.len() && !is_space(b[i]) && b[i] != b'/' && b[i] != b'>' {
        i += 1;
    }
    if i == n0 {
        return None;
    }
    let name = local(&s[n0..i]);
    let mut attrs = Vec::new();
    loop {
        while i < b.len() && is_space(b[i]) {
            i += 1;
        }
        match b.get(i)? {
            b'>' => {
                let kind = if close { Kind::Close } else { Kind::Open };
                return Some((Tag { name, attrs, kind }, i + 1));
            }
            b'/' if !close => {
                if b.get(i + 1) != Some(&b'>') {
                    return None;
                }
                let t = Tag {
                    name,
                    attrs,
                    kind: Kind::Empty,
                };
                return Some((t, i + 2));
            }
            _ if close => return None,
            _ => {}
        }
        let k0 = i;
        while i < b.len() && !is_space(b[i]) && b[i] != b'=' && b[i] != b'>' && b[i] != b'/' {
            i += 1;
        }
        let key = &s[k0..i];
        while i < b.len() && is_space(b[i]) {
            i += 1;
        }
        if key.is_empty() || b.get(i) != Some(&b'=') {
            return None;
        }
        i += 1;
        while i < b.len() && is_space(b[i]) {
            i += 1;
        }
        let q = *b.get(i)?;
        if q != b'"' && q != b'\'' {
            return None;
        }
        let v0 = i + 1;
        let len = s[v0..].find(q as char)?;
        i = v0 + len + 1;
        if key == "xmlns" || key.starts_with("xmlns:") {
            continue;
        }
        attrs.push((local(key), unescape(&s[v0..v0 + len], true)));
    }
}

/// `s` with the five entities and character references replaced; in an attribute value (`attr`) a literal tab, line feed or carriage return becomes a
/// space, as XML normalizes it. An unknown or broken reference is kept as written.
pub fn unescape(s: &str, attr: bool) -> Cow<'_, str> {
    let ws = |c: char| attr && matches!(c, '\t' | '\n' | '\r');
    if !s.contains('&') && !s.contains(ws) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find(|c: char| c == '&' || ws(c)) {
        out.push_str(&rest[..p]);
        let t = &rest[p..];
        if !t.starts_with('&') {
            out.push(' ');
            rest = &t[1..];
            continue;
        }
        let ent = t[1..].find(';').map(|e| &t[1..1 + e]);
        let c = ent.and_then(|e| match e {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => e
                .strip_prefix("#x")
                .map(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| e.strip_prefix('#').map(|d| d.parse().ok()))
                .flatten()
                .and_then(char::from_u32),
        });
        match (c, ent) {
            (Some(c), Some(e)) => {
                out.push(c);
                rest = &t[e.len() + 2..];
            }
            _ => {
                out.push('&');
                rest = &t[1..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}
