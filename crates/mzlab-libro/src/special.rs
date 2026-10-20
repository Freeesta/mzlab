//! Special blocks of the Libro dialect (equations, boxes, tables, figures, questions): a second `impl` of the block parser in blocks.rs. Port of speciali.py.
use crate::blocks::{Parser, R};
use crate::scan::{eq_end, follow_id, numcell};
use crate::{escape, is_w};

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

impl Parser<'_> {
    pub(crate) fn equation(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
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

    pub(crate) fn boxed(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
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

    pub(crate) fn table(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
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

    pub(crate) fn figure(
        &mut self,
        f: (String, String, Option<String>, String),
        at: usize,
    ) -> R<String> {
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

    pub(crate) fn question(&mut self, lines: &[&str], i: usize, base: usize) -> R<(String, usize)> {
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
