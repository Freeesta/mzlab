//! The bindings of the molecules (`api/molecules/<op>`): JSON in, JSON out (`to_csv` gives the CSV text). `load` leaves the catalogue and the molecules of the
//! user in the engine; `search` and `get` read them; the others are pure conversions of a CSV or of a `molecules/1` document.

use super::levels::con;
use super::{
    digest, errors_json, from_csv, from_json, hits_json, ion_of, record_json, search, search_wide,
    to_csv, to_json, Molecule, RowError,
};
use crate::elements::{adducts, ion_mz};
use crate::error::{Error, Result};
use crate::json::{parse, round, Out, Value};

fn errors_doc(e: &[RowError]) -> String {
    format!("{{\"errors\":{}}}", errors_json(e))
}

/// Records from a document given as JSON text or as an object.
fn doc_of(v: &Value) -> std::result::Result<Vec<Molecule>, Vec<RowError>> {
    match v {
        Value::Str(s) => from_json(s.as_bytes()),
        Value::Null => Ok(Vec::new()),
        o => super::json::from_value(o),
    }
}

fn load(v: &Value) -> Result<String> {
    let cat = match v.get("catalog").and_then(|c| c.str()) {
        Some(t) => match from_csv(t, "catalog", None) {
            Ok(mut ms) => {
                ms.iter_mut().for_each(|m| m.origin.kind = "catalog".into());
                Some(ms)
            }
            Err(e) => return Ok(errors_doc(&e)),
        },
        None => None,
    };
    let others = match v.get("user").map(doc_of) {
        Some(Err(e)) => return Ok(errors_doc(&e)),
        Some(Ok(ms)) => Some(ms),
        None => None,
    };
    Ok(con(|a| {
        a.catalog = cat.unwrap_or(std::mem::take(&mut a.catalog));
        a.others = others.unwrap_or(std::mem::take(&mut a.others));
        a.update();
        format!(
            "{{\"others\":{},\"catalog\":{},\"errors\":[],\"total\":{}}}",
            a.others.len(),
            a.catalog.len(),
            a.all.len()
        )
    }))
}

fn search_hits(v: &Value) -> String {
    let q = v.get("q").and_then(|x| x.str()).unwrap_or("");
    let limit = v
        .get("limit")
        .and_then(|x| x.f64())
        .map_or(8, |x| x as usize);
    let ppm = v.get("ppm").and_then(|x| x.f64()).unwrap_or(10.0);
    let wide = v.get("wide").is_some_and(|x| x.bool() == Some(true));
    con(|a| {
        let hits = if wide {
            search_wide(&a.all, q, limit, ppm)
        } else {
            search(&a.all, q, limit, ppm)
        };
        hits_json(&a.all, &hits)
    })
}

fn one(v: &Value) -> String {
    let id = v.get("id").and_then(|x| x.str()).unwrap_or("");
    con(|a| {
        a.all
            .iter()
            .find(|m| m.id == id)
            .map_or("null".to_string(), record_json)
    })
}

