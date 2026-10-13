//! The canonical JSON of `campioni/2` (keys in alphabetical order: the same metadata give the same bytes and the same SHA-256), its reader,
//! and the migration of the old places where roles and times were kept.

use super::{number_replicates, Campione, SCHEMA, TIPI, TIPI_BIANCO, UNITA_TEMPO};
use crate::budget::sha256_hex;
use crate::error::{Error, Result};
use crate::json::{parse, Out, Value};

fn opt_str(j: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => j.str(s),
        None => j.raw("null"),
    };
}

fn one(j: &mut Out, c: &Campione) {
    j.raw("{\"composto\":")
        .str(&c.compound)
        .raw(",\"concentrazione\":")
        .opt(c.conc);
    j.raw(",\"file\":")
        .str(&c.file)
        .raw(",\"istd\":")
        .raw(if c.istd { "true" } else { "false" });
    j.raw(",\"note\":")
        .str(&c.note)
        .raw(",\"ordine_iniezione\":")
        .int(c.order as i64);
    j.raw(",\"replica\":")
        .int(c.replicate as i64)
        .raw(",\"serie\":")
        .str(&c.series);
    j.raw(",\"sha256\":")
        .str(&c.sha256)
        .raw(",\"tempo\":")
        .opt(c.time);
    j.raw(",\"tempo_confermato\":")
        .raw(if c.time_confirmed { "true" } else { "false" });
    j.raw(",\"tempo_min\":")
        .opt(c.time_min)
        .raw(",\"tipo\":")
        .str(&c.kind)
        .raw(",\"tipo_bianco\":");
    opt_str(j, c.blank_kind.as_deref());
    j.raw(",\"unita\":");
    opt_str(j, c.unit.as_deref());
    j.raw(",\"unita_tempo\":").str(&c.time_unit).raw("}");
}

/// `{"campioni":[...],"schema":"campioni/2"}` with the keys of every file in alphabetical order.
pub fn to_json(cs: &[Campione]) -> String {
    let mut j = Out::default();
    j.raw("{\"campioni\":[");
    for (i, c) in cs.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" });
        one(&mut j, c);
    }
    j.raw("],\"schema\":").str(SCHEMA).raw("}");
    j.0
}

/// SHA-256 of the canonical JSON (what the `campioni` operation of the `.mzflow` records).
pub fn digest(cs: &[Campione]) -> String {
    sha256_hex(to_json(cs).as_bytes())
}

fn bad() -> Error {
    Error::new("error.badCampioni")
}

fn text(v: &Value, k: &str) -> Result<String> {
    Ok(v.get(k).and_then(|x| x.str()).ok_or_else(bad)?.to_string())
}

fn number(v: &Value, k: &str) -> Result<Option<f64>> {
    match v.get(k) {
        None => Ok(None),
        Some(x) if x.is_null() => Ok(None),
        Some(x) => x.f64().filter(|f| f.is_finite()).map(Some).ok_or_else(bad),
    }
}

fn whole(v: &Value, k: &str) -> Result<u32> {
    let x = number(v, k)?.ok_or_else(bad)?;
    (x >= 1.0 && x.fract() == 0.0 && x < 4e9)
        .then_some(x as u32)
        .ok_or_else(bad)
}

fn opt_text(v: &Value, k: &str, allowed: &[&str]) -> Result<Option<String>> {
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

fn read_one(v: &Value) -> Result<Campione> {
    let mut c = Campione::new(
        &text(v, "file")?,
        &text(v, "sha256")?,
        whole(v, "ordine_iniezione")?,
    );
    c.kind = text(v, "tipo")?;
    c.time_unit = text(v, "unita_tempo")?;
    if !TIPI.contains(&c.kind.as_str()) || !UNITA_TEMPO.contains(&c.time_unit.as_str()) {
        return Err(bad());
    }
    c.blank_kind = opt_text(v, "tipo_bianco", &TIPI_BIANCO)?;
    let unit = c.time_unit.clone();
    c.set_time(number(v, "tempo")?, &unit);
    if c.kind == "t0" && c.time.is_none() {
        c.time_min = Some(0.0);
    }
    c.time_confirmed = v
        .get("tempo_confermato")
        .and_then(|x| x.bool())
        .ok_or_else(bad)?;
    c.series = text(v, "serie")?;
    c.replicate = whole(v, "replica")?;
    c.conc = number(v, "concentrazione")?;
    c.unit = opt_text(v, "unita", &[])?;
    c.istd = v.get("istd").and_then(|x| x.bool()).ok_or_else(bad)?;
    c.note = text(v, "note")?;
    c.compound = text(v, "composto")?;
    Ok(c)
}

/// Reads a `campioni/2` document: the roles, kinds of blank and units must be the ones of the schema; `tempo_min` is derived again from
/// `tempo` and `unita_tempo` (a time zero without a time has 0 minutes).
pub fn from_json(bytes: &[u8]) -> Result<Vec<Campione>> {
    let doc = parse(bytes)?;
    if doc.get("schema").and_then(|v| v.str()) != Some(SCHEMA) {
        return Err(bad());
    }
    doc.get("campioni")
        .map(|a| a.arr())
        .unwrap_or(&[])
        .iter()
        .map(read_one)
        .collect()
}

/// A role of the old vocabularies (Data view, QA/QC, `campioni/1`, Dig) in `campioni/2`; None when unknown.
fn legacy_kind(t: &str) -> Option<&'static str> {
    Some(match t {
        "sample" => "sample",
        "blank" => "blank",
        "standard" | "std" => "std",
        "calibration" => "calibration",
        "dark" | "control" => "dark",
        "qc" => "qc",
        "t0" => "t0",
        "infusion" => "infusion",
        _ => return None,
    })
}

