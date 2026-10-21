//! The canonical JSON of `molecules/1` (keys in alphabetical order, records sorted by `id`, lists tidy: the same molecules give the same bytes and the same
//! SHA-256), its reader, and the JSON of the answers (errors, search hits).

use super::{finalizza, repeated, Molecola, Origine, Proprieta, RowError, SCHEMA};
use crate::budget::sha256_hex;
use crate::json::{parse, Out, Value};

fn strs(j: &mut Out, v: &[String]) {
    j.raw("[");
    for (i, s) in v.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).str(s);
    }
    j.raw("]");
}

fn opt_str(j: &mut Out, v: &Option<String>) {
    match v {
        Some(s) => j.str(s),
        None => j.raw("null"),
    };
}

fn origin(j: &mut Out, o: &Origine) {
    j.raw("{\"catalogo\":");
    strs(j, &o.catalogo);
    j.raw(",\"creato\":");
    opt_str(j, &o.creato);
    j.raw(",\"file\":");
    opt_str(j, &o.file);
    j.raw(",\"modificato\":");
    opt_str(j, &o.modificato);
    j.raw(",\"tipo\":").str(&o.tipo).raw("}");
}

fn property(j: &mut Out, p: &Proprieta) {
    j.raw("{\"chiave\":")
        .str(&p.chiave)
        .raw(",\"condizioni\":")
        .str(&p.condizioni);
    j.raw(",\"fonte\":")
        .str(&p.fonte)
        .raw(",\"unita\":")
        .str(&p.unita);
    j.raw(",\"valore\":").num(p.valore).raw("}");
}

fn externals(j: &mut Out, m: &Molecola) {
    j.raw("{");
    if let Some(c) = &m.cas {
        j.raw("\"cas\":").str(c);
    }
    if let Some(c) = &m.pubchem_cid {
        j.raw(if m.cas.is_some() { "," } else { "" })
            .raw("\"pubchem_cid\":")
            .str(c);
    }
    j.raw("}");
}

/// One record as canonical JSON.
pub fn record_json(m: &Molecola) -> String {
    let mut j = Out::default();
    j.raw("{\"addotti\":");
    strs(&mut j, &m.addotti);
    j.raw(",\"carica\":")
        .int(i64::from(m.carica))
        .raw(",\"classi\":");
    strs(&mut j, &m.classi);
    j.raw(",\"esterni\":");
    externals(&mut j, m);
    j.raw(",\"formula\":")
        .str(&m.formula)
        .raw(",\"id\":")
        .str(&m.id)
        .raw(",\"inchikey\":");
    opt_str(&mut j, &m.inchikey);
    j.raw(",\"massa\":")
        .num(m.massa)
        .raw(",\"nome\":")
        .str(&m.nome)
        .raw(",\"note\":")
        .str(&m.note);
    j.raw(",\"origine\":");
    origin(&mut j, &m.origine);
    j.raw(",\"proprieta\":[");
    for (i, p) in m.proprieta.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" });
        property(&mut j, p);
    }
    j.raw("],\"raccolte\":");
    strs(&mut j, &m.raccolte);
    j.raw(",\"sinonimi\":");
    strs(&mut j, &m.sinonimi);
    j.raw(",\"smiles\":");
    opt_str(&mut j, &m.smiles);
    j.raw("}");
    j.0
}

/// `{"molecole":[...],"schema":"molecules/1"}`, the records sorted by `id`.
pub fn to_json(ms: &[Molecola]) -> String {
    let mut sorted: Vec<&Molecola> = ms.iter().collect();
    crate::sortc::slice_by(&mut sorted, |a, b| a.id.cmp(&b.id));
    let mut j = Out::default();
    j.raw("{\"molecole\":[");
    for (i, m) in sorted.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).raw(&record_json(m));
    }
    j.raw("],\"schema\":").str(SCHEMA).raw("}");
    j.0
}

/// SHA-256 of the canonical JSON (what `prov/1` records when a step uses the molecules).
pub fn digest(ms: &[Molecola]) -> String {
    sha256_hex(to_json(ms).as_bytes())
}

/// `[{"riga":..,"chiave":..,"testo":..}]`.
pub fn errors_json(errs: &[RowError]) -> String {
    let mut j = Out::default();
    j.raw("[");
    for (i, e) in errs.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" })
            .raw("{\"chiave\":")
            .str(e.key);
        j.raw(",\"riga\":")
            .int(e.line as i64)
            .raw(",\"testo\":")
            .str(&e.detail)
            .raw("}");
    }
    j.raw("]");
    j.0
}

