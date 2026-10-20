//! Reader of `mzflow/2` (and of the keys of `mzflow/1`): JSON text to a `Plan`. It only checks the shape of the file; what the steps mean is `check`.

use super::{FileInput, Instance, Observed, Plan, Program, Ref};
use crate::error::{Error, Result};
use crate::json::{self, Value};

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::str).map(str::to_string)
}

/// `file:NAME` or `STEP.OUTPUT`.
fn reference(s: &str, instance: &str) -> Result<Ref> {
    let bad = || {
        Error::new("err.flow.ref")
            .with("instance", instance)
            .with("ref", s)
    };
    if let Some(name) = s.strip_prefix("file:") {
        return if name.is_empty() {
            Err(bad())
        } else {
            Ok(Ref::File(name.into()))
        };
    }
    match s.split_once('.') {
        Some((i, p)) if !i.is_empty() && !p.is_empty() && !p.contains('.') => Ok(Ref::Out {
            instance: i.into(),
            port: p.into(),
        }),
        _ => Err(bad()),
    }
}

/// `id@version` to (id, version).
fn step_ref(s: &str, instance: &str) -> Result<(String, u32)> {
    let bad = || {
        Error::new("err.flow.step")
            .with("instance", instance)
            .with("step", s)
    };
    let (id, v) = s.rsplit_once('@').ok_or_else(bad)?;
    let v: u32 = v.parse().map_err(|_| bad())?;
    if id.is_empty() {
        return Err(bad());
    }
    Ok((id.into(), v))
}

fn instance(v: &Value) -> Result<Instance> {
    let id = text(v, "id").ok_or_else(|| Error::new("err.flow.instance").with("field", "id"))?;
    let step = text(v, "step").ok_or_else(|| {
        Error::new("err.flow.instance")
            .with("instance", &id)
            .with("field", "step")
    })?;
    let (step, version) = step_ref(&step, &id)?;
    let mut ins = Vec::new();
    if let Some(Value::Obj(kv)) = v.get("in") {
        for (port, val) in kv {
            let refs: Result<Vec<Ref>> = match val {
                Value::Str(s) => Ok(vec![reference(s, &id)?]),
                Value::Arr(a) => a
                    .iter()
                    .map(|x| reference(x.str().unwrap_or_default(), &id))
                    .collect(),
                _ => Err(Error::new("err.flow.ref")
                    .with("instance", &id)
                    .with("port", port)),
            };
            ins.push((port.clone(), refs?));
        }
    }
    let params = match v.get("params") {
        Some(Value::Obj(kv)) => kv.clone(),
        _ => Vec::new(),
    };
    Ok(Instance {
        id,
        step,
        version,
        algorithm: text(v, "algorithm"),
        preset: text(v, "preset"),
        label: text(v, "label"),
        ins,
        params,
    })
}

fn observed(v: &Value) -> Observed {
    let strs = |k: &str| {
        v.get(k)
            .map(Value::arr)
            .unwrap_or_default()
            .iter()
            .filter_map(|x| x.str().map(str::to_string))
            .collect()
    };
    let nums = |k: &str| -> Vec<f64> {
        v.get(k)
            .map(Value::arr)
            .unwrap_or_default()
            .iter()
            .filter_map(Value::f64)
            .collect()
    };
    Observed {
        resolution: strs("resolution"),
        polarity: nums("polarity").iter().map(|x| *x as i8).collect(),
        ms_levels: nums("ms_levels").iter().map(|x| *x as u32).collect(),
    }
}

fn file_input(v: &Value) -> Result<FileInput> {
    let name = text(v, "name").ok_or_else(|| Error::new("err.flow.input").with("field", "name"))?;
    Ok(FileInput {
        sha256: text(v, "sha256").unwrap_or_default(),
        bytes: v.get("bytes").and_then(Value::f64).map(|b| b as u64),
        ty: text(v, "type"),
        observed: v.get("facets").map(observed).unwrap_or_default(),
        name,
    })
}

fn program(v: Option<&Value>) -> Program {
    let Some(p) = v else {
        return Program::default();
    };
    // the keys of mzflow/1 (`nome`, `versione`) are still read
    let pick = |a: &str, b: &str| text(p, a).or_else(|| text(p, b));
    Program {
        name: pick("name", "nome").unwrap_or_default(),
        version: pick("version", "versione").unwrap_or_default(),
        commit: text(p, "commit"),
    }
}

/// Reads a workflow file. A file without `format` is taken as `mzflow/2` (the fragments of the study have only `steps`); any other format is refused.
pub fn parse(bytes: &[u8]) -> Result<Plan> {
    let v = json::parse(bytes).map_err(|_| Error::new("err.flow.json"))?;
    if !matches!(v, Value::Obj(_)) {
        return Err(Error::new("err.flow.json"));
    }
    // mzflow/1 keys: formato, programma, operazioni
    let format = text(&v, "format").or_else(|| text(&v, "formato"));
    let legacy = format.as_deref() == Some("mzflow/1");
    if let Some(f) = &format {
        if f != "mzflow/2" && !legacy {
            return Err(Error::new("err.flow.format").with("format", f));
        }
    }
    let list = |k: &str| v.get(k).map(Value::arr).unwrap_or_default();
    let mut plan = Plan {
        format: format.unwrap_or_else(|| "mzflow/2".into()),
        program: program(v.get("program").or_else(|| v.get("programma"))),
        operations: v.get("operations").or_else(|| v.get("operazioni")).cloned(),
        ..Plan::default()
    };
    for f in list("inputs") {
        plan.inputs.push(file_input(f)?);
    }
    for s in list("steps") {
        plan.steps.push(instance(s)?);
    }
    Ok(plan)
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
