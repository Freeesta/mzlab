//! The canonical JSON of `campioni/3` (`docs/schemi/campioni-3.json`) and its reader, which also migrates a `campioni/2`: keys in alphabetical
//! order, files sorted by injection order (then by their own bytes), columns and free values by name, so equal metadata give the same bytes
//! and the same SHA-256 whatever the order they were written in.

use super::json::{bad, number, opt_text, text};
use super::{Campione, Cell, Colonna, DatiFile, Tabella, TipoColonna};
use crate::budget::sha256_hex;
use crate::error::Result;
use crate::json::{parse, Out, Value};

pub const SCHEMA3: &str = "campioni/3";

fn opt_str(j: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => j.str(s),
        None => j.raw("null"),
    };
}

fn cell(j: &mut Out, c: &Cell) {
    match c {
        Cell::Testo(t) => j.str(t),
        Cell::Numero(x) => j.num(*x),
    };
}

fn range(j: &mut Out, a: &str, b: &str, v: Option<(f64, f64)>) {
    j.raw(",\"").raw(a).raw("\":").opt(v.map(|x| x.1));
    j.raw(",\"").raw(b).raw("\":").opt(v.map(|x| x.0));
}

fn file_data(j: &mut Out, d: &DatiFile) {
    j.raw("{\"metodo\":").str(&d.method);
    j.raw(",\"modello\":").str(&d.instrument);
    range(j, "mz_max", "mz_min", d.mz);
    range(j, "rt_max", "rt_min", d.rt);
    j.raw(",\"scansioni\":")
        .opt(d.scans.map(f64::from))
        .raw("}");
}

/// One file, keys in alphabetical order.
fn one(c: &Campione) -> String {
    let mut j = Out::default();
    j.raw("{\"acquisito\":");
    opt_str(&mut j, c.acquired.as_deref());
    j.raw(",\"campione\":").str(&c.sub);
    j.raw(",\"campione_id\":").str(&c.sample_id);
    j.raw(",\"composto\":").str(&c.compound);
    j.raw(",\"concentrazione\":").opt(c.conc);
    if let Some(d) = &c.file_data {
        j.raw(",\"dati_file\":");
        file_data(&mut j, d);
    }
    j.raw(",\"extra\":{");
    let mut x: Vec<&(String, Cell)> = c.extra.iter().collect();
    crate::sortc::slice_by(&mut x, |a, b| a.0.cmp(&b.0));
    for (i, (k, v)) in x.into_iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).str(k).raw(":");
        cell(&mut j, v);
    }
    j.raw("},\"file\":").str(&c.file);
    j.raw(",\"gruppo\":").str(&c.group);
    j.raw(",\"istd\":")
        .raw(if c.istd { "true" } else { "false" });
    j.raw(",\"lotto\":").str(&c.batch);
    j.raw(",\"matrice\":").str(&c.matrix);
    j.raw(",\"note\":").str(&c.note);
    j.raw(",\"ordine_iniezione\":").int(c.order as i64);
    j.raw(",\"polarita\":").opt(c.polarity.map(f64::from));
    j.raw(",\"reiniezione\":")
        .raw(if c.reinjection { "true" } else { "false" });
    j.raw(",\"replica\":").int(c.replicate as i64);
    j.raw(",\"serie\":").str(&c.series);
    j.raw(",\"sha256\":").str(&c.sha256);
    j.raw(",\"tempo\":").opt(c.time);
    j.raw(",\"tempo_confermato\":")
        .raw(if c.time_confirmed { "true" } else { "false" });
    j.raw(",\"tempo_min\":").opt(c.time_min);
    j.raw(",\"tipo\":").str(&c.kind).raw(",\"tipo_bianco\":");
    opt_str(&mut j, c.blank_kind.as_deref());
    j.raw(",\"unita\":");
    opt_str(&mut j, c.unit.as_deref());
    j.raw(",\"unita_tempo\":").str(&c.time_unit).raw("}");
    j.0
}

fn column(j: &mut Out, c: &Colonna) {
    j.raw("{\"nome\":").str(&c.name);
    if c.required {
        j.raw(",\"obbligatoria\":true");
    }
    j.raw(",\"tipo\":").str(c.kind.name());
    if let Some(v) = &c.values {
        let mut v = v.clone();
        crate::sortc::slice_ord(&mut v);
        v.dedup();
        j.raw(",\"valori\":[");
        for (i, x) in v.iter().enumerate() {
            j.raw(if i > 0 { "," } else { "" }).str(x);
        }
        j.raw("]");
    }
    j.raw("}");
}

