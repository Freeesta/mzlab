//! Block parser of the Libro dialect: headings, paragraphs, lists, tables, boxes, equations, figures, questions, comments, raw and private blocks.
//! Renders straight to the HTML of the existing Teoria components; anything it does not know is a LibroError. Port of blocchi.py, liste.py, speciali.py, grezzo.py.
use std::collections::HashMap;

use crate::inline::{render, Ctx};
use crate::page::{known_snippet, Options};
use crate::scan::{
    cut, explicit_id, fig_line, first2, head, item, kicker, script_ref, slug, unsafe_match,
    word_dash,
};
use crate::{escape, LibroError};

pub(crate) type R<T> = Result<T, LibroError>;

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

    pub(crate) fn err<T>(&self, line: usize, msg: impl Into<String>) -> R<T> {
        Err(LibroError::new(self.file, line, msg))
    }

    pub(crate) fn inl(&mut self, text: &str, line: usize) -> R<String> {
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
}

pub struct Parser<'a> {
    pub(crate) file: &'a str,
    pub(crate) prefix: String,
    pub(crate) pubblico: bool,
    pub(crate) raw_ok: bool,
    pub ctx: Ctx,
    pub eq: HashMap<String, String>,
    pub fig: HashMap<String, String>,
    pub titoli: Vec<(usize, String)>,
    pub head: Vec<String>,
    pub(crate) caption: Option<String>,
    pub scripts: Vec<String>,
    pub scripts_before: Vec<String>,
    pub html_attrs: Vec<(String, String)>,
    pub snippets: Vec<String>,
    pub page_title: String,
    pub full_title: String,
    pub no_appname: bool,
    pub esteso: bool,
}
