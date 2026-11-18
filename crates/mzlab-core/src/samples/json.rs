//! The canonical JSON of `samples/1` (keys in alphabetical order: the same metadata give the same bytes and the same SHA-256), its reader,
//! and the migration of the old places where roles and times were kept.

use super::{Sample, BLANK_TYPES, SCHEMA, TIME_UNITS, TYPES};
use crate::budget::sha256_hex;
use crate::error::{Error, Result};
use crate::json::{parse, Out, Value};

fn opt_str(j: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => j.str(s),
        None => j.raw("null"),
    };
}

fn one(j: &mut Out, c: &Sample) {
    j.raw("{\"blank_kind\":");
    opt_str(j, c.blank_kind.as_deref());
    j.raw(",\"compound\":")
        .str(&c.compound)
        .raw(",\"concentration\":")
        .opt(c.conc);
    j.raw(",\"concentration_unit\":");
    opt_str(j, c.unit.as_deref());
    j.raw(",\"file\":")
        .str(&c.file)
        .raw(",\"injection_order\":")
        .int(c.order as i64);
    j.raw(",\"istd\":")
        .raw(if c.istd { "true" } else { "false" });
    j.raw(",\"kind\":")
        .str(&c.kind)
        .raw(",\"note\":")
        .str(&c.note);
    j.raw(",\"replicate\":")
        .int(c.replicate as i64)
        .raw(",\"series\":")
        .str(&c.series);
    j.raw(",\"sha256\":")
        .str(&c.sha256)
        .raw(",\"time\":")
        .opt(c.time);
    j.raw(",\"time_confirmed\":")
        .raw(if c.time_confirmed { "true" } else { "false" });
    j.raw(",\"time_min\":")
        .opt(c.time_min)
        .raw(",\"time_unit\":")
        .str(&c.time_unit)
        .raw("}");
}

/// `{"schema":"samples/1","samples":[...]}`: `schema` first, then the keys of every file in alphabetical order.
pub fn to_json(cs: &[Sample]) -> String {
    let mut j = Out::default();
    j.raw("{\"schema\":").str(SCHEMA).raw(",\"samples\":[");
    for (i, c) in cs.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" });
        one(&mut j, c);
    }
    j.raw("]}");
    j.0
}

/// SHA-256 of the canonical JSON (what the `samples` operation of the `.mzflow` records).
pub fn digest(cs: &[Sample]) -> String {
    sha256_hex(to_json(cs).as_bytes())
}

pub(super) fn bad() -> Error {
    Error::new("error.badSamples")
}

pub(super) fn text(v: &Value, k: &str) -> Result<String> {
    Ok(v.get(k).and_then(|x| x.str()).ok_or_else(bad)?.to_string())
}

pub(super) fn number(v: &Value, k: &str) -> Result<Option<f64>> {
    match v.get(k) {
        None => Ok(None),
        Some(x) if x.is_null() => Ok(None),
        Some(x) => x.f64().filter(|f| f.is_finite()).map(Some).ok_or_else(bad),
    }
}

pub(super) fn whole(v: &Value, k: &str) -> Result<u32> {
    let x = number(v, k)?.ok_or_else(bad)?;
    (x >= 1.0 && x.fract() == 0.0 && x < 4e9)
        .then_some(x as u32)
        .ok_or_else(bad)
}

pub(super) fn opt_text(v: &Value, k: &str, allowed: &[&str]) -> Result<Option<String>> {
    match v.get(k) {
        None => Ok(None),
        Some(x) if x.is_null() => Ok(None),
        Some(x) => x
            .str()
            .filter(|s| allowed.is_empty() || allowed.contains(s))
            .map(|s| Some(s.to_string()))
            .ok_or_else(bad),
    }
}

fn read_one(v: &Value) -> Result<Sample> {
    let mut c = Sample::new(
        &text(v, "file")?,
        &text(v, "sha256")?,
        whole(v, "injection_order")?,
    );
    c.kind = text(v, "kind")?;
    c.time_unit = text(v, "time_unit")?;
    if !TYPES.contains(&c.kind.as_str()) || !TIME_UNITS.contains(&c.time_unit.as_str()) {
        return Err(bad());
    }
    c.blank_kind = opt_text(v, "blank_kind", &BLANK_TYPES)?;
    let unit = c.time_unit.clone();
    c.set_time(number(v, "time")?, &unit);
    if c.kind == "t0" && c.time.is_none() {
        c.time_min = Some(0.0);
    }
    c.time_confirmed = v
        .get("time_confirmed")
        .and_then(|x| x.bool())
        .ok_or_else(bad)?;
    c.series = text(v, "series")?;
    c.replicate = whole(v, "replicate")?;
    c.conc = number(v, "concentration")?;
    c.unit = opt_text(v, "concentration_unit", &[])?;
    c.istd = v.get("istd").and_then(|x| x.bool()).ok_or_else(bad)?;
    c.note = text(v, "note")?;
    c.compound = text(v, "compound")?;
    super::json3::read_new(v, &mut c)?;
    Ok(c)
}

/// Reads a `samples/1` (or `/3`, the new fields apart) document: the roles, kinds of blank and units must be the ones of the schema; `time_min` is derived again from
/// `time` and `time_unit` (a time zero without a time has 0 minutes).
pub fn from_json(bytes: &[u8]) -> Result<Vec<Sample>> {
    from_value(&parse(bytes)?)
}

/// The same as `from_json` on a document already parsed (a part of a larger document).
pub fn from_value(doc: &Value) -> Result<Vec<Sample>> {
    if !matches!(
        doc.get("schema").and_then(|v| v.str()),
        Some(SCHEMA) | Some(super::SCHEMA3)
    ) {
        return Err(bad());
    }
    doc.get("samples")
        .map(|a| a.arr())
        .unwrap_or(&[])
        .iter()
        .map(read_one)
        .collect()
}

/// A role of the old vocabularies (Data view, QA/QC, `samples/1`, Dig) in `samples/1`; None when unknown.
pub(super) fn legacy_kind(t: &str) -> Option<&'static str> {
    Some(match t {
        "sample" => "sample",
        "blank" => "blank",
        "standard" | "std" => "std",
        "calibration" => "calibration",
        "dark" | "control" => "dark",
        "qc" => "qc",
        "pool" => "pool",
        "t0" => "t0",
        "infusion" => "infusion",
        _ => return None,
    })
}