/// `{"campioni":[...],"colonne":[...],"schema":"campioni/3"}`; the files are sorted by `ordine_iniezione`, then by their own bytes.
pub fn to_json3(t: &Tabella) -> String {
    let mut files: Vec<(u32, String)> = t.campioni.iter().map(|c| (c.order, one(c))).collect();
    crate::sortc::slice_ord(&mut files);
    let mut cols: Vec<&Colonna> = t.colonne.iter().collect();
    crate::sortc::slice_by(&mut cols, |a, b| a.name.cmp(&b.name));
    let mut j = Out::default();
    j.raw("{\"campioni\":[");
    for (i, (_, f)) in files.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).raw(f);
    }
    j.raw("],\"colonne\":[");
    for (i, c) in cols.into_iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" });
        column(&mut j, c);
    }
    j.raw("],\"schema\":").str(SCHEMA3).raw("}");
    j.0
}

/// SHA-256 of the canonical JSON.
pub fn digest3(t: &Tabella) -> String {
    sha256_hex(to_json3(t).as_bytes())
}

fn polarity(v: &Value) -> Result<Option<i8>> {
    match number(v, "polarita")? {
        None => Ok(None),
        Some(x) if x == 1.0 || x == -1.0 => Ok(Some(x as i8)),
        _ => Err(bad()),
    }
}

fn cell_of(v: &Value) -> Result<Cell> {
    match v {
        Value::Num(x) if x.is_finite() => Ok(Cell::Numero(*x)),
        Value::Str(s) => Ok(Cell::Testo(s.clone())),
        _ => Err(bad()),
    }
}

fn range_of(d: &Value, hi: &str, lo: &str) -> Result<Option<(f64, f64)>> {
    Ok(number(d, lo)?.zip(number(d, hi)?))
}

/// The fields `campioni/3` adds, all optional (a `campioni/2` has none): read into `c`.
pub(super) fn leggi_nuovi(v: &Value, c: &mut Campione) -> Result<()> {
    let s = |k: &str| -> Result<String> { Ok(opt_text(v, k, &[])?.unwrap_or_default()) };
    c.sub = s("campione")?;
    c.reinjection = v.get("reiniezione").and_then(|x| x.bool()).unwrap_or(false);
    c.sample_id = s("campione_id")?;
    c.matrix = s("matrice")?;
    c.batch = s("lotto")?;
    c.group = s("gruppo")?;
    c.acquired = opt_text(v, "acquisito", &[])?;
    c.polarity = polarity(v)?;
    if let Some(Value::Obj(o)) = v.get("extra") {
        for (k, x) in o {
            c.set_extra(k, cell_of(x)?);
        }
    }
    if let Some(d @ Value::Obj(_)) = v.get("dati_file") {
        c.file_data = Some(DatiFile {
            instrument: opt_text(d, "modello", &[])?.unwrap_or_default(),
            method: opt_text(d, "metodo", &[])?.unwrap_or_default(),
            mz: range_of(d, "mz_max", "mz_min")?,
            rt: range_of(d, "rt_max", "rt_min")?,
            scans: number(d, "scansioni")?.map(|x| x as u32),
        });
    }
    Ok(())
}

fn read_column(v: &Value) -> Result<Colonna> {
    let kind = TipoColonna::from_name(&text(v, "tipo")?).ok_or_else(bad)?;
    let mut c = Colonna::new(&text(v, "nome")?, kind);
    c.required = v
        .get("obbligatoria")
        .and_then(|x| x.bool())
        .unwrap_or(false);
    if let Some(a) = v.get("valori") {
        c.values = Some(
            a.arr()
                .iter()
                .map(|x| x.str().map(str::to_string).ok_or_else(bad))
                .collect::<Result<_>>()?,
        );
    }
    Ok(c)
}

/// Reads a `campioni/3` document or migrates a `campioni/2` one (the new fields empty, no columns): files in the order written.
pub fn tabella_da_value(doc: &Value) -> Result<Tabella> {
    let campioni = super::from_value(doc)?;
    let colonne = doc
        .get("colonne")
        .map(|a| a.arr().iter().map(read_column).collect::<Result<_>>())
        .transpose()?
        .unwrap_or_default();
    Ok(Tabella { campioni, colonne })
}

pub fn tabella_da_json(bytes: &[u8]) -> Result<Tabella> {
    tabella_da_value(&parse(bytes)?)
}
