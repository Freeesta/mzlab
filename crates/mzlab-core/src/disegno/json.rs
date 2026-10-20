//! The canonical JSON of `disegno/1` (keys in alphabetical order, factors and batches sorted: the same design gives the same bytes and SHA-256)
//! and its reader.

use super::{Condizione, Confronto, Disegno, Fattore, Lotto, SCHEMA};
use crate::budget::sha256_hex;
use crate::error::{Error, Result};
use crate::json::{parse, Out, Value};

fn comparison(j: &mut Out, c: &Confronto) {
    match c {
        Confronto::Pair(a, b) => {
            j.raw("{\"a\":")
                .str(a)
                .raw(",\"b\":")
                .str(b)
                .raw(",\"tipo\":\"pair\"}");
        }
        Confronto::VsReference => {
            j.raw("{\"tipo\":\"vs_reference\"}");
        }
    }
}

/// `{"condizioni":[...],"confronti":[...],"fattori":[...],"lotti":[...],"riferimento":...,"schema":"disegno/1"}`.
pub fn to_json(d: &Disegno) -> String {
    let mut j = Out::default();
    j.raw("{\"condizioni\":[");
    for (i, c) in d.conditions.iter().enumerate() {
        j.raw(if i > 0 {
            ",{\"livelli\":{"
        } else {
            "{\"livelli\":{"
        });
        for (k, (f, v)) in c.levels.iter().enumerate() {
            j.raw(if k > 0 { "," } else { "" }).str(f).raw(":").str(v);
        }
        j.raw("},\"nome\":")
            .str(&c.name)
            .raw(",\"repliche\":")
            .int(c.replicates as i64)
            .raw("}");
    }
    j.raw("],\"confronti\":[");
    for (i, c) in d.comparisons.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" });
        comparison(&mut j, c);
    }
    j.raw("],\"fattori\":[");
    for (i, f) in d.factors.iter().enumerate() {
        j.raw(if i > 0 { ",{\"nome\":" } else { "{\"nome\":" })
            .str(&f.name)
            .raw(",\"valori\":[");
        for (k, v) in f.values.iter().enumerate() {
            j.raw(if k > 0 { "," } else { "" }).str(v);
        }
        j.raw("]}");
    }
    j.raw("],\"lotti\":[");
    for (i, l) in d.batches.iter().enumerate() {
        j.raw(if i > 0 { ",{\"file\":" } else { "{\"file\":" })
            .str(&l.file)
            .raw(",\"lotto\":")
            .str(&l.batch)
            .raw(",\"sha256\":")
            .str(&l.sha256)
            .raw("}");
    }
    j.raw("],\"riferimento\":");
    match &d.reference {
        Some(r) => j.str(r),
        None => j.raw("null"),
    };
    j.raw(",\"schema\":").str(SCHEMA).raw("}");
    j.0
}

/// SHA-256 of the canonical JSON (what the plan records in `prov/1`).
pub fn digest(d: &Disegno) -> String {
    sha256_hex(to_json(d).as_bytes())
}

fn bad() -> Error {
    Error::new("error.badDisegno")
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

fn read_condition(v: &Value) -> Result<Condizione> {
    let n = v.get("repliche").and_then(|x| x.f64()).ok_or_else(bad)?;
    if !(n >= 0.0 && n.fract() == 0.0 && n < 4e9) {
        return Err(bad());
    }
    let levels = match v.get("livelli") {
        Some(Value::Obj(o)) => o
            .iter()
            .map(|(k, x)| Ok((k.clone(), x.str().ok_or_else(bad)?.to_string())))
            .collect::<Result<_>>()?,
        _ => return Err(bad()),
    };
    Ok(Condizione {
        name: text(v, "nome")?,
        replicates: n as u32,
        levels,
    })
}

fn read_comparison(v: &Value) -> Result<Confronto> {
    match v.get("tipo").and_then(|x| x.str()) {
        Some("pair") => Ok(Confronto::Pair(text(v, "a")?, text(v, "b")?)),
        Some("vs_reference") => Ok(Confronto::VsReference),
        _ => Err(bad()),
    }
}

fn read_factor(v: &Value) -> Result<Fattore> {
    Ok(Fattore {
        name: text(v, "nome")?,
        values: list(v, "valori")?
            .iter()
            .map(|x| Ok(x.str().ok_or_else(bad)?.to_string()))
            .collect::<Result<_>>()?,
    })
}

/// Reads a `disegno/1` already parsed; the design is normalised (sorted) so that the same content always gives the same bytes.
pub fn read(doc: &Value) -> Result<Disegno> {
    if doc.get("schema").and_then(|v| v.str()) != Some(SCHEMA) {
        return Err(bad());
    }
    let reference = match doc.get("riferimento") {
        Some(Value::Null) => None,
        Some(x) => Some(x.str().ok_or_else(bad)?.to_string()),
        None => return Err(bad()),
    };
    let mut d = Disegno {
        conditions: list(doc, "condizioni")?
            .iter()
            .map(read_condition)
            .collect::<Result<_>>()?,
        reference,
        comparisons: list(doc, "confronti")?
            .iter()
            .map(read_comparison)
            .collect::<Result<_>>()?,
        factors: list(doc, "fattori")?
            .iter()
            .map(read_factor)
            .collect::<Result<_>>()?,
        batches: list(doc, "lotti")?
            .iter()
            .map(|l| {
                Ok(Lotto {
                    file: text(l, "file")?,
                    sha256: text(l, "sha256")?,
                    batch: text(l, "lotto")?,
                })
            })
            .collect::<Result<_>>()?,
    };
    d.normalize();
    Ok(d)
}

/// Reads a `disegno/1` document (`error.badDisegno` when the schema, a type or a number is wrong).
pub fn from_json(bytes: &[u8]) -> Result<Disegno> {
    read(&parse(bytes)?)
}
