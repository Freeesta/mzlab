//! Block parser of the Libro dialect: headings, paragraphs, lists, tables, boxes, equations, figures, questions, comments, raw and private blocks.
//! Renders straight to the HTML of the existing Teoria components; anything it does not know is a LibroError. Port of blocchi.py, liste.py, speciali.py, grezzo.py.
use std::collections::HashMap;

use crate::inline::{render, Ctx};
use crate::page::{known_snippet, Options};
use crate::{escape, is_w, LibroError};

type R<T> = Result<T, LibroError>;

const BOXES: [(&str, &str, &str); 8] = [
    ("nota", "key", "Concetto chiave"),
    ("attenzione", "warn", "Errore frequente"),
    ("lab", "lab", "Esempio"),
    ("formula", "math", "Formula"),
    ("esercizio", "gym", "Esercizio"),
    ("lr", "lr", "Bassa risoluzione"),
    ("hr", "hr", "Alta risoluzione"),
    ("oral", "oral", "Ripasso"),
];

pub struct Parser<'a> {
    file: &'a str,
    prefix: String,
    pubblico: bool,
    raw_ok: bool,
    pub ctx: Ctx,
    pub eq: HashMap<String, String>,
    pub fig: HashMap<String, String>,
    pub titoli: Vec<(usize, String)>,
    pub head: Vec<String>,
    caption: Option<String>,
    pub scripts: Vec<String>,
    pub scripts_before: Vec<String>,
    pub html_attrs: Vec<(String, String)>,
    pub snippets: Vec<String>,
    pub page_title: String,
    pub full_title: String,
    pub no_appname: bool,
    pub esteso: bool,
}

/// Python `s[a:-b]` on characters.
fn cut(s: &str, a: usize, b: usize) -> String {
    let c: Vec<char> = s.chars().collect();
    if a + b >= c.len() {
        String::new()
    } else {
        c[a..c.len() - b].iter().collect()
    }
}

fn first2(s: &str) -> String {
    s.chars().take(2).collect()
}

/// `ITEM = ^([-*]|\d+\.) +(.*)$`: (marker starts with a digit, item text).
fn item(ln: &str) -> Option<(bool, &str)> {
    let c = ln.as_bytes();
    let mut p = match c.first()? {
        b'-' | b'*' => 1,
        d if d.is_ascii_digit() => {
            let n = c.iter().take_while(|x| x.is_ascii_digit()).count();
            if c.get(n) != Some(&b'.') {
                return None;
            }
            n + 1
        }
        _ => return None,
    };
    if c.get(p) != Some(&b' ') {
        return None;
    }
    while c.get(p) == Some(&b' ') {
        p += 1;
    }
    Some((c[0].is_ascii_digit(), &ln[p..]))
}

/// `HEAD = ^(#{1,3}) +(.+?)\s*$`: (level, text).
fn head(ln: &str) -> Option<(usize, String)> {
    let level = ln.bytes().take_while(|&b| b == b'#').count();
    if !(1..=3).contains(&level) {
        return None;
    }
    let rest = &ln[level..];
    let spaces = rest.bytes().take_while(|&b| b == b' ').count();
    if spaces == 0 {
        return None;
    }
    let text = rest[spaces..].trim_end();
    if !text.is_empty() {
        Some((level, text.to_string()))
    } else if spaces >= 2 && rest[spaces..].trim().is_empty() {
        Some((level, " ".to_string()))
    } else {
        None
    }
}

/// `\s*\{#([\w-]+)\}\s*$` searched in `text`: (start of the match, id).
fn explicit_id(text: &str) -> Option<(usize, String)> {
    let t = text.trim_end();
    let body = t.strip_suffix('}')?;
    let open = body.rfind("{#")?;
    let id = &body[open + 2..];
    if id.is_empty() || !id.chars().all(|c| is_w(c) || c == '-') {
        return None;
    }
    Some((body[..open].trim_end().len(), id.to_string()))
}

