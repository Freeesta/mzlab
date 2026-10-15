// Ported from Attributio campaign.py, readers/mzmine.py.
//! Delimited tables the way pandas reads them: one header, cells as text, the default «missing» words (`NA`, `nan`, empty, …) as missing, numbers on request
//! (`pd.to_numeric(..., errors="coerce")`). The cells live in one text buffer with an offset pair each (a merged mzMine table has hundreds of columns).

use super::error::{CampaignError, Result};
use std::collections::HashMap;

impl Frame {
    /// An empty frame with the given columns.
    pub fn with_columns(columns: &[&str]) -> Frame {
        let columns: Vec<String> = columns.iter().map(|c| c.to_string()).collect();
        Frame {
            index: index_of(&columns),
            columns,
            ..Frame::default()
        }
    }
    pub fn columns(&self) -> &[String] {
        &self.columns
    }
    pub fn len(&self) -> usize {
        self.nrows
    }
    pub fn is_empty(&self) -> bool {
        self.nrows == 0
    }
    /// Position of a column (the first one with that name).
    pub fn col(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }
    /// The raw text of a cell.
    pub fn cell(&self, r: usize, c: usize) -> &str {
        let (s, n) = self.cells[r * self.columns.len() + c];
        &self.data[s as usize..(s + n) as usize]
    }
    /// The text of a cell, `None` when pandas would read it as missing.
    pub fn str_na(&self, r: usize, c: usize) -> Option<&str> {
        Some(self.cell(r, c)).filter(|s| !is_na(s))
    }
    pub fn num(&self, r: usize, c: usize) -> Option<f64> {
        to_num(self.cell(r, c))
    }
    /// `astype(int)` of a numeric cell (truncated), `None` when missing.
    pub fn int(&self, r: usize, c: usize) -> Option<i64> {
        self.num(r, c).map(|v| v as i64)
    }
    /// The cells of a column by name (empty texts when the column is absent).
    pub fn column_texts(&self, name: &str) -> Vec<&str> {
        match self.col(name) {
            Some(c) => (0..self.nrows).map(|r| self.cell(r, c)).collect(),
            None => vec![""; self.nrows],
        }
    }
    pub fn set(&mut self, r: usize, c: usize, text: &str) {
        let start = self.data.len();
        self.data.push_str(text);
        self.cells[r * self.columns.len() + c] = (start as u32, text.len() as u32);
    }
    /// Adds a column at the end (a value per row).
    pub fn add_column(&mut self, name: &str, values: &[String]) {
        let old = self.columns.len();
        let mut cells = Vec::with_capacity(self.nrows * (old + 1));
        for r in 0..self.nrows {
            cells.extend_from_slice(&self.cells[r * old..(r + 1) * old]);
            let start = self.data.len();
            let v = values.get(r).map_or("", |s| s.as_str());
            self.data.push_str(v);
            cells.push((start as u32, v.len() as u32));
        }
        self.cells = cells;
        self.columns.push(name.to_string());
        self.index.entry(name.to_string()).or_insert(old);
    }
    /// Keeps the rows where `keep[r]` is true (the order stays).
    pub fn retain_rows(&mut self, keep: &[bool]) {
        let w = self.columns.len();
        let mut cells = Vec::new();
        let mut n = 0;
        for (r, k) in keep.iter().enumerate().take(self.nrows) {
            if *k {
                cells.extend_from_slice(&self.cells[r * w..(r + 1) * w]);
                n += 1;
            }
        }
        self.cells = cells;
        self.nrows = n;
    }
    /// Reads a delimited text (BOM, `\r\n`, quoted fields with doubled quotes and newlines; blank lines are skipped). Repeated column names get a `.1`, `.2` suffix.
    pub fn parse(text: &str, sep: char) -> Result<Frame> {
        let mut f = Frame::default();
        let mut width = 0;
        let mut first = true;
        let mut line = 0usize;
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut rec: Vec<std::borrow::Cow<str>> = Vec::new();
        let mut pos = 0;
        while pos < text.len() {
            line += 1;
            pos = next_record(text, pos, sep, &mut rec);
            if rec.len() == 1 && rec[0].is_empty() {
                continue;
            }
            if first {
                f.columns = mangle(rec.iter().map(|c| c.to_string()).collect());
                width = f.columns.len();
                first = false;
                continue;
            }
            if rec.len() > width {
                return Err(CampaignError::value(format!(
                    "Error tokenizing data. C error: Expected {width} fields in line {line}, saw {}",
                    rec.len()
                )));
            }
            for k in 0..width {
                let v = rec.get(k).map_or("", |c| c.as_ref());
                let start = f.data.len();
                f.data.push_str(v);
                f.cells.push((start as u32, v.len() as u32));
            }
            f.nrows += 1;
            if f.data.len() > u32::MAX as usize - 1 {
                return Err(CampaignError::value("table too large"));
            }
        }
        f.index = index_of(&f.columns);
        Ok(f)
    }
    /// Only the header of a delimited text (`pd.read_csv(nrows=0)`).
    pub fn header(text: &str, sep: char) -> Vec<String> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut rec = Vec::new();
        let mut pos = 0;
        while pos < text.len() {
            pos = next_record(text, pos, sep, &mut rec);
            if !(rec.len() == 1 && rec[0].is_empty()) {
                return mangle(rec.iter().map(|c| c.to_string()).collect());
            }
        }
        Vec::new()
    }
}

