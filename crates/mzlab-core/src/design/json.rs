//! The canonical JSON of `design/1` (keys in alphabetical order, factors and batches sorted: the same design gives the same bytes and SHA-256)
//! and its reader.

use super::{Batch, Condition, Design, DesignComparison, Factor, SCHEMA};
use crate::budget::sha256_hex;
use crate::error::{Error, Result};
use crate::json::{parse, Out, Value};

fn comparison(j: &mut Out, c: &DesignComparison) {
    match c {
        DesignComparison::Pair(a, b) => {
            j.raw("{\"a\":")
                .str(a)
                .raw(",\"b\":")
                .str(b)
                .raw(",\"kind\":\"pair\"}");
        }
        DesignComparison::VsReference => {
            j.raw("{\"kind\":\"vs_reference\"}");
        }
    }
}

/// `{"schema":"design/1","batches":[...],"comparisons":[...],"conditions":[...],"factors":[...],"reference":...}`.
pub fn to_json(d: &Design) -> String {
    let mut j = Out::default();
    j.raw("{\"schema\":").str(SCHEMA).raw(",\"batches\":[");
    for (i, l) in d.batches.iter().enumerate() {
        j.raw(if i > 0 { ",{\"batch\":" } else { "{\"batch\":" })
            .str(&l.batch)
            .raw(",\"file\":")
            .str(&l.file)
            .raw(",\"sha256\":")
            .str(&l.sha256)
            .raw("}");
    }
    j.raw("],\"comparisons\":[");
    for (i, c) in d.comparisons.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" });
        comparison(&mut j, c);
    }
    j.raw("],\"conditions\":[");
    for (i, c) in d.conditions.iter().enumerate() {
        j.raw(if i > 0 {
            ",{\"levels\":{"
        } else {
            "{\"levels\":{"
        });
        for (k, (f, v)) in c.levels.iter().enumerate() {
            j.raw(if k > 0 { "," } else { "" }).str(f).raw(":").str(v);
        }
        j.raw("},\"name\":")
            .str(&c.name)
            .raw(",\"replicates\":")
            .int(c.replicates as i64)
            .raw("}");
    }
    j.raw("],\"factors\":[");
    for (i, f) in d.factors.iter().enumerate() {
        j.raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(&f.name)
            .raw(",\"values\":[");
        for (k, v) in f.values.iter().enumerate() {
            j.raw(if k > 0 { "," } else { "" }).str(v);
        }
        j.raw("]}");
    }
    j.raw("],\"reference\":");
    match &d.reference {
        Some(r) => j.str(r),
        None => j.raw("null"),
    };
    j.raw("}");
    j.0
}

/// SHA-256 of the canonical JSON (what the plan records in `prov/1`).
pub fn digest(d: &Design) -> String {
    sha256_hex(to_json(d).as_bytes())
}

fn bad() -> Error {
    Error::new("error.badDesign")
}

fn text(v: &Value, k: &str) -> Result<String> {
    Ok(v.get(k).and_then(|x| x.str()).ok_or_else(bad)?.to_string())
}

fn list<'a>(v: &'a Value, k: &str) -> Result<&'a [Value]> {
    match v.get(k) {
        Some(Value::Arr(a)) => Ok(a),
        _ => Err(bad()),
    }
}

fn read_condition(v: &Value) -> Result<Condition> {
    let n = v.get("replicates").and_then(|x| x.f64()).ok_or_else(bad)?;
    if !(n >= 0.0 && n.fract() == 0.0 && n < 4e9) {
        return Err(bad());
    }
    let levels = match v.get("levels") {
        Some(Value::Obj(o)) => o
            .iter()
            .map(|(k, x)| Ok((k.clone(), x.str().ok_or_else(bad)?.to_string())))
            .collect::<Result<_>>()?,
        _ => return Err(bad()),
    };
    Ok(Condition {
        name: text(v, "name")?,
        replicates: n as u32,
        levels,
    })
}

fn read_comparison(v: &Value) -> Result<DesignComparison> {
    match v.get("kind").and_then(|x| x.str()) {
        Some("pair") => Ok(DesignComparison::Pair(text(v, "a")?, text(v, "b")?)),
        Some("vs_reference") => Ok(DesignComparison::VsReference),
        _ => Err(bad()),
    }
}

fn read_factor(v: &Value) -> Result<Factor> {
    Ok(Factor {
        name: text(v, "name")?,
        values: list(v, "values")?
            .iter()
            .map(|x| Ok(x.str().ok_or_else(bad)?.to_string()))
            .collect::<Result<_>>()?,
    })
}

/// Reads a `design/1` already parsed; the design is normalised (sorted) so that the same content always gives the same bytes.
pub fn read(doc: &Value) -> Result<Design> {
    if doc.get("schema").and_then(|v| v.str()) != Some(SCHEMA) {
        return Err(bad());
    }
    let reference = match doc.get("reference") {
        Some(Value::Null) => None,
        Some(x) => Some(x.str().ok_or_else(bad)?.to_string()),
        None => return Err(bad()),
    };
    let mut d = Design {
        conditions: list(doc, "conditions")?
            .iter()
            .map(read_condition)
            .collect::<Result<_>>()?,
        reference,
        comparisons: list(doc, "comparisons")?
            .iter()
            .map(read_comparison)
            .collect::<Result<_>>()?,
        factors: list(doc, "factors")?
            .iter()
            .map(read_factor)
            .collect::<Result<_>>()?,
        batches: list(doc, "batches")?
            .iter()
            .map(|l| {
                Ok(Batch {
                    file: text(l, "file")?,
                    sha256: text(l, "sha256")?,
                    batch: text(l, "batch")?,
                })
            })
            .collect::<Result<_>>()?,
    };
    d.normalize();
    Ok(d)
}

/// Reads a `design/1` document (`error.badDesign` when the schema, a type or a number is wrong).
pub fn from_json(bytes: &[u8]) -> Result<Design> {
    read(&parse(bytes)?)
}