/// `\s*\{kicker: *([^}]+)\}\s*$` searched in `text`: (start of the match, kicker).
fn kicker(text: &str) -> Option<(usize, String)> {
    let mut from = 0;
    while let Some(p) = text[from..].find("{kicker:") {
        let at = from + p;
        let after = &text[at + 8..];
        if let Some(close) = after.find('}') {
            if after[close + 1..].trim().is_empty() && close > 0 {
                let raw = &after[..close];
                let k = raw.trim_start_matches(' ');
                let k = if k.is_empty() { " " } else { k };
                return Some((text[..at].trim_end().len(), k.to_string()));
            }
        }
        from = at + 1;
    }
    None
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in s.to_lowercase().chars() {
        let base = match ch {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            c => c,
        };
        out.push(if base.is_ascii_lowercase() || base.is_ascii_digit() {
            base
        } else {
            '-'
        });
    }
    let mut collapsed = String::new();
    for ch in out.chars() {
        if ch == '-' && collapsed.ends_with('-') {
            continue;
        }
        collapsed.push(ch);
    }
    let t = collapsed.strip_prefix('-').unwrap_or(&collapsed);
    let t = t.strip_suffix('-').unwrap_or(t);
    t.chars().take(48).collect()
}

/// `FIG`: `![alt](src "title") {#fig:id}` -> (alt, src, title, id).
fn fig_line(ln: &str) -> Option<(String, String, Option<String>, String)> {
    let rest = ln.strip_prefix("![")?;
    let a = rest.find(']')?;
    let alt = &rest[..a];
    let rest = rest[a + 1..].strip_prefix('(')?;
    let n = rest
        .find(|c: char| c == ')' || c.is_whitespace() || c == '"')
        .unwrap_or(rest.len());
    if n == 0 {
        return None;
    }
    let src = &rest[..n];
    let mut rest = &rest[n..];
    let mut cap = None;
    if let Some(r) = rest.strip_prefix(" \"") {
        if let Some(q) = r.find('"') {
            cap = Some(r[..q].to_string());
            rest = &r[q + 1..];
        }
    }
    let rest = rest
        .strip_prefix(')')?
        .trim_start()
        .strip_prefix("{#fig:")?;
    let e = rest.find('}')?;
    let id = &rest[..e];
    if id.is_empty() || !id.chars().all(|c| is_w(c) || c == '-') || !rest[e + 1..].trim().is_empty()
    {
        return None;
    }
    Some((alt.to_string(), src.to_string(), cap, id.to_string()))
}

/// `EQEND.search`: start of the closing `$$ [{#eq:id}]` at the end of `s`, and the id.
fn eq_end(s: &[char]) -> Option<(usize, Option<String>)> {
    for p in 0..s.len().saturating_sub(1) {
        if s[p] != '$' || s[p + 1] != '$' {
            continue;
        }
        let mut k = p + 2;
        while k < s.len() && s[k].is_whitespace() {
            k += 1;
        }
        let mut id = None;
        if s[k..].starts_with(&['{', '#', 'e', 'q', ':']) {
            let st = k + 5;
            let mut j = st;
            while j < s.len() && (is_w(s[j]) || s[j] == '-') {
                j += 1;
            }
            if j > st && s.get(j) == Some(&'}') {
                id = Some(s[st..j].iter().collect());
                k = j + 1;
                while k < s.len() && s[k].is_whitespace() {
                    k += 1;
                }
            }
        }
        if k == s.len() {
            return Some((p, id));
        }
    }
    None
}

fn follow_id(ln: &str) -> Option<String> {
    let r = ln.strip_prefix("{#eq:")?;
    let e = r.find('}')?;
    let id = &r[..e];
    (!id.is_empty() && id.chars().all(|c| is_w(c) || c == '-') && r[e + 1..].trim().is_empty())
        .then(|| id.to_string())
}

/// `NUMCELL = ^[−+-]?\d[\d.,\s·⁰¹²³⁴⁵⁶⁷⁸⁹⁻]*$`
fn numcell(c: &str) -> bool {
    let mut it = c.chars().peekable();
    if it.peek().is_some_and(|x| matches!(x, '−' | '+' | '-')) {
        it.next();
    }
    if !it.next().is_some_and(|x| x.is_ascii_digit()) {
        return false;
    }
    it.all(|x| x.is_ascii_digit() || x.is_whitespace() || "., ·⁰¹²³⁴⁵⁶⁷⁸⁹⁻".contains(x))
}

