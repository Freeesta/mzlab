//! The bindings of the molecules (`api/molecules/<op>`): JSON in, JSON out (`a_csv` gives the CSV text). `carica` leaves the catalogue and the molecules of the
//! user in the engine; `cerca` and `get` read them; the others are pure conversions of a CSV or of a `molecules/1` document.

use super::livelli::con;
use super::{
    cerca, digest, errors_json, from_csv, from_json, hits_json, record_json, to_csv, to_json,
    Molecola, RowError,
};
use crate::error::{Error, Result};
use crate::json::{parse, Value};

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
    con(|a| hits_json(&a.tutte, &cerca(&a.tutte, q, limite, ppm)))
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

fn bad(op: &str) -> Error {
    Error::new("error.badMolecules").with("op", op)
}

/// `op` is `carica` (`{"catalogo": csv, "utente": document}`, each optional: what is missing stays), `cerca` (`{"q","limite","ppm"}`), `get` (`{"id"}`),
/// `da_csv` (`{"csv","file","livello"}` to a document), `valida` (`{"csv"}`), or, on a `molecules/1` document, `a_csv`, `a_json`, `digest`.
pub fn route(op: &str, input: &str) -> Result<String> {
    let v = || parse(input.as_bytes()).map_err(|_| bad(op));
    let docs = || from_json(input.as_bytes());
    match op {
        "carica" => carica(&v()?),
        "cerca" => Ok(search(&v()?)),
        "get" => Ok(one(&v()?)),
        "da_csv" | "valida" => {
            let v = v()?;
            let s = |k: &str| v.get(k).and_then(|x| x.str());
            let tipo = s("livello")
                .filter(|l| !l.is_empty())
                .unwrap_or("importata");
            Ok(match from_csv(s("csv").unwrap_or(""), tipo, s("file")) {
                Ok(ms) if op == "da_csv" => to_json(&ms),
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
