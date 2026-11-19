//! The structural check of a `design/1` document: every bad key by its path (`conditions[1].replicates`) and a stable code, so that an agent can fix
//! the file without guessing. `read` stops at the first fault with one error; this lists them all. No statistics, no samples here.

use super::{read, DesignComparison, SCHEMA};
use crate::json::Value;

/// One fault: `key` is the path of the bad member, `code` one of `missing`, `wrongType`, `badNumber`, `wrongSchema`, `badKind`, `duplicate` or
/// `unknownCondition` (the texts are `err.design.problem.<code>` in `lang/en`).
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub key: String,
    pub code: &'static str,
}

fn push(out: &mut Vec<Problem>, key: String, code: &'static str) {
    out.push(Problem { key, code });
}

/// A member that must be a string.
fn string(v: &Value, path: &str, k: &str, out: &mut Vec<Problem>) {
    match v.get(k) {
        None => push(out, format!("{path}.{k}"), "missing"),
        Some(x) if x.str().is_none() => push(out, format!("{path}.{k}"), "wrongType"),
        Some(_) => {}
    }
}

/// A member that must be an array; the items when it is one.
fn array<'a>(v: &'a Value, key: &str, out: &mut Vec<Problem>) -> &'a [Value] {
    match v.get(key) {
        Some(Value::Arr(a)) => a,
        None => {
            push(out, key.to_string(), "missing");
            &[]
        }
        Some(_) => {
            push(out, key.to_string(), "wrongType");
            &[]
        }
    }
}

fn condition(v: &Value, i: usize, out: &mut Vec<Problem>) {
    let path = format!("conditions[{i}]");
    string(v, &path, "name", out);
    match v.get("replicates") {
        None => push(out, format!("{path}.replicates"), "missing"),
        Some(x) => match x.f64() {
            None => push(out, format!("{path}.replicates"), "wrongType"),
            Some(n) if !(n >= 0.0 && n.fract() == 0.0 && n < 4e9) => {
                push(out, format!("{path}.replicates"), "badNumber");
            }
            Some(_) => {}
        },
    }
    match v.get("levels") {
        None => push(out, format!("{path}.levels"), "missing"),
        Some(Value::Obj(o)) => {
            for (k, x) in o.iter() {
                if x.str().is_none() {
                    push(out, format!("{path}.levels.{k}"), "wrongType");
                }
            }
        }
        Some(_) => push(out, format!("{path}.levels"), "wrongType"),
    }
}

fn comparison(v: &Value, i: usize, out: &mut Vec<Problem>) {
    let path = format!("comparisons[{i}]");
    match v.get("kind").and_then(Value::str) {
        Some("pair") => {
            string(v, &path, "a", out);
            string(v, &path, "b", out);
        }
        Some("vs_reference") => {}
        Some(_) => push(out, format!("{path}.kind"), "badKind"),
        None => push(
            out,
            format!("{path}.kind"),
            if v.get("kind").is_some() {
                "wrongType"
            } else {
                "missing"
            },
        ),
    }
}

fn factor(v: &Value, i: usize, out: &mut Vec<Problem>) {
    let path = format!("factors[{i}]");
    string(v, &path, "name", out);
    match v.get("values") {
        None => push(out, format!("{path}.values"), "missing"),
        Some(Value::Arr(a)) => {
            for (k, x) in a.iter().enumerate() {
                if x.str().is_none() {
                    push(out, format!("{path}.values[{k}]"), "wrongType");
                }
            }
        }
        Some(_) => push(out, format!("{path}.values"), "wrongType"),
    }
}

/// What is wrong with `doc` as a `design/1`, in the order of the document; empty when it reads and its names agree (the reference and the pairs name
/// conditions that exist, no condition twice).
pub fn problems(doc: &Value) -> Vec<Problem> {
    let mut out = Vec::new();
    if !matches!(doc, Value::Obj(_)) {
        push(&mut out, "$".into(), "wrongType");
        return out;
    }
    match doc.get("schema").and_then(Value::str) {
        Some(SCHEMA) => {}
        Some(_) => push(&mut out, "schema".into(), "wrongSchema"),
        None => push(&mut out, "schema".into(), "missing"),
    }
    for (i, c) in array(doc, "conditions", &mut out).iter().enumerate() {
        condition(c, i, &mut out);
    }
    match doc.get("reference") {
        None => push(&mut out, "reference".into(), "missing"),
        Some(Value::Null) => {}
        Some(x) if x.str().is_none() => push(&mut out, "reference".into(), "wrongType"),
        Some(_) => {}
    }
    for (i, c) in array(doc, "comparisons", &mut out).iter().enumerate() {
        comparison(c, i, &mut out);
    }
    for (i, f) in array(doc, "factors", &mut out).iter().enumerate() {
        factor(f, i, &mut out);
    }
    for (i, b) in array(doc, "batches", &mut out).iter().enumerate() {
        for k in ["file", "sha256", "batch"] {
            string(b, &format!("batches[{i}]"), k, &mut out);
        }
    }
    if !out.is_empty() {
        return out;
    }
    let Ok(d) = read(doc) else {
        push(&mut out, "$".into(), "wrongType");
        return out;
    };
    let known = |n: &str| d.conditions.iter().any(|c| c.name == n);
    for (i, c) in d.conditions.iter().enumerate() {
        if d.conditions[..i].iter().any(|p| p.name == c.name) {
            push(&mut out, format!("conditions[{i}].name"), "duplicate");
        }
    }
    if d.reference.as_deref().is_some_and(|r| !known(r)) {
        push(&mut out, "reference".into(), "unknownCondition");
    }
    for (i, c) in d.comparisons.iter().enumerate() {
        if let DesignComparison::Pair(a, b) = c {
            for (k, n) in [("a", a), ("b", b)] {
                if !known(n) {
                    push(
                        &mut out,
                        format!("comparisons[{i}].{k}"),
                        "unknownCondition",
                    );
                }
            }
        }
    }
    out
}
