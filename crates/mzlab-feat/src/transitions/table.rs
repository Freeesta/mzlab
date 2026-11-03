//! The CSV of the transition table: a header naming the columns (any order, any subset of `HEADER` with `name`), `;` (or tab / `,`) between fields, `#` and
//! empty lines skipped, a decimal comma read as a point when the delimiter is not a comma. Q1 is written, or worked out from `formula` + `adduct` with the
//! masses of `mzlab_core::elements` (never typed as a number in the code). Every wrong row gives its own `error.liste.*` error and the table is refused.

use mzlab_core::elements::{adducts, ion_mz, mass, parse_formula};

use crate::liste::csv::{delimiter, split};
use crate::liste::RowError;

/// The columns of the table, in the order `to_csv` writes them.
pub const HEADER: [&str; 10] = [
    "name",
    "q1",
    "q3",
    "polarity",
    "rt",
    "rt_window",
    "isomer_group",
    "note",
    "formula",
    "adduct",
];

/// One row: a SIM ion (`q3` none) or an SRM transition, with its expected RT (none = not known yet) and half window, both in minutes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    pub name: String,
    /// The m/z of Q1: the one written, else the one of `formula` + `adduct`.
    pub q1: f64,
    pub q3: Option<f64>,
    /// 1 or -1.
    pub polarity: i8,
    pub rt: Option<f64>,
    pub rt_window: Option<f64>,
    pub isomer_group: String,
    pub note: String,
    pub formula: String,
    pub adduct: String,
    /// The cells as written, in the order of `HEADER`: what `to_csv` writes back.
    pub cells: [String; 10],
}

fn err(line: usize, key: &'static str, detail: &str) -> RowError {
    RowError {
        line,
        key,
        detail: detail.to_string(),
    }
}

/// A number of the table: a decimal comma is a point unless the comma separates the fields.
fn number(text: &str, comma_is_point: bool) -> Option<f64> {
    let t = if comma_is_point {
        text.replace(',', ".")
    } else {
        text.to_string()
    };
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn polarity_of(text: &str, adduct: &str) -> Option<i8> {
    match text.to_ascii_lowercase().as_str() {
        "" if adduct.ends_with('-') => Some(-1),
        "" | "+" | "+1" | "1" | "pos" | "positive" => Some(1),
        "-" | "-1" | "neg" | "negative" => Some(-1),
        _ => None,
    }
}

fn row(line: usize, c: &[String; 10], comma_is_point: bool) -> Result<Row, RowError> {
    let get = |name: &str| {
        HEADER
            .iter()
            .position(|h| *h == name)
            .map_or("", |i| c[i].as_str())
    };
    let name = get("name");
    if name.is_empty() {
        return Err(err(line, "error.liste.noName", ""));
    }
    let adduct_text = get("adduct");
    let polarity = polarity_of(get("polarity"), adduct_text)
        .ok_or_else(|| err(line, "error.liste.badPolarity", get("polarity")))?;
    let adduct = match adduct_text {
        "" if polarity > 0 => "[M+H]+",
        "" => "[M-H]-",
        a => a,
    };
    let known = adducts().iter().any(|(a, _)| *a == adduct);
    if !known || adduct.ends_with('-') != (polarity < 0) {
        return Err(err(line, "error.liste.badAdduct", adduct));
    }
    let positive = |key: &'static str, col: &str| -> Result<Option<f64>, RowError> {
        let t = get(col);
        if t.is_empty() {
            return Ok(None);
        }
        match number(t, comma_is_point) {
            Some(v) if v > 0.0 || (col == "rt" && v == 0.0) => Ok(Some(v)),
            _ => Err(err(line, key, t)),
        }
    };
    let formula = get("formula");
    let q1 = match positive("error.liste.badMz", "q1")? {
        Some(v) => v,
        None if formula.is_empty() => return Err(err(line, "error.liste.noMz", name)),
        None => {
            let f =
                parse_formula(formula).map_err(|_| err(line, "error.liste.badFormula", formula))?;
            ion_mz(mass(&f), adduct).ok_or_else(|| err(line, "error.liste.badAdduct", adduct))?
        }
    };
    Ok(Row {
        name: name.to_string(),
        q1,
        q3: positive("error.liste.badMz", "q3")?,
        polarity,
        rt: positive("error.liste.badRt", "rt")?,
        rt_window: positive("error.liste.badRt", "rt_window")?,
        isomer_group: get("isomer_group").to_string(),
        note: get("note").to_string(),
        formula: formula.to_string(),
        adduct: adduct.to_string(),
        cells: c.clone(),
    })
}

/// The rows of the table in `text`, in its order, or an error for every wrong row (lines of the text, from 1).
pub fn parse(text: &str) -> Result<Vec<Row>, Vec<RowError>> {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let Some(&(first_line, first)) = lines.first() else {
        return Ok(Vec::new());
    };
    let d = delimiter(first);
    let head: Vec<String> = split(first, d).iter().map(|c| c.to_lowercase()).collect();
    if head.first().map(String::as_str) != Some("name") {
        return Err(vec![err(
            first_line,
            "error.liste.noHeader",
            &HEADER.join(";"),
        )]);
    }
    let mut at = Vec::new();
    for h in &head {
        match HEADER.iter().position(|c| c == h) {
            Some(i) => at.push(i),
            None => return Err(vec![err(first_line, "error.liste.badColumn", h)]),
        }
    }
    let (mut rows, mut errors): (Vec<Row>, Vec<RowError>) = (Vec::new(), Vec::new());
    for &(line, l) in &lines[1..] {
        let mut cells: [String; 10] = Default::default();
        for (k, v) in split(l, d).into_iter().enumerate() {
            if let Some(&i) = at.get(k) {
                cells[i] = v;
            }
        }
        match row(line, &cells, d != ',') {
            Ok(r) if rows.iter().any(|o: &Row| o.name == r.name) => {
                errors.push(err(line, "error.liste.twice", &r.name));
            }
            Ok(r) => rows.push(r),
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        Ok(rows)
    } else {
        Err(errors)
    }
}

fn quote(s: &str) -> String {
    if s.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The table as the CSV the plan keeps: every column of `HEADER`, the cells as written.
pub fn to_csv(rows: &[Row]) -> String {
    let mut out = HEADER.join(";") + "\n";
    for r in rows {
        let line: Vec<String> = r.cells.iter().map(|c| quote(c)).collect();
        out += &(line.join(";") + "\n");
    }
    out
}
