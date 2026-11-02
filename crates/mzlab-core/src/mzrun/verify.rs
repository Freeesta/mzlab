//! `verify`: the fingerprints of a written `mzrun/1` record against its own content: the file, the plan, the digest of the results and the entities
//! the `prov/1` names. It does not run anything and does not read the schema (`tests/test_mzrun_schema.py` does).

use super::{results_digest, self_sha256};
use crate::budget::sha256_hex;
use crate::error::{Error, Result};
use crate::flow::value_text;
use crate::json::{parse, Value};

/// What did not match; empty `problems` = the record is what it says.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub self_sha256: String,
    pub problems: Vec<String>,
}

fn field<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::str)
}

/// Parses `text` and checks `self_sha256`, `plan_sha256`, `results_digest` and, for every entity the `prov/1` also holds, its hash. A file of a newer
/// schema is `error.newerFormat`; anything that is not an `mzrun/1` object is `err.mzrun.read`.
pub fn verify(text: &[u8]) -> Result<Verdict> {
    let v = parse(text).map_err(|_| Error::new("err.mzrun.read"))?;
    if field(&v, "schema") != Some("mzrun/1") {
        return Err(Error::new("err.mzrun.read"));
    }
    if v.get("min_reader")
        .and_then(Value::f64)
        .is_none_or(|n| n > 1.0)
    {
        return Err(Error::new("error.newerFormat"));
    }
    let mut problems = Vec::new();
    let got = self_sha256(v.entries());
    if field(&v, "self_sha256") != Some(&got) {
        problems.push("self_sha256".to_string());
    }
    let plan = v.get("plan").ok_or_else(|| Error::new("err.mzrun.read"))?;
    if field(&v, "plan_sha256") != Some(&sha256_hex(value_text(plan).as_bytes())) {
        problems.push("plan_sha256".to_string());
    }
    if let Some(r) = v.get("results") {
        let ents: Vec<(String, String)> = r
            .get("entities")
            .map_or(&[][..], Value::entries)
            .iter()
            .map(|(k, h)| (k.clone(), h.str().unwrap_or("").to_string()))
            .collect();
        if field(r, "results_digest") != Some(&results_digest(&ents)) {
            problems.push("results_digest".to_string());
        }
        let prov = v.get("prov").and_then(|p| p.get("entity"));
        for (name, sha) in &ents {
            // a prov/1 without that entity (a trimmed one) has nothing to disagree with
            let held = prov.and_then(|e| e.get(name)).map(|e| field(e, "sha256"));
            if held.is_some_and(|h| h != Some(sha)) {
                problems.push(format!("entity:{name}"));
            }
        }
    }
    Ok(Verdict {
        self_sha256: got,
        problems,
    })
}