/// `{"catalog": "hrms"}` (the catalogue it comes from), `{"collection": name}` (a collection) or `{"molecules": document}` (records that are not in the archive,
/// e.g. a file just read) plus `{"adducts": [..]}`: the molecules of that set with the m/z of every adduct asked for (`null` where the adduct is unknown).
fn list(v: &Value) -> std::result::Result<String, Vec<RowError>> {
    let want = |k: &str| v.get(k).and_then(|x| x.str()).filter(|s| !s.is_empty());
    let (cat, col) = (want("catalog"), want("collection"));
    let ads: Vec<&str> = v
        .get("adducts")
        .map(|a| a.arr())
        .unwrap_or(&[])
        .iter()
        .filter_map(|x| x.str())
        .collect();
    let own = v.get("molecules").map(doc_of).transpose()?;
    let pick = |all: &[Molecule]| -> String {
        let mut j = Out::default();
        j.raw("[");
        let sel = all.iter().filter(|m| {
            own.is_some()
                || (cat.is_some() || col.is_some())
                    && cat.is_none_or(|c| m.origin.catalog.iter().any(|x| x == c))
                    && col.is_none_or(|c| m.collections.iter().any(|x| x == c))
        });
        for (i, m) in sel.enumerate() {
            j.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" });
            j.str(&m.id).raw(",\"name\":").str(&m.name);
            j.raw(",\"synonyms\":[");
            for (k, s) in m.synonyms.iter().enumerate() {
                j.raw(if k > 0 { "," } else { "" }).str(s);
            }
            j.raw("],\"formula\":").str(&m.formula);
            j.raw(",\"mass\":").num(m.mass).raw(",\"mz\":[");
            for (k, ad) in ads.iter().enumerate() {
                j.raw(if k > 0 { "," } else { "" });
                j.opt(ion_mz(m.mass, ad).map(|x| round(x, 6)));
            }
            j.raw("]}");
        }
        j.raw("]");
        j.0
    };
    Ok(match &own {
        Some(ms) => pick(ms),
        None => con(|a| pick(&a.all)),
    })
}

fn bad(op: &str) -> Error {
    Error::new("error.badMolecules").with("op", op)
}

/// `op` is `load` (`{"catalog": csv, "user": document}`, each optional: what is missing stays), `search` (`{"q","limit","ppm","wide"}`), `ion` (`{"q","adduct"}`), `adducts` (the names of the adduct table), `get` (`{"id"}`), `list` (`{"collection","adducts"}`),
/// `from_csv` (`{"csv","file","level","collection"}` to a document), `validate` (`{"csv"}`), or, on a `molecules/1` document, `to_csv`, `to_json`, `digest`.
pub fn route(op: &str, input: &str) -> Result<String> {
    let v = || parse(input.as_bytes()).map_err(|_| bad(op));
    let docs = || from_json(input.as_bytes());
    match op {
        "load" => load(&v()?),
        "search" => Ok(search_hits(&v()?)),
        "get" => Ok(one(&v()?)),
        "adducts" => {
            let mut j = Out::default();
            j.raw("[");
            for (i, (a, _)) in adducts().iter().enumerate() {
                j.raw(if i > 0 { "," } else { "" }).str(a);
            }
            Ok(j.raw("]").0.clone())
        }
        "ion" => {
            let v = v()?;
            let s = |k: &str| v.get(k).and_then(|x| x.str()).unwrap_or("");
            ion_of(s("q"), s("adduct"))
        }
        "list" => Ok(list(&v()?).unwrap_or_else(|e| errors_doc(&e))),
        "from_csv" | "validate" => {
            let v = v()?;
            let s = |k: &str| v.get(k).and_then(|x| x.str());
            let kind = s("level").filter(|l| !l.is_empty()).unwrap_or("imported");
            Ok(match from_csv(s("csv").unwrap_or(""), kind, s("file")) {
                Ok(mut ms) if op == "from_csv" => {
                    if let Some(c) = s("collection").filter(|c| !c.is_empty()) {
                        for m in &mut ms {
                            if !m.collections.iter().any(|x| x == c) {
                                m.collections.push(c.to_string());
                            }
                        }
                    }
                    to_json(&ms)
                }
                Ok(ms) => format!("{{\"errors\":[],\"n\":{}}}", ms.len()),
                Err(e) => errors_doc(&e),
            })
        }
        "to_csv" => docs()
            .map(|ms| to_csv(&ms))
            .map_err(|_| Error::new("error.molecules.badJson")),
        "to_json" => Ok(docs().map_or_else(|e| errors_doc(&e), |ms| to_json(&ms))),
        "digest" => docs()
            .map(|ms| digest(&ms))
            .map_err(|_| Error::new("error.molecules.badJson")),
        _ => Err(bad(op)),
    }
}