fn unsafe_match(body: &str) -> Option<String> {
    const TAGS: [&str; 8] = [
        "script", "iframe", "object", "embed", "link", "meta", "base", "form",
    ];
    let c: Vec<char> = body
        .chars()
        .map(|x| x.to_lowercase().next().unwrap_or(x))
        .collect();
    let at = |p: usize, w: &str| {
        c[p..]
            .iter()
            .zip(w.chars())
            .filter(|(a, b)| a == &b)
            .count()
            == w.chars().count()
            && c.len() - p >= w.chars().count()
    };
    for p in 0..c.len() {
        if c[p] == '<' {
            let mut k = p + 1;
            while k < c.len() && c[k].is_whitespace() {
                k += 1;
            }
            for t in TAGS {
                let n = t.len();
                if at(k, t) && c.get(k + n).is_none_or(|&x| !is_w(x)) {
                    return Some(c[p..k + n].iter().collect::<String>().trim().to_string());
                }
            }
        }
        if c[p].is_whitespace() && c.get(p + 1) == Some(&'o') && c.get(p + 2) == Some(&'n') {
            let mut k = p + 3;
            while k < c.len() && is_w(c[k]) {
                k += 1;
            }
            if k > p + 3 {
                let mut m = k;
                while m < c.len() && c[m].is_whitespace() {
                    m += 1;
                }
                if c.get(m) == Some(&'=') {
                    return Some(c[p..=m].iter().collect::<String>().trim().to_string());
                }
            }
        }
        if at(p, "javascript:") {
            return Some("javascript:".to_string());
        }
    }
    None
}

fn word_dash(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| is_w(c) || c == '-')
}

fn script_ref(v: &str) -> Option<(bool, &str)> {
    let (before, rest) = if let Some(r) = v.strip_prefix("script-before:") {
        (true, r)
    } else {
        (false, v.strip_prefix("script:")?)
    };
    let r = rest.trim_start_matches(' ');
    (r.len() > 3
        && r.ends_with(".js")
        && r.chars().all(|c| is_w(c) || matches!(c, '.' | '/' | '-')))
    .then_some((before, r))
}

impl<'a> Parser<'a> {
    pub fn new(file: &'a str, prefix: &str, opt: &Options) -> Self {
        Parser {
            file,
            prefix: prefix.to_string(),
            pubblico: opt.pubblico,
            raw_ok: opt.raw_ok,
            ctx: Ctx {
                teoria: opt.teoria,
                ..Ctx::default()
            },
            eq: HashMap::new(),
            fig: HashMap::new(),
            titoli: Vec::new(),
            head: Vec::new(),
            caption: None,
            scripts: Vec::new(),
            scripts_before: Vec::new(),
            html_attrs: Vec::new(),
            snippets: Vec::new(),
            page_title: String::new(),
            full_title: String::new(),
            no_appname: false,
            esteso: false,
        }
    }

    fn err<T>(&self, line: usize, msg: impl Into<String>) -> R<T> {
        Err(LibroError::new(self.file, line, msg))
    }

    fn inl(&mut self, text: &str, line: usize) -> R<String> {
        render(text, self.file, line, &mut self.ctx)
    }