/// A value of a result row, for the tables the page shows and for the comparisons with the golden data.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Null,
    Bool(bool),
    Int(i64),
    Num(f64),
    Str(String),
}

impl Cell {
    pub fn str(v: &Option<String>) -> Cell {
        v.as_ref().map_or(Cell::Null, |s| Cell::Str(s.clone()))
    }
    pub fn num(v: Option<f64>) -> Cell {
        v.filter(|x| x.is_finite()).map_or(Cell::Null, Cell::Num)
    }
}

/// A typed row of a source table: its column names (those of Attributio) and its values in that order.
pub trait Record {
    const COLUMNS: &'static [&'static str];
    fn cells(&self) -> Vec<Cell>;
}

/// The words pandas reads as missing in every column.
pub fn is_na(s: &str) -> bool {
    matches!(
        s,
        "" | "#N/A"
            | "#N/A N/A"
            | "#NA"
            | "-1.#IND"
            | "-1.#QNAN"
            | "-NaN"
            | "-nan"
            | "1.#IND"
            | "1.#QNAN"
            | "<NA>"
            | "N/A"
            | "NA"
            | "NULL"
            | "NaN"
            | "None"
            | "n/a"
            | "nan"
            | "null"
    )
}

/// `pd.to_numeric(s, errors="coerce")` of one cell: `None` for a missing or non-numeric text and for NaN.
pub fn to_num(s: &str) -> Option<f64> {
    let t = s.trim();
    if is_na(t) || !t.is_ascii() {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| !v.is_nan())
}

#[derive(Clone, Debug, Default)]
pub struct Frame {
    columns: Vec<String>,
    index: HashMap<String, usize>,
    data: String,
    cells: Vec<(u32, u32)>,
    nrows: usize,
}

fn index_of(columns: &[String]) -> HashMap<String, usize> {
    let mut m = HashMap::new();
    for (i, c) in columns.iter().enumerate() {
        m.entry(c.clone()).or_insert(i);
    }
    m
}

fn mangle(cols: Vec<String>) -> Vec<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut out = Vec::with_capacity(cols.len());
    for c in cols {
        let n = seen.entry(c.clone()).or_insert(0);
        out.push(if *n == 0 {
            c.clone()
        } else {
            format!("{c}.{n}")
        });
        *n += 1;
    }
    out
}

/// One record starting at `pos`; returns the position after its line end.
fn next_record<'t>(
    text: &'t str,
    mut pos: usize,
    sep: char,
    rec: &mut Vec<std::borrow::Cow<'t, str>>,
) -> usize {
    rec.clear();
    let b = text.as_bytes();
    loop {
        if b.get(pos) == Some(&b'"') {
            let (field, next) = quoted(text, pos + 1);
            rec.push(field.into());
            pos = next;
        } else {
            let end = text[pos..]
                .find([sep, '\n', '\r'])
                .map_or(text.len(), |i| pos + i);
            rec.push(text[pos..end].into());
            pos = end;
        }
        match text[pos..].chars().next() {
            Some(c) if c == sep => pos += c.len_utf8(),
            Some('\r') => {
                pos += 1;
                if b.get(pos) == Some(&b'\n') {
                    pos += 1;
                }
                return pos;
            }
            Some('\n') => return pos + 1,
            _ => return pos,
        }
    }
}

/// The text of a quoted field (from after the opening quote) and the position after the closing quote and any stray text up to the delimiter.
fn quoted(text: &str, mut pos: usize) -> (String, usize) {
    let mut s = String::new();
    loop {
        match text[pos..].find('"') {
            None => {
                s.push_str(&text[pos..]);
                return (s, text.len());
            }
            Some(i) => {
                s.push_str(&text[pos..pos + i]);
                pos += i + 1;
                if text.as_bytes().get(pos) == Some(&b'"') {
                    s.push('"');
                    pos += 1;
                } else {
                    return (s, pos);
                }
            }
        }
    }
}

/// A field as `DataFrame.to_csv` writes it (minimal quoting).
pub fn csv_field(v: &str, sep: char) -> String {
    if v.contains(sep) || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

/// Rows as a delimited text with a header and «\n» line ends.
pub fn to_csv(columns: &[String], rows: &[Vec<String>], sep: char) -> String {
    let line = |cells: &[String]| {
        cells
            .iter()
            .map(|c| csv_field(c, sep))
            .collect::<Vec<_>>()
            .join(&sep.to_string())
    };
    let mut out = line(columns);
    out.push('\n');
    for r in rows {
        out.push_str(&line(r));
        out.push('\n');
    }
    out
}
