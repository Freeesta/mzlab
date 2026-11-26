//! `step-params/1`: the parameters of one Flow step with their defaults, read from the `step/1` descriptor (`json::step_json`), the only place they are
//! written. The CLI, the app route and the `.wasm` of the site all call `route`, so the three print the same bytes and no second table can drift.

use crate::error::{Error, Result};
use crate::json::{self, Value};
use crate::jsonb::{text, write, Obj};

/// One parameter of the descriptor as an entry of `step-params/1`; `algorithm` is the id of the algorithm it belongs to (`None`: the step's own).
fn entry(instance: &str, algorithm: Option<&str>, name: &str, p: &Value) -> Value {
    let flag = |k: &str| p.get(k).and_then(Value::bool) == Some(true);
    let get = |k: &str| p.get(k).cloned().unwrap_or(Value::Null);
    let ty = match p.get("type").and_then(Value::str) {
        Some("array") => "pair",
        Some(t) => t,
        None => "choice",
    };
    let default = get("default");
    // no default: the step measures it (`estimated`) or the rule is not applied (`off`)
    let when_empty = match (default.is_null(), flag("x-estimate")) {
        (false, _) => Value::Null,
        (true, true) => text("estimated"),
        (true, false) => text("off"),
    };
    let description = p
        .get("description")
        .or_else(|| p.get("title"))
        .cloned()
        .unwrap_or(Value::Null);
    Obj::new()
        .put("name", text(name))
        .put("set_key", text(&format!("{instance}.{name}")))
        .put("type", text(ty))
        .put("default", default)
        .put("when_empty", when_empty)
        .put("minimum", get("minimum"))
        .put("maximum", get("maximum"))
        .put("enum", get("enum"))
        .put("unit", get("x-unit"))
        .put("algorithm", algorithm.map_or(Value::Null, text))
        .b("advanced", flag("x-advanced"))
        .put("description", description)
        .done()
}

/// The entries of a `{"properties": {...}}` schema, in the order the descriptor lists them.
fn entries(out: &mut Vec<Value>, instance: &str, algorithm: Option<&str>, schema: Option<&Value>) {
    let props = schema.and_then(|s| s.get("properties"));
    for (name, p) in props.map_or(&[][..], Value::entries) {
        out.push(entry(instance, algorithm, name, p));
    }
}

/// The named presets of the step: `set_key` selects one, `params` are the values it writes (the `--set` key and the value), `evidence` says why those values.
fn presets(instance: &str, list: Option<&Value>) -> Vec<Value> {
    let get = |p: &Value, k: &str| p.get(k).cloned().unwrap_or(Value::Null);
    let mut out = Vec::new();
    for (id, p) in list.map_or(&[][..], Value::entries) {
        let values = p.get("params").map_or(&[][..], Value::entries);
        let values = values.iter().map(|(k, v)| {
            Obj::new()
                .put("set_key", text(&format!("{instance}.{k}")))
                .put("value", v.clone())
                .done()
        });
        out.push(
            Obj::new()
                .put("id", text(id))
                .put("set_key", text(&format!("{instance}.preset")))
                .put("title", get(p, "title"))
                .put("params", Value::Arr(values.collect()))
                .put("evidence", get(p, "evidence"))
                .done(),
        );
    }
    out
}

/// `step-params/1` of a `step/1` object: every parameter of the step, then those of each algorithm, with the key `--set` takes. The instance name is
/// the last part of the step id, as in `flow steps --template`.
pub fn route(input: &str) -> Result<String> {
    let step = json::parse(input.as_bytes())?;
    let (Some(id), Some(version)) = (
        step.get("id").and_then(Value::str),
        step.get("version").and_then(Value::f64),
    ) else {
        return Err(Error::new("err.flow.step.unknown").with("step", "(not a step/1 object)"));
    };
    let instance = id.rsplit('.').next().unwrap_or(id);
    let mut list = Vec::new();
    entries(&mut list, instance, None, step.get("params"));
    for a in step.get("algorithms").map_or(&[][..], Value::arr) {
        let algo = a.get("id").and_then(Value::str);
        entries(&mut list, instance, algo, a.get("params"));
    }
    let mut doc = Obj::new()
        .put("schema", text("step-params/1"))
        .put("step", text(id))
        .n("version", version)
        .put("title", step.get("title").cloned().unwrap_or(Value::Null))
        .put("instance", text(instance))
        .put("params", Value::Arr(list));
    let presets = presets(instance, step.get("presets"));
    if !presets.is_empty() {
        doc = doc.put("presets", Value::Arr(presets));
    }
    let doc = doc.done();
    Ok(write(&doc) + "\n")
}
