//! The name model of `samples/1`: a regex with named groups (`(?P<sample_id>[A-Z]\d+)_(?P<polarity>pos|neg)`) or a template with placeholders
//! (`{sample_id}_{polarity}_{time}min_r{replicate}`) read against the name of each file (folders and extension left out). A group named
//! like a field gives it (`sample_id`, `polarity`, `matrix`, `batch`, `group`, `series`, `time`, `time_unit`, `replicate`, `concentration`, `concentration_unit`,
//! `kind`, `injection_order`, `note`, `compound`), any other name goes to a free column. Each value comes with the part of the name that gave it,
//! in characters of the whole name, end excluded, as in `propose_explained`. Times stay unconfirmed; a name the model does not match is left as it was.

use super::json::role_alias;
use super::regex::Regex;
use super::{
    names::propose, number_replicates, table_from_value, table_to_json, Cell, Column, ColumnType,
    Sample, Table,
};
use crate::error::{Error, Result};
use crate::json::{parse, Out};

/// The part of a name that gave a field.
#[derive(Debug, Clone, PartialEq)]
pub struct Pos {
    pub field: String,
    pub start: usize,
    pub end: usize,
}

/// The files with the model applied, the spans per file and the indices of the files the model did not match.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub table: Table,
    pub positions: Vec<Vec<Pos>>,
    pub unmatched: Vec<usize>,
}

fn pattern_of(field: &str) -> &'static str {
    match field {
        "polarity" => r"positive|negative|pos|neg|\+1|-1|\+|-|p|n",
        "time" => r"-?\d+(?:[.,pP]\d+)?",
        "concentration" => r"-?\d+(?:[.,]\d+)?",
        "replicate" | "injection_order" => r"\d+",
        "time_unit" => "min|sec|s|h|d",
        _ => ".+?",
    }
}

/// The regex of a template: every `{name}` becomes a named group, the rest is literal text; read without regard to case.
fn from_template(t: &str) -> String {
    let s: Vec<char> = t.chars().collect();
    let mut out = String::from("(?i)");
    let mut i = 0;
    while i < s.len() {
        let name_end = (s[i] == '{')
            .then(|| s[i + 1..].iter().position(|&c| c == '}'))
            .flatten()
            .map(|n| i + 1 + n)
            .filter(|&e| {
                let n = &s[i + 1..e];
                n.first().is_some_and(|c| c.is_alphabetic() || *c == '_')
                    && n.iter().all(|c| c.is_alphanumeric() || *c == '_')
            });
        if let Some(e) = name_end {
            let name: String = s[i + 1..e].iter().collect();
            out.push_str(&format!("(?P<{name}>{})", pattern_of(&name)));
            i = e + 1;
            continue;
        }
        if "\\.^$|?*+()[]{}".contains(s[i]) {
            out.push('\\');
        }
        out.push(s[i]);
        i += 1;
    }
    out
}

fn compile(model: &str) -> Result<Regex> {
    if model.contains("(?P<") || model.contains("(?<") {
        Regex::new(model)
    } else {
        Regex::new(&from_template(model))
    }
}

fn num(v: &str) -> Option<f64> {
    v.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite())
}

fn polarity(v: &str) -> Option<i8> {
    match v.to_lowercase().as_str() {
        "pos" | "positive" | "p" | "+" | "+1" | "1" => Some(1),
        "neg" | "negative" | "n" | "-" | "-1" => Some(-1),
        _ => None,
    }
}

fn time_unit(v: &str) -> Option<&'static str> {
    match v.to_lowercase().as_str() {
        "s" | "sec" | "second" | "seconds" => Some("s"),
        "min" | "minute" | "minutes" => Some("min"),
        "h" | "hour" | "hours" => Some("h"),
        "d" | "day" | "days" => Some("d"),
        _ => None,
    }
}

fn whole(v: &str) -> Option<u32> {
    v.trim().parse::<u32>().ok().filter(|&n| n >= 1)
}

/// Gives `c` the value of one group; false when the value is not one the field takes.
fn set_field(c: &mut Sample, name: &str, v: &str, cols: &[Column]) -> bool {
    match name {
        "sample_id" => c.sample_id = v.into(),
        "matrix" => c.matrix = v.into(),
        "batch" => c.batch = v.into(),
        "group" => c.group = v.into(),
        "series" => c.series = v.into(),
        "note" => c.note = v.into(),
        "compound" => c.compound = v.into(),
        "concentration_unit" => c.unit = Some(v.into()),
        "polarity" => return polarity(v).map(|p| c.polarity = Some(p)).is_some(),
        "replicate" => return whole(v).map(|n| c.replicate = n).is_some(),
        "injection_order" => return whole(v).map(|n| c.order = n).is_some(),
        "concentration" => return num(v).map(|x| c.conc = Some(x)).is_some(),
        "kind" => {
            return role_alias(&v.to_lowercase())
                .map(|k| c.kind = k.into())
                .is_some()
        }
        "time" | "time_unit" => return true,
        _ => {
            let numeric = cols
                .iter()
                .any(|k| k.name == name && k.kind == ColumnType::Number);
            let cell = match num(v).filter(|_| numeric) {
                Some(x) => Cell::Number(x),
                None => Cell::Text(v.into()),
            };
            c.set_extra(name, cell);
        }
    }
    true
}

