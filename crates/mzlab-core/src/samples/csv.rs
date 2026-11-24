//! The metadata of `samples/1` as a CSV table (`,` `;` or tab): one row per file, the header the names of the fields (`file`, `sha256`, `sample_id`,
//! `polarity`, ..., then one column per free column); `condition`, `replicate`, `injection_order`, `role` (`control` = dark, `standard` = std),
//! `time` (minutes), `dose` (`25 ppm`) and the like are read too, and an unknown column becomes a free column (a number when every value is one).
//! What `csv_write` writes `csv_read` reads back unchanged (`time_min` is derived, `file_data` is not in the table).

use std::borrow::Cow;

use super::columns::infer;
use super::csv_fields::{records, Cells};
use super::json::role_alias;
use super::{
    number_replicates, table_from_value, table_to_json, Cell, Column, ColumnType, Sample, Table,
};
use crate::error::{Error, Result};
use crate::json::{parse, Out};

/// The fixed columns, in the order written.
const FIXED: [&str; 23] = [
    "file",
    "sub_sample",
    "sha256",
    "sample_id",
    "polarity",
    "matrix",
    "batch",
    "acquired",
    "group",
    "kind",
    "blank_kind",
    "time",
    "time_unit",
    "time_confirmed",
    "series",
    "reinjection",
    "replicate",
    "concentration",
    "concentration_unit",
    "injection_order",
    "istd",
    "note",
    "compound",
];

pub(super) fn bad(row: usize, col: &str) -> Error {
    Error::new("error.badCsv")
        .with("row", row)
        .with("column", col)
}

/// `v.to_lowercase()`, without a copy when `v` is already lower-case ASCII (the usual cell).
fn lower(v: &str) -> Cow<'_, str> {
    if v.bytes().any(|b| b.is_ascii_uppercase() || !b.is_ascii()) {
        Cow::Owned(v.to_lowercase())
    } else {
        Cow::Borrowed(v)
    }
}

/// The field a header names (the fixed name, or a synonym), None for a free column.
pub(super) fn field_of(h: &str) -> Option<&'static str> {
    let l = h.trim().to_lowercase();
    if let Some(f) = FIXED.iter().find(|f| **f == l) {
        return Some(f);
    }
    Some(match l.as_str() {
        "name" | "filename" | "file_name" | "file name" => "file",
        "sample_id" | "sample" | "sample_name" | "sample name" => "sample_id",
        "polarity" => "polarity",
        "matrix" => "matrix",
        "batch" => "batch",
        "acquired" => "acquired",
        "group" => "group",
        "role" | "type" => "kind",
        "condition" => "series",
        "replicate" => "replicate",
        "injection_order" => "injection_order",
        "time_min" | "time" => "time_min",
        "dose" => "dose",
        _ => return None,
    })
}

/// The separator of the first line: the one of `,` `;` tab used most outside quotes (`,` when none).
pub(super) fn detect(text: &str) -> char {
    let mut n = [0usize; 3];
    let mut inq = false;
    for c in text.trim_start_matches('\u{feff}').chars() {
        match c {
            '"' => inq = !inq,
            '\n' | '\r' if !inq => break,
            ',' if !inq => n[0] += 1,
            ';' if !inq => n[1] += 1,
            '\t' if !inq => n[2] += 1,
            _ => {}
        }
    }
    let best = (0..3).max_by_key(|&i| (n[i], 3 - i)).unwrap_or(0);
    if n[best] == 0 {
        ','
    } else {
        [',', ';', '\t'][best]
    }
}

fn num(v: &str) -> Option<f64> {
    v.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite())
}

fn flag(v: &str) -> Option<bool> {
    match lower(v).as_ref() {
        "true" | "1" | "si" | "sì" | "yes" | "y" | "vero" => Some(true),
        "false" | "0" | "no" | "n" | "falso" => Some(false),
        _ => None,
    }
}

fn unit_of(v: &str) -> Option<&'static str> {
    super::TIME_UNITS.iter().copied().find(|u| *u == lower(v))
}

fn whole(v: &str) -> Option<u32> {
    v.parse::<u32>().ok().filter(|&n| n >= 1)
}

