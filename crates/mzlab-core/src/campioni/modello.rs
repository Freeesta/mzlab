//! The name model of `campioni/3`: a regex with named groups (`(?P<campione_id>[A-Z]\d+)_(?P<polarita>pos|neg)`) or a template with placeholders
//! (`{campione_id}_{polarita}_{tempo}min_r{replica}`) read against the name of each file (folders and extension left out). A group named
//! like a field gives it (`campione_id`, `polarita`, `matrice`, `lotto`, `gruppo`, `serie`, `tempo`, `unita_tempo`, `replica`, `concentrazione`, `unita`,
//! `tipo`, `ordine_iniezione`, `note`, `composto`), any other name goes to a free column. Each value comes with the part of the name that gave it,
//! in characters of the whole name, end excluded, as in `proponi_spiegato`. Times stay unconfirmed; a name the model does not match is left as it was.

use super::json::legacy_kind;
use super::regex::Regex;
use super::{
    nomi::proponi, number_replicates, tabella_da_value, to_json3, Campione, Cell, Colonna, Tabella,
    TipoColonna,
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
pub struct Applicato {
    pub tabella: Tabella,
    pub positions: Vec<Vec<Pos>>,
    pub unmatched: Vec<usize>,
}

fn pattern_of(field: &str) -> &'static str {
    match field {
        "polarita" => r"positive|negative|pos|neg|\+1|-1|\+|-|p|n",
        "tempo" => r"-?\d+(?:[.,pP]\d+)?",
        "concentrazione" => r"-?\d+(?:[.,]\d+)?",
        "replica" | "ordine_iniezione" => r"\d+",
        "unita_tempo" => "min|sec|s|h|d",
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
fn set_field(c: &mut Campione, name: &str, v: &str, cols: &[Colonna]) -> bool {
    match name {
        "campione_id" => c.sample_id = v.into(),
        "matrice" => c.matrix = v.into(),
        "lotto" => c.batch = v.into(),
        "gruppo" => c.group = v.into(),
        "serie" => c.series = v.into(),
        "note" => c.note = v.into(),
        "composto" => c.compound = v.into(),
        "unita" => c.unit = Some(v.into()),
        "polarita" => return polarity(v).map(|p| c.polarity = Some(p)).is_some(),
        "replica" => return whole(v).map(|n| c.replicate = n).is_some(),
        "ordine_iniezione" => return whole(v).map(|n| c.order = n).is_some(),
        "concentrazione" => return num(v).map(|x| c.conc = Some(x)).is_some(),
        "tipo" => {
            return legacy_kind(&v.to_lowercase())
                .map(|k| c.kind = k.into())
                .is_some()
        }
        "tempo" | "unita_tempo" => return true,
        _ => {
            let numeric = cols
                .iter()
                .any(|k| k.name == name && k.kind == TipoColonna::Numero);
            let cell = match num(v).filter(|_| numeric) {
                Some(x) => Cell::Numero(x),
                None => Cell::Testo(v.into()),
            };
            c.set_extra(name, cell);
        }
    }
    true
}

/// The time and its unit of one match, applied together: the unit of the name or the one the file had.
fn set_time(c: &mut Campione, time: Option<&str>, unit: Option<&str>) {
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

fn one(re: &Regex, c: &mut Campione, cols: &[Colonna]) -> Option<Vec<Pos>> {
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
            "tempo" => time = Some(v.clone()),
            "unita_tempo" => unit = Some(v.clone()),
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
    if used.contains(&"tempo") || used.contains(&"unita_tempo") {
        set_time(c, time.as_deref(), unit.as_deref());
    }
    super::idcampione::applica(c, &used);
    Some(spans)
}

/// Applies `model` to the files of `t`; a bad model is `error.badModello`.
pub fn applica(model: &str, mut t: Tabella) -> Result<Applicato> {
    let re = compile(model)?;
    let mut positions = Vec::with_capacity(t.campioni.len());
    let mut unmatched = Vec::new();
    let mut keep: Vec<Option<u32>> = Vec::with_capacity(t.campioni.len());
    for (i, c) in t.campioni.iter_mut().enumerate() {
        let before = c.replicate;
        let named = |n: &str| re.names.iter().any(|g| g.as_deref() == Some(n));
        match one(&re, c, &t.colonne) {
            Some(p) => {
                keep.push(named("replica").then_some(c.replicate));
                positions.push(p);
            }
            None => {
                keep.push(Some(before));
                positions.push(Vec::new());
                unmatched.push(i);
            }
        }
    }
    number_replicates(&mut t.campioni);
    for (c, k) in t.campioni.iter_mut().zip(keep) {
        if let Some(r) = k {
            c.replicate = r;
        }
    }
    Ok(Applicato {
        tabella: t,
        positions,
        unmatched,
    })
}

/// The binding: `{"modello", "campioni": <campioni/2 or /3>}` or `{"modello", "files":[{"name","sha256","kind"}]}` (proposed from the names first)
/// → `{"campioni":<campioni/3>,"posizioni":[[{"campo","fine","inizio"}]],"senza_corrispondenza":[index]}`.
pub fn applica_modello(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let model = v
        .get("modello")
        .and_then(|x| x.str())
        .ok_or_else(|| Error::new("error.badModello"))?;
    let t = match v.get("campioni") {
        Some(d) => tabella_da_value(d)?,
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
            Tabella {
                campioni: proponi(&files),
                colonne: Vec::new(),
            }
        }
    };
    let a = applica(model, t)?;
    let mut j = Out::default();
    j.raw("{\"campioni\":").raw(&to_json3(&a.tabella));
    j.raw(",\"posizioni\":[");
    for (i, x) in a.positions.iter().enumerate() {
        j.raw(if i > 0 { ",[" } else { "[" });
        for (k, q) in x.iter().enumerate() {
            j.raw(if k > 0 { ",{\"campo\":" } else { "{\"campo\":" })
                .str(&q.field)
                .raw(",\"fine\":")
                .int(q.end as i64)
                .raw(",\"inizio\":")
                .int(q.start as i64)
                .raw("}");
        }
        j.raw("]");
    }
    j.raw("],\"senza_corrispondenza\":[");
    for (i, n) in a.unmatched.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).int(*n as i64);
    }
    j.raw("]}");
    Ok(j.0)
}
