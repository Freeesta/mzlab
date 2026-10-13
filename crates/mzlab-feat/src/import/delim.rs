//! Delimited text (streaming over a string) and the tolerant reading of cells: the rules of `eachRow`, `num`, `txt` and `colMap` of the table readers.

use super::ImportError;

/// A JavaScript `trim`: the white space of the language plus the byte order mark.
pub fn jstrim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// A JavaScript `Number(s)` for the strings the tables hold: decimal (with exponent), `0x`/`0o`/`0b` and the empty string (0); `None` is NaN.
pub fn js_number(s: &str) -> Option<f64> {
    let t = jstrim(s);
    if t.is_empty() {
        return Some(0.0);
    }
    let radix = match t.get(..2) {
        Some("0x") | Some("0X") => 16,
        Some("0o") | Some("0O") => 8,
        Some("0b") | Some("0B") => 2,
        _ => 0,
    };
    if radix > 0 {
        return u64::from_str_radix(&t[2..], radix).ok().map(|v| v as f64);
    }
    let b = t.as_bytes();
    let ok = b
        .iter()
        .all(|c| c.is_ascii_digit() || matches!(c, b'+' | b'-' | b'.' | b'e' | b'E'));
    if t == "Infinity" || t == "+Infinity" || t == "-Infinity" {
        return t
            .starts_with('-')
            .then_some(f64::NEG_INFINITY)
            .or(Some(f64::INFINITY));
    }
    if !ok {
        return None;
    }
    t.parse::<f64>().ok()
}

/// `num`: a finite number from a cell, with «na», «nan», «null», «none», «-» and the empty cell as missing; a decimal comma is accepted.
pub fn num(v: &str) -> Option<f64> {
    let s = jstrim(v);
    if s.is_empty()
        || ["na", "nan", "null", "none", "-"]
            .iter()
            .any(|k| s.eq_ignore_ascii_case(k))
    {
        return None;
    }
    js_number(&s.replacen(',', ".", 1)).filter(|x| x.is_finite())
}

/// `txt`: the trimmed text of a cell.
pub fn txt(v: &str) -> &str {
    jstrim(v)
}

/// The cell of column `i` (empty when the column is missing or the row is short).
pub fn cell(r: &[String], i: Option<usize>) -> &str {
    i.and_then(|i| r.get(i)).map_or("", |s| s.as_str())
}

/// Position of each column name (lower case, first one wins).
pub struct Cols(Vec<(String, usize)>);

impl Cols {
    pub fn new(hdr: &[String]) -> Cols {
        let mut v: Vec<(String, usize)> = Vec::new();
        for (i, h) in hdr.iter().enumerate() {
            let k = h.to_lowercase();
            if !v.iter().any(|(x, _)| *x == k) {
                v.push((k, i));
            }
        }
        Cols(v)
    }
    /// The first of `names` (in order of preference, exact, case-insensitive) that the header has.
    pub fn pick(&self, names: &[&str]) -> Option<usize> {
        names.iter().find_map(|n| {
            let k = n.to_lowercase();
            self.0.iter().find(|(x, _)| *x == k).map(|(_, i)| *i)
        })
    }
}

/// `splitQuoted`: one line with quoted fields (a doubled quote is a quote). After a closing quote anything but the delimiter ends the row.
pub fn split_quoted(line: &str, d: u8) -> Vec<String> {
    let b = line.as_bytes();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i <= n {
        if b.get(i) == Some(&b'"') {
            let mut j = i + 1;
            let mut s = String::new();
            loop {
                match line[j..].find('"').map(|k| k + j) {
                    None => {
                        s.push_str(&line[j..]);
                        j = n;
                        break;
                    }
                    Some(k) if b.get(k + 1) == Some(&b'"') => {
                        s.push_str(&line[j..=k]);
                        j = k + 2;
                    }
                    Some(k) => {
                        s.push_str(&line[j..k]);
                        j = k + 1;
                        break;
                    }
                }
            }
            out.push(s);
            i = if b.get(j) == Some(&d) { j + 1 } else { n + 1 };
        } else {
            match line[i..].find(d as char) {
                None => {
                    out.push(line[i..].to_string());
                    break;
                }
                Some(k) => {
                    out.push(line[i..i + k].to_string());
                    i += k + 1;
                }
            }
        }
    }
    out
}

/// A test of the cells of a candidate header row.
pub type HeaderTest<'a> = Option<&'a dyn Fn(&[String]) -> bool>;

/// `eachRow`: `on_header(names)` runs for the first row (or the first one `is_header` accepts) and gives the state of the reader (the columns it found);
/// `on_row(state, cells, row number)` runs for each following one. Quoted newlines are joined, empty lines after the header are skipped.
/// Returns the state, or `None` when the text has no header.
pub fn each_row<S>(
    text: &str,
    d: u8,
    is_header: HeaderTest,
    on_header: impl FnOnce(&[String]) -> Result<S, ImportError>,
    mut on_row: impl FnMut(&mut S, &[String], usize) -> Result<(), ImportError>,
) -> Result<Option<S>, ImportError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.is_empty() {
        return Ok(None);
    }
    let (mut state, mut on_header): (Option<S>, Option<_>) = (None, Some(on_header));
    let (mut no, mut pend) = (0, String::new());
    for raw in text.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let line: std::borrow::Cow<str> = if pend.is_empty() {
            raw.into()
        } else {
            format!("{}\n{}", std::mem::take(&mut pend), raw).into()
        };
        if line.matches('"').count() % 2 == 1 {
            pend = line.into_owned();
            continue;
        }
        if line.is_empty() && state.is_some() {
            continue;
        }
        let cells: Vec<String> = if line.contains('"') {
            split_quoted(&line, d)
        } else {
            line.split(d as char).map(String::from).collect()
        };
        match state.as_mut() {
            None => {
                if is_header.is_some_and(|f| !f(&cells)) {
                    continue;
                }
                let h: Vec<String> = cells.iter().map(|c| jstrim(c).to_string()).collect();
                state = Some((on_header.take().expect("one header"))(&h)?);
            }
            Some(s) => {
                no += 1;
                on_row(s, &cells, no)?;
            }
        }
    }
    Ok(state)
}
