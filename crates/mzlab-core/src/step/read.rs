//! Reads `step/1` descriptors back from their JSON (the registry the Flow page already holds), keeping what the plan check, the canonical parameters and the
//! scheduler use: ports, parameters, algorithms, presets, where a step runs. Texts for people (titles, docs, references) are left empty. Used by `flow::sched_route`.

use super::{Algorithm, DataType, Facets, Kind, Param, Port, Preset, RunsIn, Status, Step};
use crate::error::{Error, Result};
use crate::json::Value;

fn bad() -> Error {
    Error::new("err.flow.registry")
}

fn text(v: &Value, k: &str) -> Option<&'static str> {
    v.get(k).and_then(Value::str).map(crate::error::intern)
}

fn entries(v: Option<&Value>) -> &[(String, Value)] {
    match v {
        Some(Value::Obj(kv)) => kv,
        _ => &[],
    }
}

fn nums<T>(v: Option<&Value>, f: impl Fn(f64) -> T) -> Vec<T> {
    v.map(Value::arr)
        .unwrap_or_default()
        .iter()
        .filter_map(Value::f64)
        .map(f)
        .collect()
}

fn strs(v: Option<&Value>) -> Vec<&'static str> {
    v.map(Value::arr)
        .unwrap_or_default()
        .iter()
        .filter_map(Value::str)
        .map(crate::error::intern)
        .collect()
}

fn facets(v: Option<&Value>) -> Facets {
    let Some(v) = v else { return Facets::default() };
    Facets {
        resolution: strs(v.get("resolution")),
        polarity: nums(v.get("polarity"), |x| x as i8),
        ms_levels: nums(v.get("ms_levels"), |x| x as u32),
    }
}

fn data_type(v: &Value) -> Result<DataType> {
    DataType::from_name(v.str().ok_or_else(bad)?).ok_or_else(bad)
}

fn ports(v: Option<&Value>) -> Result<Vec<Port>> {
    entries(v)
        .iter()
        .map(|(name, p)| {
            let mut port = Port::new(crate::error::intern(name), data_type(p.get("type").ok_or_else(bad)?)?, "");
            port.many = p.get("many").and_then(Value::bool) == Some(true);
            port.optional = p.get("optional").and_then(Value::bool) == Some(true);
            port.accepts = p
                .get("accepts")
                .map(Value::arr)
                .unwrap_or_default()
                .iter()
                .map(data_type)
                .collect::<Result<_>>()?;
            port.requires = facets(p.get("requires"));
            Ok(port)
        })
        .collect()
}

fn kind(p: &Value) -> Kind {
    if let Some(e) = p.get("enum") {
        return Kind::Choice(e.arr().to_vec());
    }
    match p.get("type").and_then(Value::str) {
        Some("integer") => Kind::Integer,
        Some("boolean") => Kind::Bool,
        Some("string") => Kind::Text,
        Some("array") => Kind::Pair(
            p.get("items")
                .and_then(|i| i.get("minimum"))
                .and_then(Value::f64)
                .unwrap_or(0.0),
        ),
        _ => Kind::Number,
    }
}

fn params(v: Option<&Value>) -> Vec<Param> {
    let flag = |p: &Value, k: &str| p.get(k).and_then(Value::bool) == Some(true);
    entries(v.and_then(|s| s.get("properties")))
        .iter()
        .map(|(name, p)| {
            let mut out = Param::number(crate::error::intern(name), "");
            out.kind = kind(p);
            out.default = p.get("default").cloned();
            out.min = p.get("minimum").and_then(Value::f64);
            out.max = p.get("maximum").and_then(Value::f64);
            out.estimate = flag(p, "x-estimate");
            out.noresult = flag(p, "x-noresult");
            out.list = ["transformations", "adducts"]
                .into_iter()
                .find(|k| p.get("x-liste").and_then(Value::str) == Some(*k));
            out
        })
        .collect()
}

fn algorithm(a: &Value) -> Result<Algorithm> {
    Ok(Algorithm {
        id: text(a, "id").ok_or_else(bad)?,
        version: a.get("version").and_then(Value::f64).unwrap_or(1.0) as u32,
        title: "",
        doc: "",
        references: Vec::new(),
        applicability: Default::default(),
        params: params(a.get("params")),
        status: if a.get("status").and_then(Value::str) == Some("planned") {
            Status::Planned
        } else {
            Status::Available
        },
    })
}

fn preset((id, p): &(String, Value)) -> Preset {
    Preset {
        id: crate::error::intern(id),
        title: "",
        when: facets(p.get("when")),
        params: entries(p.get("params"))
            .iter()
            .map(|(k, v)| (crate::error::intern(k), v.clone()))
            .collect(),
        evidence: "",
        algorithm: text(p, "algorithm"),
    }
}

fn step(s: &Value) -> Result<Step> {
    let mut out = Step::core(text(s, "id").ok_or_else(bad)?, "", "");
    out.version = s.get("version").and_then(Value::f64).unwrap_or(1.0) as u32;
    out.ins = ports(s.get("in"))?;
    out.outs = ports(s.get("out"))?;
    out.params = params(s.get("params"));
    if let Some(r) = s.get("runs_in") {
        out.runs_in = RunsIn {
            wasm: text(r, "wasm"),
            wasm_ceiling_bytes: r
                .get("wasm_ceiling_bytes")
                .and_then(Value::f64)
                .map(|x| x as u64),
            native: r.get("native").and_then(Value::bool) == Some(true),
        };
    }
    out.algorithms = s
        .get("algorithms")
        .map(Value::arr)
        .unwrap_or_default()
        .iter()
        .map(algorithm)
        .collect::<Result<_>>()?;
    out.default_algorithm = text(s, "default_algorithm");
    out.per_item = s.get("per_item").and_then(Value::bool) == Some(true);
    if let Some(c) = s.get("converts") {
        out.converts = Some((
            data_type(c.get("from").ok_or_else(bad)?)?,
            data_type(c.get("to").ok_or_else(bad)?)?,
        ));
    }
    out.presets = entries(s.get("presets")).iter().map(preset).collect();
    out.default_preset = text(s, "default_preset");
    Ok(out)
}

/// The registry of an array of `step/1` objects (or of a single object).
pub fn registry_from_value(v: &Value) -> Result<Vec<Step>> {
    match v {
        Value::Arr(list) => list.iter().map(step).collect(),
        Value::Obj(_) => Ok(vec![step(v)?]),
        _ => Err(bad()),
    }
}
