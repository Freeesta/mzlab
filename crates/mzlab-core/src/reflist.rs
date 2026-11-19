//! Reference lists of known m/z values (`reflist/1`): the CSV a person adds as a list in the Known contaminants panel, read here for the page,
//! the app and `mzlab-cli db ref-list`. A good row is an ion (m/z, adduct, polarity); a wrong row gives an error with its line number.

use crate::elements::{adducts, ion_mz, mass, parse_formula};
use crate::error::{Error, Result};
use crate::json::{parse, Value};
use crate::jsonb::{int, num, text, write, Obj};

const HEAD: [(&str, &[&str]); 6] = [
    (
        "mz",
        &[
            "mz",
            "m/z",
            "ionmz",
            "monoisotopicmass",
            "exactmass",
            "mass",
        ],
    ),
    (
        "name",
        &[
            "name",
            "compound",
            "compoundname",
            "preferredname",
            "substance",
            "title",
        ],
    ),
    ("formula", &["formula", "molecularformula", "molformula"]),
    (
        "polarity",
        &["polarity", "ionmode", "ionizationmode", "mode"],
    ),
    ("adduct", &["adduct", "ion"]),
    (
        "note",
        &[
            "note",
            "comment",
            "source",
            "cas",
            "casrn",
            "description",
            "id",
        ],
    ),
];

fn split(line: &str, sep: char) -> Vec<String> {
    let (mut out, mut cur, mut q) = (Vec::new(), String::new(), false);
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        match (q, c) {
            (true, '"') if it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            (true, '"') => q = false,
            (false, '"') => q = true,
            (false, c) if c == sep => out.push(std::mem::take(&mut cur).trim().to_string()),
            (_, c) => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// `;`, tab or `,`: the most frequent outside quotes in the header line.
fn separator(line: &str) -> char {
    let (mut best, mut n_best) = (',', 0);
    for s in [';', '\t', ','] {
        let (mut n, mut q) = (0, false);
        for c in line.chars() {
            match c {
                '"' => q = !q,
                c if c == s && !q => n += 1,
                _ => {}
            }
        }
        if n > n_best {
            (best, n_best) = (s, n);
        }
    }
    best
}

fn column(head: &[String], names: &[&str]) -> Option<usize> {
    let clean = |s: &str| {
        s.to_lowercase()
            .chars()
            .filter(|c| !matches!(c, ' ' | '_' | '-'))
            .collect::<String>()
    };
    head.iter().position(|h| names.contains(&clean(h).as_str()))
}

fn polarity(s: &str) -> Option<i64> {
    match s.to_lowercase().as_str() {
        "" | "0" | "both" => Some(0),
        "+" | "1" | "pos" | "positive" => Some(1),
        "-" | "-1" | "neg" | "negative" => Some(-1),
        _ => None,
    }
}

fn error(line: usize, key: &str, field: &str, value: &str) -> Value {
    Obj::new()
        .i("line", line as i64)
        .put("key", text(key))
        .put("field", text(field))
        .put("value", text(value))
        .done()
}

/// `{"csv": TEXT, "name": optional}` -> `reflist/1`: `{schema, name, compounds, items:[{line,name,formula,adduct,mz,pol,note}], errors:[{line,key,field,value}]}`.
/// A row needs an m/z or a formula; with a formula only, the ion is `[M+H]+` (negative polarity: `[M-H]-`) unless the adduct column says another.
pub fn route(body: &str) -> Result<String> {
    let req = parse(body.as_bytes()).map_err(|_| Error::new("err.reflist.request"))?;
    let csv = req
        .get("csv")
        .and_then(Value::str)
        .ok_or_else(|| Error::new("err.reflist.request"))?;
    let name = req.get("name").and_then(Value::str).unwrap_or("");
    let (mut items, mut errors) = (Vec::new(), Vec::new());
    let mut rows = csv
        .trim_start_matches('\u{feff}')
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_end_matches('\r')))
        .filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'));
    if let Some((hl, h)) = rows.next() {
        let sep = separator(h);
        let head = split(h, sep);
        let col: Vec<Option<usize>> = HEAD.iter().map(|(_, n)| column(&head, n)).collect();
        if col[0].is_none() && col[2].is_none() {
            errors.push(error(hl, "err.reflist.header", "header", h));
        } else {
            for (ln, l) in rows {
                let f = split(l, sep);
                let get = |k: usize| col[k].and_then(|i| f.get(i)).map_or("", String::as_str);
                match item(ln, get) {
                    Ok(v) => items.push(v),
                    Err(e) => errors.push(e),
                }
            }
        }
    } else {
        errors.push(error(1, "err.reflist.empty", "file", ""));
    }
    let compounds = items
        .iter()
        .filter_map(|v| v.get("name").and_then(Value::str))
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let o = Obj::new()
        .put("schema", text("reflist/1"))
        .put("name", text(name))
        .i("compounds", compounds as i64)
        .put("items", Value::Arr(items))
        .put("errors", Value::Arr(errors))
        .done();
    Ok(write(&o))
}

fn item<'a>(line: usize, get: impl Fn(usize) -> &'a str) -> std::result::Result<Value, Value> {
    let (mz_t, name, formula, pol_t, adduct, note) =
        (get(0), get(1), get(2), get(3), get(4), get(5));
    let Some(pol) = polarity(pol_t) else {
        return Err(error(line, "err.reflist.polarity", "polarity", pol_t));
    };
    if !adduct.is_empty() && !adducts().iter().any(|(a, _)| *a == adduct) {
        return Err(error(line, "err.reflist.adduct", "adduct", adduct));
    }
    let given = if mz_t.is_empty() {
        None
    } else {
        match mz_t.replace(',', ".").parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 => Some(v),
            _ => return Err(error(line, "err.reflist.mz", "mz", mz_t)),
        }
    };
    let neutral = if formula.is_empty() {
        None
    } else {
        match parse_formula(formula) {
            Ok(f) => Some(mass(&f)),
            Err(_) => return Err(error(line, "err.reflist.formula", "formula", formula)),
        }
    };
    let (mz, ion) = match (given, neutral) {
        (Some(m), _) => (m, adduct),
        (None, Some(n)) => {
            let a = if !adduct.is_empty() {
                adduct
            } else if pol < 0 {
                "[M-H]-"
            } else {
                "[M+H]+"
            };
            (ion_mz(n, a).unwrap_or(n), a)
        }
        (None, None) => return Err(error(line, "err.reflist.noMass", "mz", "")),
    };
    // the polarity the row did not give is the one of its adduct
    let pol = if pol != 0 {
        pol
    } else if ion.ends_with('-') {
        -1
    } else if ion.ends_with('+') {
        1
    } else {
        0
    };
    let shown = if !name.is_empty() {
        name.to_string()
    } else if !formula.is_empty() {
        formula.to_string()
    } else {
        format!("m/z {mz_t}")
    };
    Ok(Obj::new()
        .i("line", line as i64)
        .put("name", text(&shown))
        .put("formula", text(formula))
        .put("adduct", text(ion))
        .put("mz", num((mz * 1e5).round() / 1e5))
        .put("pol", int(pol))
        .put("note", text(note))
        .done())
}

pub mod lookup;

#[cfg(test)]
mod tests;