/// `25 ppm` or `25ppm` → (25, Some("ppm")).
fn dose(v: &str) -> Option<(f64, Option<String>)> {
    let cut = v
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+' | 'e' | 'E')))
        .unwrap_or(v.len());
    let (n, u) = v.split_at(cut);
    let u = u.trim();
    num(n).map(|x| (x, (!u.is_empty()).then(|| u.to_string())))
}

/// Gives `c` the value of one fixed column; false when the value is not one the column takes.
fn put(c: &mut Sample, f: &str, v: &str) -> bool {
    let pol = |p: &str| match lower(p).as_ref() {
        "1" | "+1" | "+" | "pos" | "positive" => Some(1),
        "-1" | "-" | "neg" | "negative" => Some(-1),
        _ => None,
    };
    match f {
        "file" => c.file = v.into(),
        "sha256" => c.sha256 = v.into(),
        "sub_sample" => c.sub = v.into(),
        "sample_id" => c.sample_id = v.into(),
        "matrix" => c.matrix = v.into(),
        "batch" => c.batch = v.into(),
        "group" => c.group = v.into(),
        "series" => c.series = v.into(),
        "note" => c.note = v.into(),
        "compound" => c.compound = v.into(),
        "acquired" => c.acquired = (!v.is_empty()).then(|| v.into()),
        "concentration_unit" => c.unit = (!v.is_empty()).then(|| v.into()),
        "polarity" => return v.is_empty() || pol(v).map(|p| c.polarity = Some(p)).is_some(),
        "kind" => {
            return v.is_empty() || role_alias(&lower(v)).map(|k| c.kind = k.into()).is_some()
        }
        "blank_kind" => {
            return v.is_empty()
                || super::BLANK_TYPES
                    .contains(&v)
                    .then(|| c.blank_kind = Some(v.into()))
                    .is_some()
        }
        "istd" => return v.is_empty() || flag(v).map(|b| c.istd = b).is_some(),
        "reinjection" => return v.is_empty() || flag(v).map(|b| c.reinjection = b).is_some(),
        "replicate" => return v.is_empty() || whole(v).map(|n| c.replicate = n).is_some(),
        "injection_order" => return v.is_empty() || whole(v).map(|n| c.order = n).is_some(),
        "concentration" => return v.is_empty() || num(v).map(|x| c.conc = Some(x)).is_some(),
        "dose" => {
            return v.is_empty()
                || dose(v)
                    .map(|(x, u)| {
                        c.conc = Some(x);
                        c.unit = u;
                    })
                    .is_some()
        }
        _ => {}
    }
    true
}

/// The positions of the columns of the times in the header.
struct TimeCols {
    unit: Option<usize>,
    time: Option<usize>,
    minutes: Option<usize>,
    confirmed: Option<usize>,
}

/// The times of one row: `time` + `time_unit`, else `time_min`; confirmed as written, else when there is one.
fn time_of<'a>(
    c: &mut Sample,
    row: usize,
    tc: &TimeCols,
    cell: &dyn Fn(usize) -> &'a str,
) -> Result<()> {
    let get = |i: Option<usize>| i.map(cell).filter(|v| !v.is_empty());
    let unit = match get(tc.unit) {
        Some(u) => unit_of(u).ok_or_else(|| bad(row, "time_unit"))?,
        None => "min",
    };
    let (t, u, col) = match (get(tc.time), get(tc.minutes)) {
        (Some(t), _) => (Some(t), unit, "time"),
        (None, Some(t)) => (Some(t), "min", "time_min"),
        _ => (None, unit, "time"),
    };
    let t = t.map(|t| num(t).ok_or_else(|| bad(row, col))).transpose()?;
    c.set_time(t, u);
    if c.kind == "t0" && t.is_none() {
        c.time_min = Some(0.0);
    }
    c.time_confirmed = match get(tc.confirmed) {
        Some(v) => flag(v).ok_or_else(|| bad(row, "time_confirmed"))?,
        None => t.is_some(),
    };
    Ok(())
}

