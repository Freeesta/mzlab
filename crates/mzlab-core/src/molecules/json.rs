//! The canonical JSON of `molecules/1` (keys in alphabetical order, records sorted by `id`, lists tidy: the same molecules give the same bytes and the same
//! SHA-256), its reader, and the JSON of the answers (errors, search hits).

use super::{finalize, repeated, Molecule, Origin, Property, RowError, SCHEMA};
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

fn origin(j: &mut Out, o: &Origin) {
    j.raw("{\"catalog\":");
    strs(j, &o.catalog);
    j.raw(",\"created\":");
    opt_str(j, &o.created);
    j.raw(",\"file\":");
    opt_str(j, &o.file);
    j.raw(",\"modified\":");
    opt_str(j, &o.modified);
    j.raw(",\"type\":").str(&o.kind).raw("}");
}

fn property(j: &mut Out, p: &Property) {
    j.raw("{\"conditions\":")
        .str(&p.conditions)
        .raw(",\"key\":")
        .str(&p.key);
    j.raw(",\"source\":")
        .str(&p.provenance)
        .raw(",\"unit\":")
        .str(&p.unit);
    j.raw(",\"value\":").num(p.value).raw("}");
}

fn externals(j: &mut Out, m: &Molecule) {
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
pub fn record_json(m: &Molecule) -> String {
    let mut j = Out::default();
    j.raw("{\"adducts\":");
    strs(&mut j, &m.adducts);
    j.raw(",\"charge\":")
        .int(i64::from(m.load))
        .raw(",\"classes\":");
    strs(&mut j, &m.classes);
    j.raw(",\"collections\":");
    strs(&mut j, &m.collections);
    j.raw(",\"external\":");
    externals(&mut j, m);
    j.raw(",\"formula\":")
        .str(&m.formula)
        .raw(",\"id\":")
        .str(&m.id)
        .raw(",\"inchikey\":");
    opt_str(&mut j, &m.inchikey);
    j.raw(",\"mass\":")
        .num(m.mass)
        .raw(",\"name\":")
        .str(&m.name)
        .raw(",\"notes\":")
        .str(&m.notes);
    j.raw(",\"origin\":");
    origin(&mut j, &m.origin);
    j.raw(",\"properties\":[");
    for (i, p) in m.properties.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" });
        property(&mut j, p);
    }
    j.raw("],\"smiles\":");
    opt_str(&mut j, &m.smiles);
    j.raw(",\"synonyms\":");
    strs(&mut j, &m.synonyms);
    j.raw("}");
    j.0
}

/// `{"schema":"molecules/1","molecules":[...]}` (`schema` first, then alphabetical), the records sorted by `id`.
pub fn to_json(ms: &[Molecule]) -> String {
    let mut sorted: Vec<&Molecule> = ms.iter().collect();
    crate::sortc::slice_by(&mut sorted, |a, b| a.id.cmp(&b.id));
    let mut j = Out::default();
    j.raw("{\"schema\":").str(SCHEMA).raw(",\"molecules\":[");
    for (i, m) in sorted.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).raw(&record_json(m));
    }
    j.raw("]}");
    j.0
}

/// SHA-256 of the canonical JSON (what `prov/1` records when a step uses the molecules).
pub fn digest(ms: &[Molecule]) -> String {
    sha256_hex(to_json(ms).as_bytes())
}

/// `[{"key":..,"line":..,"text":..}]`.
pub fn errors_json(errs: &[RowError]) -> String {
    let mut j = Out::default();
    j.raw("[");
    for (i, e) in errs.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" })
            .raw("{\"key\":")
            .str(e.key);
        j.raw(",\"line\":")
            .int(e.line as i64)
            .raw(",\"text\":")
            .str(&e.detail)
            .raw("}");
    }
    j.raw("]");
    j.0
}

