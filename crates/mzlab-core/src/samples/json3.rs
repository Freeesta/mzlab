//! The canonical JSON of `samples/2` (`docs/schemi/samples-2.json`) and its reader, which also migrates a `samples/1`: keys in alphabetical
//! order, files sorted by injection order (then by their own bytes), columns and free values by name, so equal metadata give the same bytes
//! and the same SHA-256 whatever the order they were written in.

use super::json::{bad, number, opt_text, text};
use super::{Cell, Column, ColumnType, FileData, Sample, Table};
use crate::budget::sha256_hex;
use crate::error::Result;
use crate::json::{parse, Out, Value};

pub const SCHEMA3: &str = "samples/2";

fn opt_str(j: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => j.str(s),
        None => j.raw("null"),
    };
}

fn cell(j: &mut Out, c: &Cell) {
    match c {
        Cell::Text(t) => j.str(t),
        Cell::Number(x) => j.num(*x),
    };
}

fn range(j: &mut Out, a: &str, b: &str, v: Option<(f64, f64)>) {
    j.raw(",\"").raw(a).raw("\":").opt(v.map(|x| x.1));
    j.raw(",\"").raw(b).raw("\":").opt(v.map(|x| x.0));
}

fn file_data(j: &mut Out, d: &FileData) {
    j.raw("{\"instrument\":").str(&d.instrument);
    j.raw(",\"method\":").str(&d.method);
    range(j, "mz_max", "mz_min", d.mz);
    range(j, "rt_max", "rt_min", d.rt);
    j.raw(",\"scans\":").opt(d.scans.map(f64::from)).raw("}");
}

/// One file, keys in alphabetical order.
fn one(c: &Sample) -> String {
    let mut j = Out::default();
    j.raw("{\"acquired\":");
    opt_str(&mut j, c.acquired.as_deref());
    j.raw(",\"batch\":").str(&c.batch);
    j.raw(",\"blank_kind\":");
    opt_str(&mut j, c.blank_kind.as_deref());
    j.raw(",\"compound\":").str(&c.compound);
    j.raw(",\"concentration\":").opt(c.conc);
    j.raw(",\"concentration_unit\":");
    opt_str(&mut j, c.unit.as_deref());
    j.raw(",\"extra\":{");
    let mut x: Vec<&(String, Cell)> = c.extra.iter().collect();
    crate::sortc::slice_by(&mut x, |a, b| a.0.cmp(&b.0));
    for (i, (k, v)) in x.into_iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).str(k).raw(":");
        cell(&mut j, v);
    }
    j.raw("},\"file\":").str(&c.file);
    if let Some(d) = &c.file_data {
        j.raw(",\"file_data\":");
        file_data(&mut j, d);
    }
    j.raw(",\"group\":").str(&c.group);
    j.raw(",\"injection_order\":").int(c.order as i64);
    j.raw(",\"istd\":")
        .raw(if c.istd { "true" } else { "false" });
    j.raw(",\"kind\":").str(&c.kind);
    j.raw(",\"matrix\":").str(&c.matrix);
    j.raw(",\"note\":").str(&c.note);
    j.raw(",\"polarity\":").opt(c.polarity.map(f64::from));
    j.raw(",\"reinjection\":")
        .raw(if c.reinjection { "true" } else { "false" });
    j.raw(",\"replicate\":").int(c.replicate as i64);
    j.raw(",\"sample_id\":").str(&c.sample_id);
    j.raw(",\"series\":").str(&c.series);
    j.raw(",\"sha256\":").str(&c.sha256);
    j.raw(",\"sub_sample\":").str(&c.sub);
    j.raw(",\"time\":").opt(c.time);
    j.raw(",\"time_confirmed\":")
        .raw(if c.time_confirmed { "true" } else { "false" });
    j.raw(",\"time_min\":").opt(c.time_min);
    j.raw(",\"time_unit\":").str(&c.time_unit).raw("}");
    j.0
}

fn column(j: &mut Out, c: &Column) {
    j.raw("{\"kind\":").str(c.kind.name());
    j.raw(",\"name\":").str(&c.name);
    if c.required {
        j.raw(",\"required\":true");
    }
    if let Some(v) = &c.values {
        let mut v = v.clone();
        crate::sortc::slice_ord(&mut v);
        v.dedup();
        j.raw(",\"values\":[");
        for (i, x) in v.iter().enumerate() {
            j.raw(if i > 0 { "," } else { "" }).str(x);
        }
        j.raw("]");
    }
    j.raw("}");
}

