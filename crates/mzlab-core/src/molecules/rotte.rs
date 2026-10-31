//! The bindings of the molecules (`api/molecules/<op>`): JSON in, JSON out (`a_csv` gives the CSV text). `carica` leaves the catalogue and the molecules of the
//! user in the engine; `cerca` and `get` read them; the others are pure conversions of a CSV or of a `molecules/1` document.

use super::livelli::con;
use super::{
    cerca, cerca_ampia, digest, errors_json, from_csv, from_json, hits_json, ione, record_json,
    to_csv, to_json, Molecola, RowError,
};
use crate::elements::{adducts, ion_mz};
use crate::error::{Error, Result};
use crate::json::{parse, round, Out, Value};

fn errors_doc(e: &[RowError]) -> String {
    format!("{{\"errori\":{}}}", errors_json(e))
}

/// Records from a document given as JSON text or as an object.
fn doc_of(v: &Value) -> std::result::Result<Vec<Molecola>, Vec<RowError>> {
    match v {
        Value::Str(s) => from_json(s.as_bytes()),
        Value::Null => Ok(Vec::new()),
        o => super::json::from_value(o),
    }
}

fn carica(v: &Value) -> Result<String> {
    let cat = match v.get("catalogo").and_then(|c| c.str()) {
        Some(t) => match from_csv(t, "catalogo", None) {
            Ok(mut ms) => {
                ms.iter_mut()
                    .for_each(|m| m.origine.tipo = "catalogo".into());
                Some(ms)
            }
            Err(e) => return Ok(errors_doc(&e)),
        },
        None => None,
    };
    let altri = match v.get("utente").map(doc_of) {
        Some(Err(e)) => return Ok(errors_doc(&e)),
        Some(Ok(ms)) => Some(ms),
        None => None,
    };
    Ok(con(|a| {
        a.catalogo = cat.unwrap_or(std::mem::take(&mut a.catalogo));
        a.altri = altri.unwrap_or(std::mem::take(&mut a.altri));
        a.aggiorna();
        format!(
            "{{\"altri\":{},\"catalogo\":{},\"errori\":[],\"totale\":{}}}",
            a.altri.len(),
            a.catalogo.len(),
            a.tutte.len()
        )
    }))
}

fn search(v: &Value) -> String {
    let q = v.get("q").and_then(|x| x.str()).unwrap_or("");
    let limite = v
        .get("limite")
        .and_then(|x| x.f64())
        .map_or(8, |x| x as usize);
    let ppm = v.get("ppm").and_then(|x| x.f64()).unwrap_or(10.0);
    let ampia = v.get("ampia").is_some_and(|x| x.bool() == Some(true));
    con(|a| {
        let hits = if ampia {
            cerca_ampia(&a.tutte, q, limite, ppm)
        } else {
            cerca(&a.tutte, q, limite, ppm)
        };
        hits_json(&a.tutte, &hits)
    })
}

fn one(v: &Value) -> String {
    let id = v.get("id").and_then(|x| x.str()).unwrap_or("");
    con(|a| {
        a.tutte
            .iter()
            .find(|m| m.id == id)
            .map_or("null".to_string(), record_json)
    })
}

