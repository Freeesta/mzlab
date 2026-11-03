//! `migrate`: an `mzrun/1` file written by an engine whose steps had older versions (`spectra.msn_graph@1`) brought to the current ones with the table of
//! `flow::migrate`: the plan, `resolved` and `seeds` take the new versions and names, every renamed parameter is a `compat.changes` entry of kind
//! `migrated` and each migrated instance a limit `preset.param.migrated`; `plan_sha256` and `self_sha256` are recomputed. Used when a preset is applied.

use super::self_sha256;
use crate::budget::sha256_hex;
use crate::error::{Error, Result};
use crate::flow::migrate::{migrate_name, migrate_params, Migrated, Params};
use crate::flow::value_text;
use crate::json::{parse, Value};

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn obj(kv: Vec<(&str, Value)>) -> Value {
    Value::Obj(kv.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// `id@version` to (id, version).
fn split(r: &str) -> Option<(&str, u32)> {
    let (id, v) = r.rsplit_once('@')?;
    Some((id, v.parse().ok()?))
}

/// Replaces (or adds) `key` of an object.
fn put(v: &mut Value, key: &str, x: Value) {
    if let Value::Obj(kv) = v {
        match kv.iter_mut().find(|(k, _)| k == key) {
            Some(e) => e.1 = x,
            None => kv.push((key.to_string(), x)),
        }
    }
}

/// The keys of an object renamed with `migrate_name` of `step@version`.
fn rename_keys(v: &Value, step: &str, version: u32) -> Value {
    let kv = v.entries().iter().map(|(k, x)| (migrate_name(step, version, k).unwrap_or_else(|| k.clone()), x.clone()));
    Value::Obj(kv.collect())
}

/// One entry of `plan.steps` or of `resolved` migrated; `None` when nothing changes.
fn entry(v: &Value) -> Option<(Value, String, Migrated)> {
    let (id, version) = split(v.get("step")?.str()?)?;
    let params: Params = v.get("params").map_or(vec![], |p| p.entries().to_vec());
    let (to, p, m) = migrate_params(id, version, &params)?;
    let mut out = v.clone();
    put(&mut out, "step", s(&format!("{id}@{to}")));
    if v.get("params").is_some() {
        put(&mut out, "params", Value::Obj(p));
    }
    for key in ["pin", "estimates"] {
        if let Some(x) = v.get(key) {
            put(&mut out, key, rename_keys(x, id, version));
        }
    }
    Some((out, id.to_string(), m))
}

/// The file `text` with its steps at the current versions (canonical text, one final LF); a file with nothing to migrate comes back unchanged but
/// canonical. Not an `mzrun/1` object: `err.mzrun.read`; a newer schema: `error.newerFormat`.
pub fn migrate(text: &[u8]) -> Result<String> {
    let mut v = parse(text).map_err(|_| Error::new("err.mzrun.read"))?;
    if v.get("schema").and_then(Value::str) != Some("mzrun/1") {
        return Err(Error::new("err.mzrun.read"));
    }
    if v.get("min_reader").and_then(Value::f64).is_none_or(|n| n > 1.0) {
        return Err(Error::new("error.newerFormat"));
    }
    // (instance, step, what changed, the params after) of every migrated instance, `resolved` first: it holds every parameter
    let mut done: Vec<(String, String, Migrated, Value)> = Vec::new();
    if let Some(Value::Obj(kv)) = v.get("resolved").cloned() {
        let mut out = kv.clone();
        for (name, r) in out.iter_mut() {
            if let Some((new, step, m)) = entry(r) {
                done.push((name.clone(), step, m, new.get("params").cloned().unwrap_or(Value::Null)));
                *r = new;
            }
        }
        put(&mut v, "resolved", Value::Obj(out));
    }
    let mut plan = v.get("plan").cloned().ok_or_else(|| Error::new("err.mzrun.read"))?;
    if let Some(Value::Arr(steps)) = plan.get("steps").cloned() {
        let mut out = steps.clone();
        for st in out.iter_mut() {
            if let Some((new, step, m)) = entry(st) {
                let name = st.get("id").and_then(Value::str).unwrap_or_default().to_string();
                if !done.iter().any(|d| d.0 == name) {
                    done.push((name, step, m, new.get("params").cloned().unwrap_or(Value::Null)));
                }
                *st = new;
            }
        }
        put(&mut plan, "steps", Value::Arr(out));
    }
    if done.is_empty() {
        return Ok(value_text(&v) + "\n");
    }
    if let Some(Value::Obj(kv)) = v.get("seeds").cloned() {
        let renamed = kv.into_iter().map(|(k, x)| {
            let new = k.split_once('.').and_then(|(i, p)| {
                let d = done.iter().find(|d| d.0 == i)?;
                Some(format!("{i}.{}", migrate_name(&d.1, d.2.from, p)?))
            });
            (new.unwrap_or(k), x)
        });
        put(&mut v, "seeds", Value::Obj(renamed.collect()));
    }
    let mut changes: Vec<Value> = v.get("compat").and_then(|c| c.get("changes")).map_or(vec![], |c| c.arr().to_vec());
    let mut limits: Vec<Value> = v.get("limits").map_or(vec![], |l| l.arr().to_vec());
    for (name, step, m, params) in &done {
        for (_, new) in &m.renamed {
            let value = params.get(new).cloned().unwrap_or(Value::Null);
            changes.push(obj(vec![("instance", s(name)), ("param", s(new)), ("kind", s("migrated")), ("value", value), ("confirmed", Value::Bool(true))]));
        }
        let names: Vec<String> = m.renamed.iter().map(|(a, b)| format!("{a} → {b}")).collect();
        let detail = format!("{name}: {step}@{} → @{}{}", m.from, m.to, if names.is_empty() { String::new() } else { format!(" ({})", names.join(", ")) });
        limits.push(obj(vec![("code", s("preset.param.migrated")), ("detail", s(&detail))]));
    }
    let from = v.get("compat").and_then(|c| c.get("from_engine")).or_else(|| v.get("engine")).cloned().unwrap_or(Value::Null);
    put(&mut v, "compat", obj(vec![("from_engine", from), ("changes", Value::Arr(changes))]));
    put(&mut v, "limits", Value::Arr(limits));
    put(&mut v, "plan_sha256", s(&sha256_hex(value_text(&plan).as_bytes())));
    put(&mut v, "plan", plan);
    let fp = self_sha256(v.entries());
    put(&mut v, "self_sha256", s(&fp));
    Ok(value_text(&v) + "\n")
}