/// The entry of `name`, made at the end when it is new.
fn slot<'a>(cs: &'a mut Vec<Campione>, name: &str) -> &'a mut Campione {
    match cs.iter().position(|c| c.file == name) {
        Some(i) => &mut cs[i],
        None => {
            let n = cs.len() as u32 + 1;
            let mut c = Campione::new(name, "", n);
            c.replicate = 0; // 0 = not written: numbered at the end
            cs.push(c);
            cs.last_mut().unwrap()
        }
    }
}

/// The fields one old entry gives: name, SHA-256, role, minutes (confirmed or not), concentration, unit, replicate.
struct Old<'a> {
    v: &'a Value,
    name: &'a str,
    kind: &'a str,
    minutes: &'a str,
    conc: &'a str,
    unit: &'a str,
    confirmed: bool,
}

fn apply(cs: &mut Vec<Campione>, o: &Old) -> Result<()> {
    let s = |k: &str| o.v.get(k).and_then(|x| x.str()).filter(|x| !x.is_empty());
    let Some(name) = s(o.name) else { return Ok(()) };
    let c = slot(cs, name);
    if let Some(sha) = s("sha256") {
        c.sha256 = sha.to_string();
    }
    if let Some(k) = s(o.kind).and_then(legacy_kind) {
        c.kind = k.to_string();
    }
    if let Some(t) = number(o.v, o.minutes)? {
        c.set_time(Some(t), "min");
        c.time_confirmed = o.confirmed;
    }
    if let Some(x) = number(o.v, o.conc)? {
        c.conc = Some(x);
        c.unit = s(o.unit).map(str::to_string);
    }
    if let Some(r) =
        o.v.get(if o.confirmed { "rep" } else { "replica" })
            .and_then(|x| x.f64())
            .filter(|r| *r >= 1.0 && r.fract() == 0.0)
    {
        c.replicate = r as u32;
    }
    Ok(())
}

fn list(a: Option<&Value>) -> &[Value] {
    a.map(|x| x.arr()).unwrap_or(&[])
}

/// `campioni/2` from the old places, `{"session":[NB.session v1], "qaqc":{"roles":{file: role}}, "campioni1":{campioni/1}, "mzflow":{"files":[...]}}`
/// (every part optional). The order of the files is the one of the session, then the `.mzflow`, then `campioni/1`; a field is taken from
/// the session first, then the `.mzflow`, then `campioni/1`, and the QA/QC role wins (it kept the QC and dark roles the session saved as
/// `sample`). The times of the session and of the `.mzflow` were written by the user (confirmed); those of `campioni/1` were proposed.
pub fn da_legacy(bytes: &[u8]) -> Result<Vec<Campione>> {
    let doc = parse(bytes)?;
    let session = list(doc.get("session"));
    let flow = list(doc.get("mzflow").and_then(|m| m.get("files")));
    let c1 = list(doc.get("campioni1").and_then(|m| m.get("campioni")));
    let mut cs: Vec<Campione> = Vec::new();
    for (v, k) in session
        .iter()
        .map(|v| (v, "file"))
        .chain(flow.iter().map(|v| (v, "nome")))
        .chain(c1.iter().map(|v| (v, "file")))
    {
        if let Some(n) = v.get(k).and_then(|x| x.str()).filter(|x| !x.is_empty()) {
            slot(&mut cs, n);
        }
    }
    for v in c1.iter().rev() {
        apply(
            &mut cs,
            &Old {
                v,
                name: "file",
                kind: "tipo",
                minutes: "tempo_min",
                conc: "concentrazione",
                unit: "unita",
                confirmed: false,
            },
        )?;
        let c = slot(&mut cs, v.get("file").and_then(|x| x.str()).unwrap_or(""));
        c.istd = v.get("istd").and_then(|x| x.bool()).unwrap_or(false);
        c.note = v
            .get("note")
            .and_then(|x| x.str())
            .unwrap_or("")
            .to_string();
    }
    cs.retain(|c| !c.file.is_empty());
    for v in flow.iter().rev() {
        apply(
            &mut cs,
            &Old {
                v,
                name: "nome",
                kind: "tipo",
                minutes: "tempo",
                conc: "concentrazione",
                unit: "unita_conc",
                confirmed: true,
            },
        )?;
    }
    for v in session.iter().rev() {
        apply(
            &mut cs,
            &Old {
                v,
                name: "file",
                kind: "type",
                minutes: "time",
                conc: "conc",
                unit: "cunit",
                confirmed: true,
            },
        )?;
    }
    finish(&mut cs, doc.get("qaqc"));
    Ok(cs)
}

/// The QA/QC roles, a time zero for a sample at 0 minutes, no time for the files that have none, the replicates not written.
fn finish(cs: &mut [Campione], qaqc: Option<&Value>) {
    let roles = qaqc.and_then(|q| q.get("roles").or_else(|| q.get("ruoli")));
    let mut written = Vec::with_capacity(cs.len());
    for c in cs.iter_mut() {
        if let Some(k) = roles
            .and_then(|r| r.get(&c.file))
            .and_then(|x| x.str())
            .and_then(legacy_kind)
        {
            c.kind = k.to_string();
        }
        if c.kind == "sample" && c.time_min == Some(0.0) {
            c.kind = "t0".into();
        }
        if matches!(
            c.kind.as_str(),
            "blank" | "std" | "calibration" | "qc" | "infusion"
        ) {
            c.set_time(None, "min");
        }
        written.push(c.replicate);
    }
    number_replicates(cs);
    for (c, w) in cs.iter_mut().zip(written) {
        if w > 0 {
            c.replicate = w;
        }
    }
}