/// The search answer: `[{id, name, formula, charge, mass, smiles, adducts, score}]`.
pub fn hits_json(ms: &[Molecule], hits: &[(usize, f64)]) -> String {
    let mut j = Out::default();
    j.raw("[");
    for (i, &(k, score)) in hits.iter().enumerate() {
        let m = &ms[k];
        j.raw(if i > 0 { "," } else { "" }).raw("{\"adducts\":");
        strs(&mut j, &m.adducts);
        j.raw(",\"charge\":")
            .int(i64::from(m.load))
            .raw(",\"formula\":")
            .str(&m.formula);
        j.raw(",\"id\":")
            .str(&m.id)
            .raw(",\"mass\":")
            .num(m.mass)
            .raw(",\"name\":")
            .str(&m.name);
        j.raw(",\"score\":").num(score).raw(",\"smiles\":");
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

fn read_properties(v: &Value) -> Vec<Property> {
    let n = |p: &Value| p.get("value").and_then(|x| x.f64()).unwrap_or(f64::NAN);
    let a = v.get("properties").map(|a| a.arr()).unwrap_or(&[]);
    a.iter()
        .map(|p| Property {
            key: text(p, "key"),
            value: n(p),
            unit: text(p, "unit"),
            provenance: text(p, "source"),
            conditions: text(p, "conditions"),
        })
        .collect()
}

/// One record from its JSON; `mass` is ignored (computed again) unless the record has no formula (a mass-only entry). A charge that is not a whole number is refused.
fn read_one(v: &Value) -> Result<Molecule, (&'static str, String)> {
    let charge = v.get("charge").and_then(|x| x.f64()).unwrap_or(0.0);
    if charge.fract() != 0.0 || charge.abs() > 1e6 {
        return Err(("error.molecules.badCharge", charge.to_string()));
    }
    // a record saved before the keys were English (`nome`): refused with its own message, never read as a record without a name
    if v.get("name").is_none() && v.get("nome").is_some() {
        return Err(("error.molecules.oldFormat", String::new()));
    }
    let o = v.get("origin").cloned().unwrap_or(Value::Null);
    let ext = v.get("external").cloned().unwrap_or(Value::Null);
    let mut m = Molecule {
        id: text(v, "id"),
        name: text(v, "name"),
        synonyms: list(v, "synonyms"),
        formula: text(v, "formula"),
        load: charge as i32,
        smiles: opt(v, "smiles"),
        inchikey: opt(v, "inchikey"),
        adducts: list(v, "adducts"),
        collections: list(v, "collections"),
        classes: list(v, "classes"),
        cas: opt(&ext, "cas"),
        pubchem_cid: opt(&ext, "pubchem_cid"),
        properties: read_properties(v),
        notes: text(v, "notes"),
        ..Molecule::default()
    };
    m.origin = Origin {
        kind: text(&o, "type"),
        catalog: list(&o, "catalog"),
        file: opt(&o, "file"),
        created: opt(&o, "created"),
        modified: opt(&o, "modified"),
    };
    let control = v
        .get("mass")
        .and_then(|x| x.f64())
        .filter(|_| m.formula.trim().is_empty());
    finalize(&mut m, control)?;
    Ok(m)
}

/// The records of a parsed `molecules/1` document, or an error for every wrong record (its 1-based position) or repeated `id`.
pub fn from_value(doc: &Value) -> Result<Vec<Molecule>, Vec<RowError>> {
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
    // a document without the `molecules` list is not this format (an archive written before the keys were English): refused, never read as empty
    let Some(list) = doc.get("molecules").map(|a| a.arr()) else {
        let old = doc.get("molecole").is_some();
        return Err(vec![RowError {
            line: 0,
            key: if old {
                "error.molecules.oldFormat"
            } else {
                "error.molecules.badJson"
            },
            detail: if old {
                String::new()
            } else {
                "molecules".into()
            },
        }]);
    };
    let (mut out, mut errors) = (Vec::new(), Vec::new());
    for (i, v) in list.iter().enumerate() {
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
pub fn from_json(bytes: &[u8]) -> Result<Vec<Molecule>, Vec<RowError>> {
    match parse(bytes) {
        Ok(doc) => from_value(&doc),
        Err(_) => Err(vec![RowError {
            line: 0,
            key: "error.molecules.badJson",
            detail: "json".into(),
        }]),
    }
}