/// The search answer: `[{id, nome, formula, carica, massa, smiles, addotti, punteggio}]`.
pub fn hits_json(ms: &[Molecola], hits: &[(usize, f64)]) -> String {
    let mut j = Out::default();
    j.raw("[");
    for (i, &(k, score)) in hits.iter().enumerate() {
        let m = &ms[k];
        j.raw(if i > 0 { "," } else { "" }).raw("{\"addotti\":");
        strs(&mut j, &m.addotti);
        j.raw(",\"carica\":")
            .int(i64::from(m.carica))
            .raw(",\"formula\":")
            .str(&m.formula);
        j.raw(",\"id\":")
            .str(&m.id)
            .raw(",\"massa\":")
            .num(m.massa)
            .raw(",\"nome\":")
            .str(&m.nome);
        j.raw(",\"punteggio\":").num(score).raw(",\"smiles\":");
        opt_str(&mut j, &m.smiles);
        j.raw("}");
    }
    j.raw("]");
    j.0
}

fn text(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.str()).unwrap_or("").to_string()
}

fn opt(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.str()).map(str::to_string)
}

fn list(v: &Value, k: &str) -> Vec<String> {
    v.get(k)
        .map(|a| a.arr())
        .unwrap_or(&[])
        .iter()
        .filter_map(|x| x.str())
        .map(str::to_string)
        .collect()
}

fn read_properties(v: &Value) -> Vec<Proprieta> {
    let n = |p: &Value| p.get("valore").and_then(|x| x.f64()).unwrap_or(f64::NAN);
    let a = v.get("proprieta").map(|a| a.arr()).unwrap_or(&[]);
    a.iter()
        .map(|p| Proprieta {
            chiave: text(p, "chiave"),
            valore: n(p),
            unita: text(p, "unita"),
            fonte: text(p, "fonte"),
            condizioni: text(p, "condizioni"),
        })
        .collect()
}

/// One record from its JSON; `massa` is ignored (computed again) unless the record has no formula (a mass-only entry). A charge that is not a whole number is refused.
fn read_one(v: &Value) -> Result<Molecola, (&'static str, String)> {
    let charge = v.get("carica").and_then(|x| x.f64()).unwrap_or(0.0);
    if charge.fract() != 0.0 || charge.abs() > 1e6 {
        return Err(("error.molecules.badCharge", charge.to_string()));
    }
    let o = v.get("origine").cloned().unwrap_or(Value::Null);
    let ext = v.get("esterni").cloned().unwrap_or(Value::Null);
    let mut m = Molecola {
        id: text(v, "id"),
        nome: text(v, "nome"),
        sinonimi: list(v, "sinonimi"),
        formula: text(v, "formula"),
        carica: charge as i32,
        smiles: opt(v, "smiles"),
        inchikey: opt(v, "inchikey"),
        addotti: list(v, "addotti"),
        raccolte: list(v, "raccolte"),
        classi: list(v, "classi"),
        cas: opt(&ext, "cas"),
        pubchem_cid: opt(&ext, "pubchem_cid"),
        proprieta: read_properties(v),
        note: text(v, "note"),
        ..Molecola::default()
    };
    m.origine = Origine {
        tipo: text(&o, "tipo"),
        catalogo: list(&o, "catalogo"),
        file: opt(&o, "file"),
        creato: opt(&o, "creato"),
        modificato: opt(&o, "modificato"),
    };
    let control = v
        .get("massa")
        .and_then(|x| x.f64())
        .filter(|_| m.formula.trim().is_empty());
    finalizza(&mut m, control)?;
    Ok(m)
}

/// The records of a parsed `molecules/1` document, or an error for every wrong record (its 1-based position) or repeated `id`.
pub fn from_value(doc: &Value) -> Result<Vec<Molecola>, Vec<RowError>> {
    let bad = |detail: &str| {
        vec![RowError {
            line: 0,
            key: "error.molecules.badJson",
            detail: detail.to_string(),
        }]
    };
    if doc.get("schema").and_then(|v| v.str()) != Some(SCHEMA) {
        return Err(bad("schema"));
    }
    let (mut out, mut errors) = (Vec::new(), Vec::new());
    for (i, v) in doc
        .get("molecole")
        .map(|a| a.arr())
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        match read_one(v) {
            Ok(m) => out.push((i + 1, m)),
            Err((key, detail)) => errors.push(RowError {
                line: i + 1,
                key,
                detail,
            }),
        }
    }
    repeated(&out, &mut errors);
    if errors.is_empty() {
        Ok(out.into_iter().map(|r| r.1).collect())
    } else {
        Err(errors)
    }
}

/// Reads a `molecules/1` document (see `from_value`).
pub fn from_json(bytes: &[u8]) -> Result<Vec<Molecola>, Vec<RowError>> {
    match parse(bytes) {
        Ok(doc) => from_value(&doc),
        Err(_) => Err(vec![RowError {
            line: 0,
            key: "error.molecules.badJson",
            detail: "json".into(),
        }]),
    }
}
