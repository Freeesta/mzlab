//! One series table: the CSV that `mzlab-cli features` writes (`id, mz, rt, rt_range:min, rt_range:max, ...`; the sample columns are not read) or any
//! CSV with those names. A wrong row refuses the table with its line (the header is line 1) and the reason.

use super::params::Params;
use crate::error::{Error, Result};

pub struct Row {
    pub id: String,
    pub mz: f64,
    pub rt: f64,
    /// The RT window the series itself gave, min.
    pub window: Option<(f64, f64)>,
    pub polarity: Option<i8>,
    pub line: usize,
}

pub struct Series {
    pub name: String,
    pub rows: Vec<Row>,
    /// Rows whose window was missing or empty and became `rt ± rt_tol`.
    pub windowless: usize,
    /// Rows that took the polarity of the parameter.
    pub polarity_set: usize,
}

fn bad(table: &str, line: usize, detail: &str) -> Error {
    Error::new("err.union.table")
        .with("table", table)
        .with("row", line.to_string())
        .with("detail", detail)
}

/// The fields of a line; a field in double quotes may hold the delimiter, `""` is a quote.
fn split(line: &str, d: char) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if quoted && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' if quoted || cur.is_empty() => quoted = !quoted,
            c if c == d && !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// The delimiter that cuts the header into most fields (`,` on a tie).
fn delimiter(header: &str) -> char {
    [',', ';', '\t']
        .into_iter()
        .max_by_key(|d| (split(header, *d).len(), *d == ','))
        .unwrap_or(',')
}

fn polarity(t: &str) -> Option<Option<i8>> {
    match t.trim().to_lowercase().as_str() {
        "" => Some(None),
        "+" | "pos" | "positive" | "1" | "+1" => Some(Some(1)),
        "-" | "neg" | "negative" | "-1" => Some(Some(-1)),
        _ => None,
    }
}

fn number(t: &str) -> Option<f64> {
    t.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

pub fn read(name: &str, text: &str, p: &Params) -> Result<Series> {
    let text = text.trim_start_matches('\u{feff}');
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_end_matches('\r')))
        .filter(|(_, l)| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
    let Some((hl, header)) = lines.next() else {
        return Err(bad(name, 1, "empty"));
    };
    let d = delimiter(header);
    let names: Vec<String> = split(header, d).iter().map(|h| h.to_lowercase()).collect();
    let col = |k: &[&str]| names.iter().position(|n| k.contains(&n.as_str()));
    let need = |k: &[&str]| col(k).ok_or_else(|| bad(name, hl, &format!("no_column_{}", k[0])));
    let (mz_at, rt_at) = (
        need(&["mz", "m/z", "row m/z"])?,
        need(&["rt", "row retention time", "retention time"])?,
    );
    let (id_at, lo_at, hi_at, pol_at) = (
        col(&["id", "name", "row id"]),
        col(&["rt_range:min", "rt_min"]),
        col(&["rt_range:max", "rt_max"]),
        col(&["polarity"]),
    );
    let mut s = Series {
        name: name.to_string(),
        rows: Vec::new(),
        windowless: 0,
        polarity_set: 0,
    };
    for (line, l) in lines {
        let cells = split(l, d);
        let get = |i: Option<usize>| i.and_then(|i| cells.get(i)).map_or("", |c| c.as_str());
        let mz = number(get(Some(mz_at)))
            .filter(|v| *v > 0.0)
            .ok_or_else(|| bad(name, line, "bad_mz"))?;
        let rt = number(get(Some(rt_at)))
            .filter(|v| *v >= 0.0)
            .ok_or_else(|| bad(name, line, "bad_rt"))?;
        let window = match (number(get(lo_at)), number(get(hi_at))) {
            (Some(a), Some(b)) if a >= 0.0 && b > a => Some((a, b)),
            _ => None,
        };
        let own = polarity(get(pol_at)).ok_or_else(|| bad(name, line, "bad_polarity"))?;
        let pol = own.or(p.polarity);
        s.polarity_set += usize::from(own.is_none() && pol.is_some());
        s.windowless += usize::from(window.is_none());
        let id = get(id_at);
        let id = if id.is_empty() {
            line.to_string()
        } else {
            id.to_string()
        };
        s.rows.push(Row {
            id,
            mz,
            rt,
            window,
            polarity: pol,
            line,
        });
    }
    Ok(s)
}