/// `{"catalogo": "hrms"}` (the catalogue it comes from), `{"raccolta": name}` (a collection) or `{"molecole": document}` (records that are not in the archive,
/// e.g. a file just read) plus `{"addotti": [..]}`: the molecules of that set with the m/z of every adduct asked for (`null` where the adduct is unknown).
fn list(v: &Value) -> std::result::Result<String, Vec<RowError>> {
    let want = |k: &str| v.get(k).and_then(|x| x.str()).filter(|s| !s.is_empty());
    let (cat, col) = (want("catalogo"), want("raccolta"));
    let ads: Vec<&str> = v
        .get("addotti")
        .map(|a| a.arr())
        .unwrap_or(&[])
        .iter()
        .filter_map(|x| x.str())
        .collect();
    let own = v.get("molecole").map(doc_of).transpose()?;
    let pick = |all: &[Molecola]| -> String {
        let mut j = Out::default();
        j.raw("[");
        let sel = all.iter().filter(|m| {
            own.is_some()
                || (cat.is_some() || col.is_some())
                    && cat.is_none_or(|c| m.origine.catalogo.iter().any(|x| x == c))
                    && col.is_none_or(|c| m.raccolte.iter().any(|x| x == c))
        });
        for (i, m) in sel.enumerate() {
            j.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" });
            j.str(&m.id).raw(",\"nome\":").str(&m.nome);
            j.raw(",\"sinonimi\":[");
            for (k, s) in m.sinonimi.iter().enumerate() {
                j.raw(if k > 0 { "," } else { "" }).str(s);
            }
            j.raw("],\"formula\":").str(&m.formula);
            j.raw(",\"massa\":").num(m.massa).raw(",\"mz\":[");
            for (k, ad) in ads.iter().enumerate() {
                j.raw(if k > 0 { "," } else { "" });
                j.opt(ion_mz(m.massa, ad).map(|x| round(x, 6)));
            }
            j.raw("]}");
        }
        j.raw("]");
        j.0
    };
    Ok(match &own {
        Some(ms) => pick(ms),
        None => con(|a| pick(&a.tutte)),
    })
}

fn bad(op: &str) -> Error {
    Error::new("error.badMolecules").with("op", op)
}

/// `op` is `carica` (`{"catalogo": csv, "utente": document}`, each optional: what is missing stays), `cerca` (`{"q","limite","ppm","ampia"}`), `ione` (`{"q","adduct"}`), `addotti` (the names of the adduct table), `get` (`{"id"}`), `elenca` (`{"catalogo"|"raccolta","addotti"}`),
/// `da_csv` (`{"csv","file","livello","raccolta"}` to a document), `valida` (`{"csv"}`), or, on a `molecules/1` document, `a_csv`, `a_json`, `digest`.
pub fn route(op: &str, input: &str) -> Result<String> {
    let v = || parse(input.as_bytes()).map_err(|_| bad(op));
    let docs = || from_json(input.as_bytes());
    match op {
        "carica" => carica(&v()?),
        "cerca" => Ok(search(&v()?)),
        "get" => Ok(one(&v()?)),
        "addotti" => {
            let mut j = Out::default();
            j.raw("[");
            for (i, (a, _)) in adducts().iter().enumerate() {
                j.raw(if i > 0 { "," } else { "" }).str(a);
            }
            Ok(j.raw("]").0.clone())
        }
        "ione" => {
            let v = v()?;
            let s = |k: &str| v.get(k).and_then(|x| x.str()).unwrap_or("");
            ione(s("q"), s("adduct"))
        }
        "elenca" => Ok(list(&v()?).unwrap_or_else(|e| errors_doc(&e))),
        "da_csv" | "valida" => {
            let v = v()?;
            let s = |k: &str| v.get(k).and_then(|x| x.str());
            let tipo = s("livello")
                .filter(|l| !l.is_empty())
                .unwrap_or("importata");
            Ok(match from_csv(s("csv").unwrap_or(""), tipo, s("file")) {
                Ok(mut ms) if op == "da_csv" => {
                    if let Some(c) = s("raccolta").filter(|c| !c.is_empty()) {
                        for m in &mut ms {
                            if !m.raccolte.iter().any(|x| x == c) {
                                m.raccolte.push(c.to_string());
                            }
                        }
                    }
                    to_json(&ms)
                }
                Ok(ms) => format!("{{\"errori\":[],\"n\":{}}}", ms.len()),
                Err(e) => errors_doc(&e),
            })
        }
        "a_csv" => docs()
            .map(|ms| to_csv(&ms))
            .map_err(|_| Error::new("error.molecules.badJson")),
        "a_json" => Ok(docs().map_or_else(|e| errors_doc(&e), |ms| to_json(&ms))),
        "digest" => docs()
            .map(|ms| digest(&ms))
            .map_err(|_| Error::new("error.molecules.badJson")),
        _ => Err(bad(op)),
    }
}