/// Each header: Ok(fixed field) or Err(free column name); an empty or repeated name is an error.
fn heads_of(head: &Cells) -> Result<Vec<std::result::Result<&'static str, String>>> {
    let mut heads = Vec::new();
    for (h, _) in head {
        let f = field_of(h).ok_or_else(|| h.trim().to_string());
        if f.as_ref().err().is_some_and(|n| n.is_empty()) || heads.contains(&f) {
            return Err(bad(1, h));
        }
        heads.push(f);
    }
    Ok(heads)
}

/// Reads `text`; `sep` None = detected; `declared` gives the type of the free columns it names (the others are inferred).
pub fn read(text: &str, sep: Option<char>, declared: &[Column]) -> Result<Table> {
    let recs = records(text, sep.unwrap_or_else(|| detect(text)))?;
    let Some((head, rows)) = recs.split_first() else {
        return Ok(Table {
            samples: Vec::new(),
            columns: declared.to_vec(),
        });
    };
    let heads = heads_of(head)?;
    let mut free: Vec<(String, Vec<Option<&str>>)> = (heads.iter())
        .filter_map(|h| h.as_ref().err())
        .map(|h| (h.clone(), Vec::new()))
        .collect();
    let mut cs = Vec::with_capacity(rows.len());
    let mut written: Vec<Option<u32>> = Vec::with_capacity(rows.len());
    let at = |f: &str| heads.iter().position(|h| h.as_ref().ok() == Some(&f));
    let tc = TimeCols {
        unit: at("time_unit"),
        time: at("time"),
        minutes: at("time_min"),
        confirmed: at("time_confirmed"),
    };
    let replicate = at("replicate");
    // the free column of each header position (the names are distinct, in header order)
    let free_at: Vec<usize> = heads
        .iter()
        .scan(0, |k, h| {
            let i = *k;
            *k += usize::from(h.is_err());
            Some(i)
        })
        .collect();
    for (r, cells) in rows.iter().enumerate() {
        let row = r + 2;
        if cells.len() > head.len() {
            return Err(bad(row, ""));
        }
        let cell = |i: usize| cells.get(i).map(|c| c.0.as_ref()).unwrap_or("");
        let mut c = Sample::new("", "", r as u32 + 1);
        for (i, h) in heads.iter().enumerate() {
            match h {
                Ok(f) if !matches!(*f, "time" | "time_unit" | "time_confirmed" | "time_min") => {
                    if !put(&mut c, f, cell(i)) {
                        return Err(bad(row, f));
                    }
                }
                Err(_) => {
                    let q = cells.get(i).is_some_and(|c| c.1);
                    free[free_at[i]]
                        .1
                        .push((!cell(i).is_empty() || q).then(|| cell(i)));
                }
                _ => {}
            }
        }
        time_of(&mut c, row, &tc, &cell)?;
        written.push(replicate.and_then(|i| whole(cell(i))));
        cs.push(c);
    }
    number_replicates(&mut cs);
    for (c, w) in cs.iter_mut().zip(written) {
        if let Some(n) = w {
            c.replicate = n;
        }
    }
    let columns = place_free(&mut cs, &free, declared);
    Ok(Table {
        samples: cs,
        columns,
    })
}

/// Gives each free column its type (declared, else inferred) and stores its values as extras; returns the columns of the table.
fn place_free(
    cs: &mut [Sample],
    free: &[(String, Vec<Option<&str>>)],
    declared: &[Column],
) -> Vec<Column> {
    let mut columns = declared.to_vec();
    for (name, values) in free {
        let col = match declared.iter().find(|d| &d.name == name) {
            Some(d) => d.clone(),
            None => {
                let c = Column::new(name, infer(values));
                columns.push(c.clone());
                c
            }
        };
        for (c, v) in cs.iter_mut().zip(values) {
            if let Some(v) = v {
                let cell = match (col.kind, num(v)) {
                    (ColumnType::Number, Some(x)) => Cell::Number(x),
                    _ => Cell::Text((*v).to_string()),
                };
                c.set_extra(name, cell);
            }
        }
    }
    columns
}

