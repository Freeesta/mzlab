//! The shape of an `mzflow/2` file, checked before it is read: every key that is not part of the format and every value of the wrong JSON type is
//! refused with its path (`steps[1].params`), so a misspelt key or a plan of the wrong kind is never taken for an empty one. What the steps mean is
//! `check`; the JavaScript reader of the Flow page (`flow/lazy/flow-check.js`) applies the same rules.

use crate::error::{Error, Result};
use crate::json::Value;

const ROOT: &[&str] = &["format", "program", "inputs", "steps", "operations"];
const INPUT: &[&str] = &["name", "sha256", "bytes", "type", "facets"];
const STEP: &[&str] = &["id", "step", "algorithm", "preset", "label", "in", "params"];

fn shape(path: &str, expected: &str) -> Error {
    Error::new("err.flow.shape")
        .with("path", path)
        .with("expected", expected)
}

/// The members of `v` when it is an object whose keys are all in `allowed`.
fn members<'a>(v: &'a Value, path: &str, allowed: &[&str]) -> Result<&'a [(String, Value)]> {
    let Value::Obj(kv) = v else {
        return Err(shape(path, "object"));
    };
    for (k, _) in kv {
        if !allowed.contains(&k.as_str()) {
            let at = if path.is_empty() {
                k.clone()
            } else {
                format!("{path}.{k}")
            };
            return Err(Error::new("err.flow.key.unknown").with("path", at));
        }
    }
    Ok(kv)
}

fn text(v: &Value, path: &str) -> Result<()> {
    v.str().map(|_| ()).ok_or_else(|| shape(path, "string"))
}

fn input(v: &Value, path: &str) -> Result<()> {
    for (k, x) in members(v, path, INPUT)? {
        let at = format!("{path}.{k}");
        match k.as_str() {
            "name" | "sha256" | "type" => text(x, &at)?,
            "bytes" if x.f64().is_none() => return Err(shape(&at, "number")),
            "facets" if !matches!(x, Value::Obj(_)) => return Err(shape(&at, "object")),
            _ => {}
        }
    }
    Ok(())
}

fn step(v: &Value, path: &str) -> Result<()> {
    for (k, x) in members(v, path, STEP)? {
        let at = format!("{path}.{k}");
        match k.as_str() {
            "id" | "step" | "algorithm" | "preset" | "label" => text(x, &at)?,
            "params" if !matches!(x, Value::Obj(_)) => return Err(shape(&at, "object")),
            "in" => {
                let Value::Obj(ports) = x else {
                    return Err(shape(&at, "object"));
                };
                for (port, r) in ports {
                    let p = format!("{at}.{port}");
                    match r {
                        Value::Str(_) => {}
                        Value::Arr(a) => {
                            for (i, s) in a.iter().enumerate() {
                                text(s, &format!("{p}[{i}]"))?;
                            }
                        }
                        _ => return Err(shape(&p, "string or array of strings")),
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Refuses a plan whose keys or value types are not those of `mzflow/2`; `legacy` (`mzflow/1`) has no `steps`.
pub(super) fn check_shape(v: &Value, legacy: bool) -> Result<()> {
    members(v, "", ROOT)?;
    match v.get("steps") {
        Some(Value::Arr(a)) => {
            for (i, s) in a.iter().enumerate() {
                step(s, &format!("steps[{i}]"))?;
            }
        }
        Some(_) => return Err(shape("steps", "array")),
        None if legacy => {}
        None => return Err(shape("steps", "array")),
    }
    match v.get("inputs") {
        Some(Value::Arr(a)) => {
            for (i, f) in a.iter().enumerate() {
                input(f, &format!("inputs[{i}]"))?;
            }
        }
        Some(_) => return Err(shape("inputs", "array")),
        None => {}
    }
    if v.get("format").is_some_and(|f| f.str().is_none()) {
        return Err(shape("format", "string"));
    }
    Ok(())
}
