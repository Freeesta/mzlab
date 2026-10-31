//! The generic reader of tables: finds the tables of a document whose header row names the wanted columns and turns their rows into entries. Which columns,
//! and in which annex of the text, is data in `normative/sources.json` (`params`), so a new list of the same shape needs no code.

use mzlab_core::json::Value;

use super::entry::{cas_tokens, Entry};
use super::markup::{self, Item, Node, Row};

impl Cfg {
    pub fn from_params(p: &Value) -> Cfg {
        let cols = p.get("columns");
        let g = |k: &str| strings(cols.and_then(|c| c.get(k)));
        let attr = cols
            .and_then(|c| c.get("attr"))
            .map(|a| {
                a.arr()
                    .iter()
                    .filter_map(|x| {
                        let m = x.get("match").and_then(Value::str)?;
                        Some((
                            m.to_string(),
                            x.get("label").and_then(Value::str).unwrap_or(m).to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Cfg {
            entry_ref: g("ref"),
            name: g("name"),
            cas: g("cas"),
            ec: g("ec"),
            dtxsid: g("dtxsid"),
            attr,
            annexes: strings(p.get("annexes")),
            prefix: p
                .get("prefix")
                .and_then(Value::str)
                .unwrap_or("")
                .to_string(),
        }
    }
}

/// Header patterns by column: a pattern matches a header when every word of it is in the header text (any case).
#[derive(Default)]
pub struct Cfg {
    pub entry_ref: Vec<String>,
    pub name: Vec<String>,
    pub cas: Vec<String>,
    pub ec: Vec<String>,
    pub dtxsid: Vec<String>,
    /// (pattern, label): columns kept as the attribute of the entry.
    pub attr: Vec<(String, String)>,
    /// Only the tables under these annex headings ("ANNEX I"); empty: every table.
    pub annexes: Vec<String>,
    /// Text put before the entry number in `entry_ref`.
    pub prefix: String,
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.map(|a| {
        a.arr()
            .iter()
            .filter_map(|x| x.str().map(String::from))
            .collect()
    })
    .unwrap_or_default()
}

pub fn header_matches(header: &str, pattern: &str) -> bool {
    let h = header.to_lowercase();
    pattern
        .split_whitespace()
        .all(|w| h.contains(&w.to_lowercase()))
}

/// The column index (counting column spans) of every cell of a row.
fn positions(row: &Row) -> Vec<usize> {
    let mut at = 0;
    row.iter()
        .map(|c| {
            let p = at;
            at += c.span.max(1);
            p
        })
        .collect()
}

fn cell_at(row: &Row, pos: usize) -> Option<&markup::Cell> {
    positions(row)
        .iter()
        .position(|&p| p == pos)
        .map(|i| &row[i])
}

struct Cols {
    entry_ref: Option<usize>,
    name: usize,
    cas: Option<usize>,
    ec: Option<usize>,
    dtxsid: Option<usize>,
    attr: Vec<(usize, String)>,
}

fn find(row: &Row, pats: &[String]) -> Option<usize> {
    let pos = positions(row);
    row.iter()
        .enumerate()
        .find(|(_, c)| pats.iter().any(|p| header_matches(&c.text(), p)))
        .map(|(i, _)| pos[i])
}

fn header_cols(row: &Row, cfg: &Cfg) -> Option<Cols> {
    let name = find(row, &cfg.name)?;
    let cas = find(row, &cfg.cas);
    let ec = find(row, &cfg.ec);
    let dtxsid = find(row, &cfg.dtxsid);
    if cas.is_none() && ec.is_none() && dtxsid.is_none() {
        return None;
    }
    let pos = positions(row);
    let mut attr = Vec::new();
    for (pat, label) in &cfg.attr {
        for (i, c) in row.iter().enumerate() {
            if header_matches(&c.text(), pat) {
                attr.push((pos[i], label.clone()));
            }
        }
    }
    Some(Cols {
        entry_ref: find(row, &cfg.entry_ref),
        name,
        cas,
        ec,
        dtxsid,
        attr,
    })
}

fn text_at(row: &Row, pos: Option<usize>) -> String {
    pos.and_then(|p| cell_at(row, p))
        .map(|c| c.text())
        .unwrap_or_default()
}

/// The `label=value` pairs of the attribute columns of a row.
fn attr_text(row: &Row, cols: &Cols) -> String {
    cols.attr
        .iter()
        .filter_map(|(p, l)| Some((l, text_at(row, Some(*p)))).filter(|(_, v)| !v.is_empty()))
        .map(|(l, v)| format!("{l}={v}"))
        .collect::<Vec<_>>()
        .join(" ; ")
}

/// Entries of one table: the first of its first four rows that names the columns is the header, the rows after it are entries.
fn read_one(rows: &[Row], cfg: &Cfg, heading: &str, counter: &mut usize) -> Vec<Entry> {
    let Some((h, cols)) = rows
        .iter()
        .take(4)
        .enumerate()
        .find_map(|(i, r)| header_cols(r, cfg).map(|c| (i, c)))
    else {
        return Vec::new();
    };
    let width = [
        Some(cols.name),
        cols.cas,
        cols.ec,
        cols.dtxsid,
        cols.entry_ref,
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or(0)
        + 1;
    let mut out = Vec::new();
    for row in &rows[h + 1..] {
        let span: usize = row.iter().map(|c| c.span.max(1)).sum();
        if span < width || row.len() < 2 {
            continue;
        }
        let name = text_at(row, Some(cols.name));
        // a note inside the table spans the data columns; "Reserved" rows hold no substance
        let spans = cell_at(row, cols.name).is_some_and(|c| c.span > 1);
        if name.is_empty()
            || spans
            || name
                .trim_matches(['[', ']'])
                .eq_ignore_ascii_case("reserved")
        {
            continue;
        }
        let num = text_at(row, cols.entry_ref);
        let carried = cols
            .entry_ref
            .and_then(|p| cell_at(row, p))
            .is_some_and(|c| c.carried);
        *counter += 1;
        let num = if num.is_empty() {
            counter.to_string()
        } else {
            num
        };
        let entry_ref = [cfg.prefix.as_str(), heading, &format!("no. {num}")]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let mut cas_text = text_at(row, cols.cas);
        if cols.cas.is_none() {
            cas_text = name.clone();
        }
        let mut e = Entry::new(&entry_ref, &name, &cas_text);
        e.ec = text_at(row, cols.ec);
        e.dtxsid = text_at(row, cols.dtxsid);
        e.attr = attr_text(row, &cols);
        // rows under a numbered row that spans them are its members: "(28) PAH" then its five PAHs
        if carried
            && let Some(head) = out
                .iter()
                .find(|o: &&Entry| o.entry_ref == e.entry_ref && o.group_entry.is_empty())
        {
            e.group_entry = head.name.clone();
        }
        out.push(e);
    }
    out
}

fn title_case_annex(s: &str) -> String {
    let mut w = s.split_whitespace();
    let first = w.next().unwrap_or("");
    let head: String = first
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + &first[c.len_utf8()..].to_lowercase())
        .unwrap_or_default();
    std::iter::once(head)
        .chain(w.map(String::from))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Entries of every matching table of the document, in order; the annex and part headings above a table give the start of `entry_ref`.
pub fn read(root: &Node, cfg: &Cfg) -> Vec<Entry> {
    let mut out = Vec::new();
    let (mut annex, mut part) = (String::new(), String::new());
    let mut counter = 0;
    for item in markup::outline(root) {
        match item {
            Item::Para(t) => {
                let words = t.split_whitespace().count();
                let up = t.to_uppercase();
                if (1..=2).contains(&words)
                    && (up == "ANNEX" || up.starts_with("ANNEX ") || up.starts_with("APPENDIX "))
                {
                    annex = t.trim().to_string();
                    part.clear();
                    counter = 0;
                } else if words == 2 && up.starts_with("PART ") {
                    part = t.trim().to_string();
                    counter = 0;
                }
            }
            Item::Table(t) => {
                if !cfg.annexes.is_empty()
                    && !cfg.annexes.iter().any(|a| a.eq_ignore_ascii_case(&annex))
                {
                    continue;
                }
                let heading = [annex.as_str(), part.as_str()]
                    .iter()
                    .filter(|s| !s.is_empty())
                    .map(|s| title_case_annex(s))
                    .collect::<Vec<_>>()
                    .join(" ");
                out.extend(read_one(&markup::rows(t), cfg, &heading, &mut counter));
            }
        }
    }
    out
}

/// Every CAS number of the entries, for tests and coverage.
pub fn cas_of(entries: &[Entry]) -> Vec<String> {
    entries.iter().flat_map(|e| cas_tokens(&e.cas)).collect()
}
