//! A small JSON Schema checker for tool arguments: the keywords the catalog `strumenti/1` uses (type, enum, const, required, properties,
//! additionalProperties, items, oneOf, anyOf, bounds, array sizes). Used by the policy and the agent; no dependency.

use mzlab_core::json::Value;

fn type_ok(t: &str, v: &Value) -> bool {
    match (t, v) {
        ("object", Value::Obj(_)) | ("array", Value::Arr(_)) | ("string", Value::Str(_)) => true,
        ("boolean", Value::Bool(_)) | ("null", Value::Null) | ("number", Value::Num(_)) => true,
        ("integer", Value::Num(n)) => n.fract() == 0.0,
        _ => false,
    }
}

fn bounds(s: &Value, v: &Value) -> Result<(), String> {
    let Some(n) = v.f64() else { return Ok(()) };
    let lim = |k: &str| s.get(k).and_then(Value::f64);
    let bad = |k: &str, l: f64| Err(format!("{k} {l}"));
    match (lim("minimum"), lim("maximum"), lim("exclusiveMinimum")) {
        (Some(l), _, _) if n < l => bad("minimum", l),
        (_, Some(l), _) if n > l => bad("maximum", l),
        (_, _, Some(l)) if n <= l => bad("exclusiveMinimum", l),
        _ => Ok(()),
    }
}

fn members(s: &Value, v: &Value) -> Result<(), String> {
    let Value::Obj(fields) = v else { return Ok(()) };
    for r in s.get("required").map_or(&[][..], Value::arr) {
        let name = r.str().unwrap_or_default();
        if v.get(name).is_none() {
            return Err(format!("{name}: required"));
        }
    }
    let props = s.get("properties");
    for (k, x) in fields {
        match props.and_then(|p| p.get(k)) {
            Some(sub) => validate(sub, x).map_err(|e| format!("{k}.{e}"))?,
            None if s.get("additionalProperties").and_then(Value::bool) == Some(false) => {
                return Err(format!("{k}: unknown"));
            }
            None => {}
        }
    }
    Ok(())
}

fn items(s: &Value, v: &Value) -> Result<(), String> {
    let Value::Arr(a) = v else { return Ok(()) };
    let n = |k: &str| s.get(k).and_then(Value::f64);
    if n("minItems").is_some_and(|m| (a.len() as f64) < m) {
        return Err("minItems".into());
    }
    if n("maxItems").is_some_and(|m| (a.len() as f64) > m) {
        return Err("maxItems".into());
    }
    match s.get("items") {
        Some(sub) => a
            .iter()
            .enumerate()
            .try_for_each(|(i, x)| validate(sub, x).map_err(|e| format!("[{i}].{e}"))),
        None => Ok(()),
    }
}

fn branches(s: &Value, v: &Value) -> Result<(), String> {
    let count = |k: &str| {
        s.get(k)
            .map(|b| b.arr().iter().filter(|x| validate(x, v).is_ok()).count())
    };
    if count("oneOf") == Some(0) || count("oneOf").is_some_and(|c| c > 1) {
        return Err("oneOf".into());
    }
    if count("anyOf") == Some(0) {
        return Err("anyOf".into());
    }
    Ok(())
}

/// `Ok` when `v` satisfies `s`; the `Err` names the first failing path and keyword.
pub fn validate(s: &Value, v: &Value) -> Result<(), String> {
    if let Some(t) = s.get("type") {
        let names: Vec<&str> = match t {
            Value::Arr(a) => a.iter().filter_map(Value::str).collect(),
            other => other.str().into_iter().collect(),
        };
        if !names.iter().any(|n| type_ok(n, v)) {
            return Err("type".into());
        }
    }
    if s.get("const").is_some_and(|c| c != v) {
        return Err("const".into());
    }
    if s.get("enum").is_some_and(|e| !e.arr().contains(v)) {
        return Err("enum".into());
    }
    bounds(s, v)?;
    members(s, v)?;
    items(s, v)?;
    branches(s, v)
}