/// The time and its unit of one match, applied together: the unit of the name or the one the file had.
fn set_time(c: &mut Sample, time: Option<&str>, unit: Option<&str>) {
    let unit = unit
        .and_then(time_unit)
        .unwrap_or(&c.time_unit.clone())
        .to_string();
    match time.map(|t| num(&t.replace(['p', 'P'], "."))) {
        Some(Some(t)) => c.set_time(Some(t), &unit),
        Some(None) => return,
        None => {
            let t = c.time;
            c.set_time(t, &unit);
        }
    }
    c.time_confirmed = false;
}

fn one(re: &Regex, c: &mut Sample, cols: &[Column]) -> Option<Vec<Pos>> {
    let (stem, _, _) = crate::project::guess_sample(&c.file);
    let off = c
        .file
        .rfind(['/', '\\'])
        .map_or(0, |i| c.file[..i + 1].chars().count());
    let chars: Vec<char> = stem.chars().collect();
    let caps = re.full(&chars)?;
    let mut spans = Vec::new();
    let (mut time, mut unit) = (None, None);
    let mut used = Vec::new();
    for (g, cap) in re.names.iter().zip(&caps) {
        let (Some(name), Some((a, b))) = (g, cap) else {
            continue;
        };
        let v: String = chars[*a..*b].iter().collect();
        match name.as_str() {
            "time" => time = Some(v.clone()),
            "time_unit" => unit = Some(v.clone()),
            _ => {}
        }
        if !v.is_empty() && set_field(c, name, &v, cols) {
            spans.push(Pos {
                field: name.clone(),
                start: off + a,
                end: off + b,
            });
            used.push(name.as_str());
        }
    }
    if used.contains(&"time") || used.contains(&"time_unit") {
        set_time(c, time.as_deref(), unit.as_deref());
    }
    super::sample_id::apply(c, &used);
    Some(spans)
}

/// Applies `model` to the files of `t`; a bad model is `error.badModel`.
pub fn apply(model: &str, mut t: Table) -> Result<Applied> {
    let re = compile(model)?;
    let mut positions = Vec::with_capacity(t.samples.len());
    let mut unmatched = Vec::new();
    let mut keep: Vec<Option<u32>> = Vec::with_capacity(t.samples.len());
    for (i, c) in t.samples.iter_mut().enumerate() {
        let before = c.replicate;
        let named = |n: &str| re.names.iter().any(|g| g.as_deref() == Some(n));
        match one(&re, c, &t.columns) {
            Some(p) => {
                keep.push(named("replicate").then_some(c.replicate));
                positions.push(p);
            }
            None => {
                keep.push(Some(before));
                positions.push(Vec::new());
                unmatched.push(i);
            }
        }
    }
    number_replicates(&mut t.samples);
    for (c, k) in t.samples.iter_mut().zip(keep) {
        if let Some(r) = k {
            c.replicate = r;
        }
    }
    Ok(Applied {
        table: t,
        positions,
        unmatched,
    })
}

/// The binding: `{"model", "samples": <samples/1 or /2>}` or `{"model", "files":[{"name","sha256","kind"}]}` (proposed from the names first)
/// → `{"samples":<samples/1>,"positions":[[{"field","end","start"}]],"unmatched":[index]}`.
pub fn apply_model(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let model = v
        .get("model")
        .and_then(|x| x.str())
        .ok_or_else(|| Error::new("error.badRequest"))?;
    let t = match v.get("samples") {
        Some(d) => table_from_value(d)?,
        None => {
            let s = |f: &crate::json::Value, k: &str| {
                f.get(k).and_then(|x| x.str()).unwrap_or("").to_string()
            };
            let files: Vec<(String, String, String)> = v
                .get("files")
                .map(|a| a.arr())
                .unwrap_or_default()
                .iter()
                .map(|f| (s(f, "name"), s(f, "sha256"), s(f, "kind")))
                .collect();
            Table {
                samples: propose(&files),
                columns: Vec::new(),
            }
        }
    };
    let a = apply(model, t)?;
    let mut j = Out::default();
    j.raw("{\"samples\":").raw(&table_to_json(&a.table));
    j.raw(",\"positions\":[");
    for (i, x) in a.positions.iter().enumerate() {
        j.raw(if i > 0 { ",[" } else { "[" });
        for (k, q) in x.iter().enumerate() {
            j.raw(if k > 0 { ",{\"field\":" } else { "{\"field\":" })
                .str(&q.field)
                .raw(",\"end\":")
                .int(q.end as i64)
                .raw(",\"start\":")
                .int(q.start as i64)
                .raw("}");
        }
        j.raw("]");
    }
    j.raw("],\"unmatched\":[");
    for (i, n) in a.unmatched.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).int(*n as i64);
    }
    j.raw("]}");
    Ok(j.0)
}