/// `{"schema":"samples/2","columns":[...],"samples":[...]}`; the files are sorted by `injection_order`, then by their own bytes.
pub fn to_json3(t: &Table) -> String {
    let mut files: Vec<(u32, String)> = t.samples.iter().map(|c| (c.order, one(c))).collect();
    crate::sortc::slice_ord(&mut files);
    let mut cols: Vec<&Column> = t.columns.iter().collect();
    crate::sortc::slice_by(&mut cols, |a, b| a.name.cmp(&b.name));
    let mut j = Out::default();
    j.raw("{\"schema\":").str(SCHEMA3).raw(",\"columns\":[");
    for (i, c) in cols.into_iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" });
        column(&mut j, c);
    }
    j.raw("],\"samples\":[");
    for (i, (_, f)) in files.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).raw(f);
    }
    j.raw("]}");
    j.0
}

/// SHA-256 of the canonical JSON.
pub fn digest3(t: &Table) -> String {
    sha256_hex(to_json3(t).as_bytes())
}

fn polarity(v: &Value) -> Result<Option<i8>> {
    match number(v, "polarity")? {
        None => Ok(None),
        Some(x) if x == 1.0 || x == -1.0 => Ok(Some(x as i8)),
        _ => Err(bad()),
    }
}

fn cell_of(v: &Value) -> Result<Cell> {
    match v {
        Value::Num(x) if x.is_finite() => Ok(Cell::Number(*x)),
        Value::Str(s) => Ok(Cell::Text(s.clone())),
        _ => Err(bad()),
    }
}

fn range_of(d: &Value, hi: &str, lo: &str) -> Result<Option<(f64, f64)>> {
    Ok(number(d, lo)?.zip(number(d, hi)?))
}

/// The fields `samples/2` adds, all optional (a `samples/1` has none): read into `c`.
pub(super) fn read_new(v: &Value, c: &mut Sample) -> Result<()> {
    let s = |k: &str| -> Result<String> { Ok(opt_text(v, k, &[])?.unwrap_or_default()) };
    c.sub = s("sub_sample")?;
    c.reinjection = v.get("reinjection").and_then(|x| x.bool()).unwrap_or(false);
    c.sample_id = s("sample_id")?;
    c.matrix = s("matrix")?;
    c.batch = s("batch")?;
    c.group = s("group")?;
    c.acquired = opt_text(v, "acquired", &[])?;
    c.polarity = polarity(v)?;
    if let Some(Value::Obj(o)) = v.get("extra") {
        for (k, x) in o {
            c.set_extra(k, cell_of(x)?);
        }
    }
    if let Some(d @ Value::Obj(_)) = v.get("file_data") {
        c.file_data = Some(FileData {
            instrument: opt_text(d, "instrument", &[])?.unwrap_or_default(),
            method: opt_text(d, "method", &[])?.unwrap_or_default(),
            mz: range_of(d, "mz_max", "mz_min")?,
            rt: range_of(d, "rt_max", "rt_min")?,
            scans: number(d, "scans")?.map(|x| x as u32),
        });
    }
    Ok(())
}

fn read_column(v: &Value) -> Result<Column> {
    let kind = ColumnType::from_name(&text(v, "kind")?).ok_or_else(bad)?;
    let mut c = Column::new(&text(v, "name")?, kind);
    c.required = v.get("required").and_then(|x| x.bool()).unwrap_or(false);
    if let Some(a) = v.get("values") {
        c.values = Some(
            a.arr()
                .iter()
                .map(|x| x.str().map(str::to_string).ok_or_else(bad))
                .collect::<Result<_>>()?,
        );
    }
    Ok(c)
}

/// Reads a `samples/2` document or migrates a `samples/1` one (the new fields empty, no columns): files in the order written.
pub fn table_from_value(doc: &Value) -> Result<Table> {
    let samples = super::from_value(doc)?;
    let columns = doc
        .get("columns")
        .map(|a| a.arr().iter().map(read_column).collect::<Result<_>>())
        .transpose()?
        .unwrap_or_default();
    Ok(Table { samples, columns })
}

pub fn table_from_json(bytes: &[u8]) -> Result<Table> {
    table_from_value(&parse(bytes)?)
}
