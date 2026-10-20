//! The CSV of a list: the delimiter is `;` (or `,` / tab when the header has no `;`), `#` lines and empty lines are skipped, a field may be in double quotes.
//! The header names the columns (`name`, `delta`, `max_repeat`, `charge`, `formula`, `mass`, `comment`, in any order); a transformation list may have no header
//! (`name;change[;comment]`, the text of mzFinder). Every wrong row gives its own error and the list is refused if there is one.

use mzlab_core::elements::parse_formula;

use super::{Adduct, Kind, Liste, RowError, Suspect, Transformation, MAX_REPEAT};
use crate::legacy::transf::{parse_delta, Delta};

fn err(line: usize, key: &'static str, detail: impl Into<String>) -> RowError {
    RowError {
        line,
        key,
        detail: detail.into(),
    }
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
            // a quote opens a field only at its start, and closes it
            '"' if quoted || cur.is_empty() => quoted = !quoted,
            c if c == d && !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn delimiter(header: &str) -> char {
    if header.contains(';') {
        ';'
    } else if header.contains('\t') {
        '\t'
    } else if header.contains(',') {
        ','
    } else {
        ';'
    }
}

fn columns(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Transformations => &["name", "delta", "max_repeat", "comment"],
        Kind::Adducts => &["name", "delta", "charge", "comment"],
        Kind::Suspects => &["name", "formula", "mass", "comment"],
    }
}

fn delta_of(line: usize, text: &str) -> Result<Delta, RowError> {
    let d = parse_delta(text).map_err(|e| err(line, "error.liste.badDelta", e))?;
    if d.is_empty() {
        return Err(err(line, "error.liste.noDelta", text));
    }
    Ok(d)
}

fn whole(line: usize, text: &str, key: &'static str) -> Result<Option<i64>, RowError> {
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<i64>()
        .map(Some)
        .map_err(|_| err(line, key, text))
}

/// The cells of one row by column name (empty when the column is absent).
struct Cells<'a>(&'a [&'static str], Vec<String>);

impl Cells<'_> {
    fn get(&self, name: &str) -> &str {
        self.0
            .iter()
            .position(|c| *c == name)
            .and_then(|i| self.1.get(i))
            .map_or("", String::as_str)
    }
}

fn row(kind: Kind, line: usize, cells: &Cells) -> Result<Liste, RowError> {
    let name = cells.get("name").to_string();
    if name.is_empty() {
        return Err(err(line, "error.liste.noName", ""));
    }
    let comment = cells.get("comment").to_string();
    Ok(match kind {
        Kind::Transformations => {
            let max = whole(line, cells.get("max_repeat"), "error.liste.badRepeat")?;
            if max.is_some_and(|m| !(1..=i64::from(MAX_REPEAT)).contains(&m)) {
                return Err(err(line, "error.liste.badRepeat", cells.get("max_repeat")));
            }
            Liste::Transformations(vec![Transformation {
                name,
                delta: delta_of(line, cells.get("delta"))?,
                max_repeat: max.map(|m| m as u32),
                comment,
            }])
        }
        Kind::Adducts => {
            let charge = whole(line, cells.get("charge"), "error.liste.badCharge")?.unwrap_or(1);
            if charge == 0 || charge.abs() > 3 {
                return Err(err(line, "error.liste.badCharge", cells.get("charge")));
            }
            Liste::Adducts(vec![Adduct {
                name,
                delta: delta_of(line, cells.get("delta"))?,
                charge: charge as i8,
                comment,
            }])
        }
        Kind::Suspects => {
            let (formula, mass) = (cells.get("formula"), cells.get("mass"));
            if !formula.is_empty() {
                parse_formula(formula).map_err(|e| err(line, "error.liste.badFormula", e.key))?;
            }
            let mass = match mass {
                "" => None,
                m => Some(
                    m.parse::<f64>()
                        .ok()
                        .filter(|x| x.is_finite() && *x > 0.0)
                        .ok_or_else(|| err(line, "error.liste.badMass", m))?,
                ),
            };
            if formula.is_empty() && mass.is_none() {
                return Err(err(line, "error.liste.noMass", ""));
            }
            Liste::Suspects(vec![Suspect {
                name,
                formula: (!formula.is_empty()).then(|| formula.to_string()),
                mass,
                comment,
            }])
        }
    })
}

fn push(into: &mut Liste, one: Liste) {
    match (into, one) {
        (Liste::Transformations(a), Liste::Transformations(mut b)) => a.append(&mut b),
        (Liste::Adducts(a), Liste::Adducts(mut b)) => a.append(&mut b),
        (Liste::Suspects(a), Liste::Suspects(mut b)) => a.append(&mut b),
        _ => unreachable!("one row of the same kind"),
    }
}

fn empty(kind: Kind) -> Liste {
    match kind {
        Kind::Transformations => Liste::Transformations(Vec::new()),
        Kind::Adducts => Liste::Adducts(Vec::new()),
        Kind::Suspects => Liste::Suspects(Vec::new()),
    }
}

/// The list of `kind` in `text`, or an error for every wrong row (line numbers are those of the text, from 1).
pub fn from_csv(kind: Kind, text: &str) -> Result<Liste, Vec<RowError>> {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let (mut list, mut errors) = (empty(kind), Vec::new());
    let Some(&(first_line, first)) = lines.first() else {
        return Ok(list);
    };
    let d = delimiter(first);
    let head: Vec<String> = split(first, d).iter().map(|c| c.to_lowercase()).collect();
    let has_header = head.first().is_some_and(|c| c == "name");
    let (cols, body): (Vec<&'static str>, &[(usize, &str)]) = if has_header {
        let named: Vec<&'static str> = head
            .iter()
            .filter_map(|h| columns(kind).iter().find(|c| *c == h).copied())
            .collect();
        if named.len() != head.len() {
            let unknown = head
                .iter()
                .find(|h| !columns(kind).contains(&h.as_str()))
                .cloned()
                .unwrap_or_default();
            return Err(vec![err(first_line, "error.liste.badColumn", unknown)]);
        }
        (named, &lines[1..])
    } else if kind == Kind::Transformations {
        (vec!["name", "delta", "comment"], &lines[..])
    } else {
        return Err(vec![err(
            first_line,
            "error.liste.noHeader",
            columns(kind).join(";"),
        )]);
    };
    for &(line, text) in body {
        match row(kind, line, &Cells(&cols, split(text, d))) {
            Ok(one) => push(&mut list, one),
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        Ok(list)
    } else {
        Err(errors)
    }
}