    pub fn blocks(&mut self, lines: &[&str], base: usize) -> R<String> {
        let mut out: Vec<String> = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            if lines[i].trim().is_empty() {
                i += 1;
                continue;
            }
            let (html, next) = self.one(lines, i, base)?;
            if !html.is_empty() {
                out.push(html);
            }
            i = next;
        }
        Ok(out.join("\n"))
    }

    fn one(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
        let ln = lines[i];
        let at = base + i;
        if ln.starts_with("<!--") {
            return self.comment(lines, i, base);
        }
        if let Some(h) = head(ln) {
            return Ok((self.heading(h, at)?, i + 1));
        }
        if ln.trim_end() == "```html" {
            return self.raw(lines, i, base);
        }
        if ln.trim_end() == "::: duo" {
            return self.duo(lines, i, base);
        }
        if ln.starts_with("$$") {
            return self.equation(lines, i, base);
        }
        if ln.starts_with('>') {
            return self.boxed(lines, i, base);
        }
        if ln.starts_with('|') {
            return self.table(lines, i, base);
        }
        if let Some(f) = fig_line(ln) {
            return Ok((self.figure(f, at)?, i + 1));
        }
        if ln.starts_with("??? ") {
            return self.question(lines, i, base);
        }
        if let Some(c) = ln.strip_prefix("Tabella: ") {
            self.caption = Some(c.trim().to_string());
            return Ok((String::new(), i + 1));
        }
        if item(ln).is_some() {
            return self.listing(lines, i, base);
        }
        self.paragraph(lines, i, base)
    }

    fn heading(&mut self, h: (usize, String), at: usize) -> R<String> {
        let (level, mut text) = h;
        let k = if level == 1 { kicker(&text) } else { None };
        let kick = k
            .as_ref()
            .map(|k| k.1.clone())
            .unwrap_or_else(|| self.prefix.clone());
        if let Some((start, _)) = &k {
            text.truncate(*start);
        }
        let e = explicit_id(&text);
        if let Some((start, _)) = &e {
            text.truncate(*start);
        }
        self.titoli.push((level, text.clone()));
        let h = self.inl(&text, at)?;
        if level == 1 {
            return Ok(format!(
                "<div class=\"kicker\">{}</div>\n<h1>{h}</h1>",
                escape(&kick, true)
            ));
        }
        let hid = match e {
            Some((_, id)) => Some(id),
            None if self.ctx.teoria => None,
            None => Some(slug(&text)),
        };
        let id = hid
            .filter(|x| !x.is_empty())
            .map(|x| format!(" id=\"{x}\""))
            .unwrap_or_default();
        Ok(format!("<h{level}{id}>{h}</h{level}>"))
    }

    fn paragraph(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
        let mut j = i;
        while j < lines.len() && !lines[j].trim().is_empty() {
            let l = lines[j];
            if j != i
                && (head(l).is_some()
                    || item(l).is_some()
                    || matches!(first2(l).as_str(), "$$" | "> " | "??" | "<!" | "``" | "::")
                    || l.starts_with('|'))
            {
                break;
            }
            j += 1;
        }
        let text: String = lines[i..j]
            .iter()
            .map(|x| x.trim())
            .collect::<Vec<_>>()
            .join(" ");
        let (cls, body) = match ["lead", "note"].iter().find_map(|c| {
            let r = text.strip_prefix(&format!("{{.{c}}} "))?;
            Some((*c, r.trim_start_matches(' ').to_string()))
        }) {
            Some((c, r)) => (format!(" class=\"{c}\""), r),
            None => (String::new(), text),
        };
        Ok((format!("<p{cls}>{}</p>", self.inl(&body, base + i)?), j))
    }

    fn listing(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
        let ordered = lines[i].as_bytes()[0].is_ascii_digit();
        let tag = if ordered { "ol" } else { "ul" };
        let n = lines.len();
        let mut items: Vec<(Vec<&str>, usize)> = Vec::new();
        let mut j = i;
        while j < n {
            let Some((digit, first)) = item(lines[j]) else {
                break;
            };
            if digit != ordered {
                break;
            }
            let mut body = vec![first];
            let mut k = j + 1;
            while k < n
                && (lines[k].starts_with("  ")
                    || (lines[k].trim().is_empty() && Self::indented_next(lines, k)))
            {
                body.push(if lines[k].starts_with("   ") {
                    &lines[k][3..]
                } else {
                    lines[k].trim()
                });
                k += 1;
            }
            items.push((body, j));
            j = k;
            while j < n && lines[j].trim().is_empty() && j + 1 < n && item(lines[j + 1]).is_some() {
                j += 1;
            }
        }
        let mut parts = Vec::new();
        for (b, s) in items {
            parts.push(self.list_item(&b, base + s)?);
        }
        Ok((format!("<{tag}>\n{}\n</{tag}>", parts.join("\n")), j))
    }

    fn indented_next(lines: &[&str], k: usize) -> bool {
        lines[k + 1..]
            .iter()
            .find(|x| !x.trim().is_empty())
            .is_some_and(|x| x.starts_with("  "))
    }

    fn list_item(&mut self, body: &[&str], at: usize) -> R<String> {
        let inner = self.blocks(body, at)?;
        let simple = inner.starts_with("<p>") && inner.matches("<p>").count() == 1;
        Ok(format!(
            "<li>{}</li>",
            if simple { cut(&inner, 3, 4) } else { inner }
        ))
    }

    fn raw(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
        if !self.raw_ok {
            return self.err(base + i, "a raw ```html block is only allowed in the imported chapters listed in tools/libro/importati.txt");
        }
        let Some(j) = (i + 1..lines.len()).find(|&k| lines[k].trim_end() == "```") else {
            return self.err(base + i, "```html block without closing ```");
        };
        let body = lines[i + 1..j].join("\n");
        if let Some(bad) = unsafe_match(&body) {
            return self.err(
                base + i,
                format!("raw HTML block contains {bad:?}, which is not allowed"),
            );
        }
        Ok((body, j + 1))
    }

    fn duo(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
        let Some(j) = (i + 1..lines.len()).find(|&k| lines[k].trim_end() == ":::") else {
            return self.err(base + i, "::: duo without closing :::");
        };
        let inner = self.blocks(&lines[i + 1..j], base + i + 1)?;
        Ok((format!("<div class=\"duo\">{inner}\n</div>"), j + 1))
    }

    fn comment(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
        let ln = lines[i].trim();
        if ln == "<!-- privato -->" {
            let Some(j) = (i + 1..lines.len()).find(|&k| lines[k].trim() == "<!-- /privato -->")
            else {
                return self.err(
                    base + i,
                    "<!-- privato --> without closing <!-- /privato -->",
                );
            };
            let inner = if self.pubblico {
                String::new()
            } else {
                format!(
                    "<div class=\"privato\" data-privato>\n{}\n</div>",
                    self.blocks(&lines[i + 1..j], base + i + 1)?
                )
            };
            return Ok((inner, j + 1));
        }
        if !ln.ends_with("-->") {
            return self.err(base + i, "multi-line HTML comments are not supported");
        }
        if self.titoli.is_empty() {
            let v = if ln.len() >= 7 {
                ln[4..ln.len() - 3].trim()
            } else {
                ""
            };
            self.directive(v, base + i)?;
        }
        Ok((String::new(), i + 1))
    }

    /// One `<!-- ... -->` line before the first heading: a directive or a plain head comment.
    fn directive(&mut self, v: &str, at: usize) -> R<()> {
        let attr = v.strip_prefix("html:").and_then(|r| {
            let (k, val) = r.trim_start_matches(' ').split_once('=')?;
            (k.starts_with("data-") && word_dash(&k[5..]) && word_dash(val))
                .then(|| (k.to_string(), val.to_string()))
        });
        let named = |p: &str| {
            v.strip_prefix(p)
                .map(|r| r.trim_start_matches(' ').to_string())
                .filter(|r| !r.is_empty())
        };
        let inline = v
            .strip_prefix("inline:")
            .map(|r| r.trim_start_matches(' '))
            .filter(|r| word_dash(r));
        if let Some(a) = attr {
            self.html_attrs.push(a);
        } else if v == "esteso" {
            self.esteso = true;
        } else if let Some(name) = inline {
            if !known_snippet(name) {
                return self.err(at, format!("unknown inline snippet {name}"));
            }
            self.snippets.push(name.to_string());
        } else if let Some(t) = named("full-title:") {
            self.full_title = t;
        } else if v == "no-appname" {
            self.no_appname = true;
        } else if let Some(t) = named("title:") {
            self.page_title = t;
        } else if let Some((before, src)) = script_ref(v) {
            if before {
                &mut self.scripts_before
            } else {
                &mut self.scripts
            }
            .push(src.to_string());
        } else {
            self.head.push(v.to_string());
        }
        Ok(())
    }

    fn equation(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
        let mut j = i;
        while j < lines.len() {
            let c: Vec<char> = lines[j].chars().collect();
            if eq_end(&c).is_some() && (j > i || lines[j].trim().chars().count() > 2) {
                break;
            }
            j += 1;
        }
        if j >= lines.len() {
            return self.err(base + i, "equation $$ without closing $$");
        }
        let body: Vec<char> = lines[i..=j].join(" ").chars().collect();
        let (start, mut eid) = eq_end(&body).unwrap_or((0, None));
        let tex: String = if start > 2 {
            body[2..start].iter().collect::<String>()
        } else {
            String::new()
        }
        .trim()
        .to_string();
        let nxt = (j + 1..(j + 3).min(lines.len())).find(|&k| !lines[k].trim().is_empty());
        if eid.is_none() {
            if let Some(f) = nxt.and_then(|k| follow_id(lines[k]).map(|id| (k, id))) {
                eid = Some(f.1);
                j = f.0;
            }
        }
        let mut label = String::new();
        if let Some(id) = &eid {
            if self.eq.contains_key(id) {
                return self.err(base + i, format!("duplicate equation id {id}"));
            }
            let num = format!("{}.{}", self.prefix, self.eq.len() + 1);
            self.eq.insert(id.clone(), num.clone());
            label = format!("<span class=\"no\">({num})</span>");
        }
        let idattr = eid.map(|id| format!(" id=\"eq-{id}\"")).unwrap_or_default();
        Ok((
            format!(
                "<span class=\"eq\"{idattr}><span class=\"tex\">{}</span>{label}</span>",
                escape(&tex, true)
            ),
            j + 1,
        ))
    }

    fn boxed(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
        let mut j = i;
        while j < lines.len() && lines[j].starts_with('>') {
            j += 1;
        }
        let inner: Vec<&str> = lines[i..j]
            .iter()
            .map(|x| x[1..].strip_prefix(' ').unwrap_or(&x[1..]))
            .collect();
        let bad = || {
            self.err(base + i, "a quote must start with [!nota], [!attenzione], [!lab], [!formula] or [!esercizio]")
        };
        let Some(r) = inner[0].strip_prefix("[!") else {
            return bad();
        };
        let Some(e) = r.find(']') else { return bad() };
        let Some(&(_, cls, default)) = BOXES.iter().find(|b| b.0 == &r[..e]) else {
            return bad();
        };
        let rest = r[e + 1..].trim_start();
        let title = if rest.is_empty() {
            default.to_string()
        } else {
            self.inl(rest, base + i)?
        };
        let inner_html = self.blocks(&inner[1..], base + i + 1)?;
        Ok((
            format!("<div class=\"box {cls}\"><b>{title}</b>\n{inner_html}\n</div>"),
            j,
        ))
    }

    fn table(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
        let mut j = i;
        while j < lines.len() && lines[j].starts_with('|') {
            j += 1;
        }
        let rows: Vec<Vec<String>> = lines[i..j]
            .iter()
            .map(|x| {
                x.trim()
                    .trim_matches('|')
                    .split('|')
                    .map(|c| c.trim().to_string())
                    .collect()
            })
            .collect();
        let sep = |c: &String| {
            let t = c.strip_prefix(':').unwrap_or(c);
            let t = t.strip_suffix(':').unwrap_or(t);
            !t.is_empty() && t.chars().all(|x| x == '-')
        };
        if rows.len() < 3 || !rows[1].iter().all(sep) {
            return self.err(
                base + i,
                "a table needs a header row and a |---| separator row",
            );
        }
        let Some(cap) = self.caption.take() else {
            return self.err(
                base + i,
                "a table needs a caption: put 'Tabella: ...' on the line before",
            );
        };
        let mut head = String::new();
        for c in &rows[0] {
            head.push_str(&format!(
                "<th scope=\"col\">{}</th>",
                self.inl(c, base + i)?
            ));
        }
        let mut body = Vec::new();
        for (k, r) in rows[2..].iter().enumerate() {
            if r.len() != rows[0].len() {
                return self.err(
                    base + i + 2 + k,
                    format!("row has {} cells, header has {}", r.len(), rows[0].len()),
                );
            }
            let mut cells = String::new();
            for c in r {
                let ncls = if numcell(c) { " class=\"n\"" } else { "" };
                cells.push_str(&format!(
                    "<td{ncls}>{}</td>",
                    self.inl(c, base + i + 2 + k)?
                ));
            }
            body.push(format!("<tr>{cells}</tr>"));
        }
        let capt = self.inl(&cap, base + i)?;
        Ok((
            format!(
                "<table>\n<caption>{capt}</caption>\n<tr>{head}</tr>\n{}\n</table>",
                body.join("\n")
            ),
            j,
        ))
    }

    fn figure(&mut self, f: (String, String, Option<String>, String), at: usize) -> R<String> {
        let (alt, src, cap, fid) = f;
        if alt.is_empty() || alt.chars().count() > 125 {
            return self.err(
                at,
                "figure alt text is required and must be at most 125 characters",
            );
        }
        if !src.chars().all(|c| is_w(c) || matches!(c, '.' | '/' | '-')) {
            return self.err(at, format!("unsupported figure path {src:?}"));
        }
        let num = format!("{}.{}", self.prefix, self.fig.len() + 1);
        self.fig.insert(fid.clone(), num.clone());
        let c = self.inl(cap.as_deref().unwrap_or(""), at)?;
        Ok(format!("<figure id=\"fig-{fid}\">\n<img src=\"{src}\" alt=\"{}\">\n<figcaption><b>Figura {num}</b> {c}</figcaption>\n</figure>", escape(&alt, true)))
    }

    fn question(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
        let mut j = i + 1;
        while j < lines.len() && !lines[j].trim().is_empty() {
            j += 1;
        }
        if j == i + 1 {
            return self.err(
                base + i,
                "a question '??? ...' needs the answer on the next line",
            );
        }
        let joined = lines[i + 1..j]
            .iter()
            .map(|x| x.trim())
            .collect::<Vec<_>>()
            .join(" ");
        let ans = self.inl(&joined, base + i + 1)?;
        let q = self.inl(lines[i][4..].trim(), base + i)?;
        Ok((
            format!("<details class=\"q\"><summary>{q}</summary><p>{ans}</p></details>"),
            j,
        ))
    }
}
