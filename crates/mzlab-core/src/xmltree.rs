//! A small XML tree reader, enough for the method XML that Analyst stores in a `.dam` (`xml.etree.ElementTree` in `mzlab/reader/methodinfo.py`):
//! elements, text before the first child, entities, CDATA, comments and declarations. Anything ill-formed is `None`, like a `ParseError`.

impl El {
    /// `findtext(name)` of the first direct child, "" when it has no text.
    pub fn findtext(&self, name: &str) -> Option<&str> {
        self.children
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.text.as_str())
    }
    pub fn find(&self, name: &str) -> Option<&El> {
        self.children.iter().find(|c| c.name == name)
    }
    pub fn findall<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a El> {
        self.children.iter().filter(move |c| c.name == name)
    }
    /// Every element called `name`, the element itself included, in document order (`iter(name)`).
    pub fn iter<'a>(&'a self, name: &str, out: &mut Vec<&'a El>) {
        if self.name == name {
            out.push(self);
        }
        for c in &self.children {
            c.iter(name, out);
        }
    }
}

impl P<'_> {
    fn starts(&self, t: &str) -> bool {
        self.s[self.i..].starts_with(t.as_bytes())
    }
    fn skip_until(&mut self, t: &str) -> Option<()> {
        let p = self.s[self.i..]
            .windows(t.len())
            .position(|w| w == t.as_bytes())?;
        self.i += p + t.len();
        Some(())
    }
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn name(&mut self) -> Option<String> {
        let a = self.i;
        while self.i < self.s.len()
            && !matches!(
                self.s[self.i],
                b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/' | b'='
            )
        {
            self.i += 1;
        }
        (self.i > a).then(|| String::from_utf8_lossy(&self.s[a..self.i]).into_owned())
    }
    /// Skips comments, declarations and processing instructions.
    fn misc(&mut self) -> Option<()> {
        loop {
            self.ws();
            if self.starts("<?") {
                self.skip_until("?>")?;
            } else if self.starts("<!--") {
                self.skip_until("-->")?;
            } else if self.starts("<!DOCTYPE") {
                self.skip_until(">")?;
            } else {
                return Some(());
            }
        }
    }
    /// One `name="value"` pair, skipped.
    fn attribute(&mut self) -> Option<()> {
        self.name()?;
        self.ws();
        if self.s.get(self.i) != Some(&b'=') {
            return None;
        }
        self.i += 1;
        self.ws();
        let q = *self.s.get(self.i)?;
        if q != b'"' && q != b'\'' {
            return None;
        }
        self.i += 1;
        while *self.s.get(self.i)? != q {
            self.i += 1;
        }
        self.i += 1;
        Some(())
    }

    fn element(&mut self) -> Option<El> {
        if self.s.get(self.i) != Some(&b'<') || self.depth >= MAX_DEPTH {
            return None;
        }
        self.depth += 1;
        let el = self.element_inner();
        self.depth -= 1;
        el
    }

    fn element_inner(&mut self) -> Option<El> {
        self.i += 1;
        let mut el = El {
            name: self.name()?,
            ..Default::default()
        };
        loop {
            self.ws();
            match self.s.get(self.i)? {
                b'/' => {
                    self.i += 1;
                    return (self.s.get(self.i) == Some(&b'>')).then(|| {
                        self.i += 1;
                        el
                    });
                }
                b'>' => {
                    self.i += 1;
                    break;
                }
                _ => self.attribute()?,
            }
        }
        let mut text = Vec::new();
        loop {
            match self.s.get(self.i)? {
                b'<' if self.starts("</") => {
                    self.i += 2;
                    let n = self.name()?;
                    self.ws();
                    if n != el.name || self.s.get(self.i) != Some(&b'>') {
                        return None;
                    }
                    self.i += 1;
                    if el.children.is_empty() {
                        el.text = decode(&text)?;
                    }
                    return Some(el);
                }
                b'<' if self.starts("<!--") => self.skip_until("-->")?,
                b'<' if self.starts("<![CDATA[") => {
                    let a = self.i + 9;
                    self.skip_until("]]>")?;
                    if el.children.is_empty() {
                        text.extend(escape(&self.s[a..self.i - 3]));
                    }
                }
                b'<' if self.starts("<?") => self.skip_until("?>")?,
                b'<' => {
                    if el.children.is_empty() {
                        el.text = decode(&text)?;
                    }
                    let c = self.element()?;
                    el.children.push(c);
                }
                &c => {
                    if el.children.is_empty() {
                        text.push(c);
                    }
                    self.i += 1;
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct El {
    pub name: String,
    /// Text before the first child element (`Element.text`).
    pub text: String,
    pub children: Vec<El>,
}

/// How deep elements may nest: the method XML of an instrument has a dozen levels, and a recursion on 10 000 would exhaust the stack of the WASM.
const MAX_DEPTH: usize = 256;

struct P<'a> {
    s: &'a [u8],
    i: usize,
    depth: usize,
}

/// CDATA is literal text: its `&` and `<` must not be read as entities when the buffer is decoded.
fn escape(b: &[u8]) -> Vec<u8> {
    let mut o = Vec::with_capacity(b.len());
    for &c in b {
        match c {
            b'&' => o.extend_from_slice(b"&amp;"),
            b'<' => o.extend_from_slice(b"&lt;"),
            c => o.push(c),
        }
    }
    o
}

fn decode(b: &[u8]) -> Option<String> {
    let s = String::from_utf8_lossy(b);
    let mut out = String::new();
    let mut rest: &str = &s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        let e = rest[p..].find(';')? + p;
        let ent = &rest[p + 1..e];
        match ent {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => {
                let n = ent
                    .strip_prefix("#x")
                    .map(|h| u32::from_str_radix(h, 16))
                    .or_else(|| ent.strip_prefix('#').map(|d| d.parse()))?;
                out.push(char::from_u32(n.ok()?)?);
            }
        }
        rest = &rest[e + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// The root element of a document, or `None` when it is not well formed.
pub fn parse(text: &str) -> Option<El> {
    let mut p = P {
        s: text.as_bytes(),
        i: 0,
        depth: 0,
    };
    p.misc()?;
    let root = p.element()?;
    p.misc()?;
    (p.i == p.s.len()).then_some(root)
}

#[cfg(test)]
mod tests;
