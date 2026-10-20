//! The canonical JSON of `liste/1` and its reader: keys in alphabetical order, items sorted by name and change, so that the same list gives the same bytes
//! (and the same SHA-256) whatever the order of the rows and whatever the CSV it came from.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use super::{Adduct, Kind, Liste, RowError, Suspect, Transformation, SCHEMA};
use crate::legacy::transf::Delta;

fn bad() -> Error {
    Error::new("error.badListe")
}

fn delta(o: &mut Out, d: &Delta) {
    o.raw("{");
    for (i, (e, n)) in d.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(e).raw(":").int(*n);
    }
    o.raw("}");
}

fn sorted<T: Clone>(v: &[T], key: impl Fn(&T) -> (String, String)) -> Vec<T> {
    let mut v = v.to_vec();
    crate::sortc::any_by(&mut v, |a, b| key(a).cmp(&key(b)));
    v
}

fn delta_key(d: &Delta) -> String {
    d.iter().map(|(e, n)| format!("{e}{n}")).collect()
}

/// `{"elementi":[...],"schema":"liste/1","tipo":"transformations"|"adducts"|"suspects"}`.
pub fn to_json(l: &Liste) -> String {
    let mut o = Out::default();
    o.raw("{\"elementi\":[");
    match l {
        Liste::Transformations(v) => {
            for (i, t) in sorted(v, |t| (t.name.clone(), delta_key(&t.delta)))
                .iter()
                .enumerate()
            {
                o.raw(if i > 0 {
                    ",{\"commento\":"
                } else {
                    "{\"commento\":"
                })
                .str(&t.comment)
                .raw(",\"delta\":");
                delta(&mut o, &t.delta);
                o.raw(",\"nome\":")
                    .str(&t.name)
                    .raw(",\"ripetizioni_max\":");
                match t.max_repeat {
                    Some(m) => o.int(i64::from(m)),
                    None => o.raw("null"),
                };
                o.raw("}");
            }
        }
        Liste::Adducts(v) => {
            for (i, a) in sorted(v, |a| (a.name.clone(), delta_key(&a.delta)))
                .iter()
                .enumerate()
            {
                o.raw(if i > 0 {
                    ",{\"carica\":"
                } else {
                    "{\"carica\":"
                })
                .int(i64::from(a.charge))
                .raw(",\"commento\":")
                .str(&a.comment)
                .raw(",\"delta\":");
                delta(&mut o, &a.delta);
                o.raw(",\"nome\":").str(&a.name).raw("}");
            }
        }
        Liste::Suspects(v) => {
            for (i, s) in sorted(v, |s| {
                (s.name.clone(), s.formula.clone().unwrap_or_default())
            })
            .iter()
            .enumerate()
            {
                o.raw(if i > 0 {
                    ",{\"commento\":"
                } else {
                    "{\"commento\":"
                })
                .str(&s.comment)
                .raw(",\"formula\":");
                match &s.formula {
                    Some(f) => o.str(f),
                    None => o.raw("null"),
                };
                o.raw(",\"massa\":")
                    .opt(s.mass)
                    .raw(",\"nome\":")
                    .str(&s.name)
                    .raw("}");
            }
        }
    }
    o.raw("],\"schema\":")
        .str(SCHEMA)
        .raw(",\"tipo\":")
        .str(l.kind().name())
        .raw("}");
    o.0
}

fn text(v: &Value, k: &str) -> Result<String> {
    v.get(k)
        .and_then(Value::str)
        .map(str::to_string)
        .ok_or_else(bad)
}

fn delta_of(v: &Value) -> Result<Delta> {
    let Some(Value::Obj(m)) = v.get("delta") else {
        return Err(bad());
    };
    let mut d: Delta = m
        .iter()
        .map(|(e, n)| {
            n.f64()
                .filter(|x| x.fract() == 0.0 && *x != 0.0)
                .map(|x| (e.clone(), x as i64))
                .ok_or_else(bad)
        })
        .collect::<Result<_>>()?;
    crate::sortc::any_by(&mut d, |a, b| a.0.cmp(&b.0));
    Ok(d)
}

fn number(v: &Value, k: &str) -> Result<Option<f64>> {
    match v.get(k) {
        Some(x) if x.is_null() => Ok(None),
        Some(x) => x.f64().filter(|n| n.is_finite()).map(Some).ok_or_else(bad),
        None => Err(bad()),
    }
}

/// Reads `liste/1`; a wrong schema, kind or item is `error.badListe`. The items keep the order of the file (the canonical one).
pub fn from_json(b: &[u8]) -> Result<Liste> {
    let v = parse(b)?;
    if v.get("schema").and_then(Value::str) != Some(SCHEMA) {
        return Err(bad());
    }
    let kind = v
        .get("tipo")
        .and_then(Value::str)
        .and_then(Kind::from_name)
        .ok_or_else(bad)?;
    let items = v.get("elementi").map(Value::arr).ok_or_else(bad)?;
    Ok(match kind {
        Kind::Transformations => Liste::Transformations(
            items
                .iter()
                .map(|i| {
                    let max = number(i, "ripetizioni_max")?.map(|m| m as u32);
                    Ok(Transformation {
                        name: text(i, "nome")?,
                        delta: delta_of(i)?,
                        max_repeat: max,
                        comment: text(i, "commento")?,
                    })
                })
                .collect::<Result<_>>()?,
        ),
        Kind::Adducts => Liste::Adducts(
            items
                .iter()
                .map(|i| {
                    let charge = number(i, "carica")?
                        .filter(|c| c.fract() == 0.0 && *c != 0.0 && c.abs() <= 3.0)
                        .ok_or_else(bad)?;
                    Ok(Adduct {
                        name: text(i, "nome")?,
                        delta: delta_of(i)?,
                        charge: charge as i8,
                        comment: text(i, "commento")?,
                    })
                })
                .collect::<Result<_>>()?,
        ),
        Kind::Suspects => Liste::Suspects(
            items
                .iter()
                .map(|i| {
                    let formula = i
                        .get("formula")
                        .filter(|f| !f.is_null())
                        .map(|_| text(i, "formula"))
                        .transpose()?;
                    Ok(Suspect {
                        name: text(i, "nome")?,
                        formula,
                        mass: number(i, "massa")?,
                        comment: text(i, "commento")?,
                    })
                })
                .collect::<Result<_>>()?,
        ),
    })
}

/// `{"errors":[{"detail":...,"key":...,"line":...}]}` for the rows of a CSV that are wrong.
pub fn errors_json(errors: &[RowError]) -> String {
    let mut o = Out::default();
    o.raw("{\"errors\":[");
    for (i, e) in errors.iter().enumerate() {
        o.raw(if i > 0 {
            ",{\"detail\":"
        } else {
            "{\"detail\":"
        })
        .str(&e.detail)
        .raw(",\"key\":")
        .str(e.key)
        .raw(",\"line\":")
        .int(e.line as i64)
        .raw("}");
    }
    o.raw("]}");
    o.0
}
