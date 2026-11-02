//! `build`: the `mzrun/1` record of a finished run, from the `mzflow/2` plan, the `prov/1` the run left and the roles of the files (`campioni/2`).

use super::{results_digest, self_sha256};
use crate::budget::sha256_hex;
use crate::campioni::{digest, Campione};
use crate::error::{Error, Result};
use crate::flow::value_text;
use crate::json::Value;

/// What a record is made of beyond the plan and the `prov/1`; resources, decisions and limits are written as the schema has them.
pub struct Parts<'a> {
    pub plan: &'a Value,
    pub prov: &'a Value,
    pub roles: &'a [Campione],
    pub resources: Vec<Value>,
    pub decisions: Vec<Value>,
    pub limits: Vec<Value>,
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn obj(kv: Vec<(&str, Value)>) -> Value {
    Value::Obj(kv.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn int(x: u64) -> Value {
    Value::Num(x as f64)
}

fn bad(what: &'static str, detail: &str) -> Error {
    Error::new("err.mzrun.build")
        .with("what", what)
        .with("detail", detail)
}

fn text<'a>(v: &'a Value, k: &str, what: &'static str) -> Result<&'a str> {
    v.get(k)
        .and_then(Value::str)
        .ok_or_else(|| bad(what, k))
}

/// The engine that ran: the `agent` of the `prov/1`, with the keys it has. The commit is required (a record names the code that made it).
fn engine(prov: &Value) -> Result<Value> {
    let agent = prov.get("agent").ok_or_else(|| bad("prov", "agent"))?;
    if text(agent, "commit", "engine").map_or(true, |c| c.len() != 40) {
        return Err(bad("engine", "commit"));
    }
    Ok(Value::Obj(agent.entries().to_vec()))
}

/// instance -> step, algorithm, preset, params, pin (`estimate` for a measured parameter, else `fixed`) and the proofs.
fn resolved(prov: &Value) -> Result<(Value, Value)> {
    let (mut out, mut seeds) = (Vec::new(), Vec::new());
    for (name, a) in prov.get("activity").map_or(&[][..], Value::entries) {
        let params = a.get("params").ok_or_else(|| bad("activity", name))?;
        let est = a.get("estimates");
        let pin = params
            .entries()
            .iter()
            .map(|(k, _)| {
                let measured = est.is_some_and(|e| e.get(k).is_some());
                (k.clone(), s(if measured { "estimate" } else { "fixed" }))
            })
            .collect();
        let mut kv = vec![
            ("step", s(text(a, "step", "activity")?)),
            ("params", params.clone()),
            ("pin", Value::Obj(pin)),
        ];
        for key in ["algorithm", "preset"] {
            if let Some(x) = a.get(key) {
                kv.push((key, x.clone()));
            }
        }
        if let Some(e) = est {
            kv.push(("estimates", e.clone()));
        }
        out.push((name.clone(), obj(kv)));
        for (k, v) in params.entries() {
            if k == "seed" || k.ends_with("_seed") {
                if let Some(n) = v.f64().filter(|n| n.fract() == 0.0) {
                    seeds.push((format!("{name}.{k}"), Value::Num(n)));
                }
            }
        }
    }
    Ok((Value::Obj(out), Value::Obj(seeds)))
}

fn input(f: &Value, roles: &[Campione]) -> Result<Value> {
    let alias = text(f, "name", "input")?;
    let sha = text(f, "sha256", "input")?;
    let c = roles
        .iter()
        .find(|c| c.sha256 == sha)
        .ok_or_else(|| bad("role", alias))?;
    let role = if c.kind == "pool" { "qc" } else { &c.kind };
    let mut kv = vec![
        ("alias", s(alias)),
        ("sha256", s(sha)),
        ("bytes", f.get("bytes").cloned().unwrap_or(int(0))),
        ("type", s(text(f, "type", "input")?)),
        ("role", s(role)),
    ];
    if let Some(b) = &c.blank_kind {
        kv.push(("tipo_bianco", s(b)));
    }
    if let Some(p) = c.polarity {
        kv.push(("polarity", s(if p > 0 { "+" } else { "-" })));
    }
    if c.order > 0 {
        kv.push(("injection_order", int(u64::from(c.order))));
    }
    if c.replicate > 0 {
        kv.push(("replicate", int(u64::from(c.replicate))));
    }
    if !c.series.is_empty() {
        kv.push(("series", s(&c.series)));
    }
    Ok(obj(kv))
}

/// The canonical text of the record (one final LF). The inputs are the files of the plan, named by their `name` as alias; each needs its role by
/// SHA-256 in `roles`. Entities of the results are what the run made, the files of the plan being the inputs.
pub fn build(p: &Parts) -> Result<String> {
    let plan_text = value_text(p.plan);
    let (resolved, seeds) = resolved(p.prov)?;
    let inputs = p
        .plan
        .get("inputs")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|f| input(f, p.roles))
        .collect::<Result<Vec<_>>>()?;
    let made: Vec<(String, String)> = p
        .prov
        .get("entity")
        .map_or(&[][..], Value::entries)
        .iter()
        .filter(|(k, _)| !k.starts_with("file:"))
        .filter_map(|(k, e)| Some((k.clone(), e.get("sha256")?.str()?.to_string())))
        .collect();
    let entities = Value::Obj(made.iter().map(|(k, h)| (k.clone(), s(h))).collect());
    let mut kv = vec![
        ("schema", s("mzrun/1")),
        ("kind", s("record")),
        ("min_reader", int(1)),
        ("engine", engine(p.prov)?),
        ("plan", p.plan.clone()),
        ("plan_sha256", s(&sha256_hex(plan_text.as_bytes()))),
        ("resolved", resolved),
        ("inputs", Value::Arr(inputs)),
        ("roles_sha256", s(&digest(p.roles))),
        ("resources", Value::Arr(p.resources.clone())),
        ("seeds", seeds),
        ("decisions", Value::Arr(p.decisions.clone())),
        ("revisions", int(0)),
        ("limits", Value::Arr(p.limits.clone())),
        ("prov", p.prov.clone()),
        (
            "results",
            obj(vec![
                ("entities", entities),
                ("results_digest", s(&results_digest(&made))),
            ]),
        ),
    ];
    let mut entries: Vec<(String, Value)> =
        kv.drain(..).map(|(k, v)| (k.to_string(), v)).collect();
    let fp = self_sha256(&entries);
    entries.push(("self_sha256".into(), s(&fp)));
    Ok(value_text(&Value::Obj(entries)) + "\n")
}