fn quote(s: &str, sep: char, force: bool) -> String {
    let t = s.trim();
    if force || s.contains([sep, '"', '\n', '\r']) || t.len() != s.len() {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn fixed(c: &Sample, f: &str) -> String {
    let n = |x: Option<f64>| x.map(|v| format!("{v}")).unwrap_or_default();
    match f {
        "file" => c.file.clone(),
        "sha256" => c.sha256.clone(),
        "sub_sample" => c.sub.clone(),
        "sample_id" => c.sample_id.clone(),
        "polarity" => c.polarity.map(|p| p.to_string()).unwrap_or_default(),
        "matrix" => c.matrix.clone(),
        "batch" => c.batch.clone(),
        "acquired" => c.acquired.clone().unwrap_or_default(),
        "group" => c.group.clone(),
        "kind" => c.kind.clone(),
        "blank_kind" => c.blank_kind.clone().unwrap_or_default(),
        "time" => n(c.time),
        "time_unit" => c.time_unit.clone(),
        "time_confirmed" => c.time_confirmed.to_string(),
        "series" => c.series.clone(),
        "reinjection" => c.reinjection.to_string(),
        "replicate" => c.replicate.to_string(),
        "concentration" => n(c.conc),
        "concentration_unit" => c.unit.clone().unwrap_or_default(),
        "injection_order" => c.order.to_string(),
        "istd" => c.istd.to_string(),
        "note" => c.note.clone(),
        _ => c.compound.clone(),
    }
}

/// The table as CSV, the files in the order given, the free columns sorted by name after the fixed ones; `\n` ends every line.
pub fn write(t: &Table, sep: char) -> String {
    let mut names: Vec<&str> = t.columns.iter().map(|c| c.name.as_str()).collect();
    names.extend(
        t.samples
            .iter()
            .flat_map(|c| c.extra.iter().map(|(k, _)| k.as_str())),
    );
    crate::sortc::slice_ord(&mut names);
    names.dedup();
    let sep_s = sep.to_string();
    let head: Vec<String> = FIXED
        .iter()
        .chain(names.iter())
        .map(|h| quote(h, sep, false))
        .collect();
    let mut out = head.join(&sep_s);
    for c in &t.samples {
        let mut cells: Vec<String> = FIXED
            .iter()
            .map(|f| quote(&fixed(c, f), sep, false))
            .collect();
        for n in &names {
            cells.push(match c.extra_of(n) {
                None => String::new(),
                Some(Cell::Number(x)) => format!("{x}"),
                Some(Cell::Text(s)) => quote(s, sep, s.is_empty()),
            });
        }
        out.push('\n');
        out.push_str(&cells.join(&sep_s));
    }
    out.push('\n');
    out
}

pub(super) fn sep_of(v: &crate::json::Value) -> Result<Option<char>> {
    match v.get("separator").and_then(|s| s.str()) {
        None | Some("") => Ok(None),
        Some(",") => Ok(Some(',')),
        Some(";") => Ok(Some(';')),
        Some("\t") | Some("tab") => Ok(Some('\t')),
        Some(_) => Err(Error::new("error.badRequest")),
    }
}

/// `{"text", "separator"?, "columns"?: [declared columns]}` → a `samples/1`.
pub fn csv_read(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let text = v
        .get("text")
        .and_then(|x| x.str())
        .ok_or_else(|| Error::new("error.badRequest"))?;
    let declared = match v.get("columns") {
        Some(c) => {
            let mut o = Out::default();
            crate::json::write_value(&mut o, c);
            let doc = format!(
                "{{\"samples\":[],\"columns\":{},\"schema\":\"samples/1\"}}",
                o.0
            );
            table_from_value(&parse(doc.as_bytes())?)?.columns
        }
        None => Vec::new(),
    };
    Ok(table_to_json(&read(text, sep_of(&v)?, &declared)?))
}

/// `{"samples": <samples/1 or /2>, "separator"?}` → `{"text": "..."}`.
pub fn csv_write(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let doc = v
        .get("samples")
        .ok_or_else(|| Error::new("error.badRequest"))?;
    let t = table_from_value(doc)?;
    let text = write(&t, sep_of(&v)?.unwrap_or(','));
    let mut j = Out::default();
    j.raw("{\"text\":").str(&text).raw("}");
    Ok(j.0)
}
