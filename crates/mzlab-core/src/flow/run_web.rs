//! The record of a run made by an executor that has no `Runner` (the page of the site runs the jobs in workers and reports what each made): `record_jobs`
//! reads those reports and writes the record with the same `record` as `flow run`, so `prov/1` and `corsa/1` have the bytes of the native run apart from
//! the declared fields (time, memory peak, agent). Used by the engine `.wasm` (`flow_record`).

use super::{record, Artifact, Ctx, Outcome, Results, Run};
use crate::budget::sha256_hex;
use crate::error::{Error, Result};
use crate::flow::sched::{JobId, Sched};
use crate::flow::{check, corsa_build, parse, plan_json, prov_json, Agent, Mem, Platform, Uscita};
use crate::json::{parse as parse_json, Value};
use crate::step::{registry_from_value, DataType, Step};

/// A `&'static str` for an error key or a value that comes from the report (a handful per run).
fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::str)
}

fn bad() -> Error {
    Error::new("err.flow.report")
}

/// The artifact a job reports for one output: `{"sha256","bytes","type"}` and the file name `label`, else `<instance>[-<root>].<ext>` as `flow run` names it.
fn artifact(ctx: &Ctx, id: &JobId, o: &Value) -> Result<Artifact> {
    let ty = text(o, "type").and_then(DataType::from_name).ok_or_else(bad)?;
    let label = match (text(o, "label"), text(o, "ext")) {
        (Some(l), _) => l.to_string(),
        (None, Some(ext)) => match ctx.item_root(id) {
            Some(root) => format!("{}-{root}.{ext}", id.instance),
            None => format!("{}.{ext}", id.instance),
        },
        _ => return Err(bad()),
    };
    Ok(Artifact {
        sha256: text(o, "sha256").ok_or_else(bad)?.to_string(),
        bytes: o.get("bytes").and_then(Value::f64).ok_or_else(bad)? as u64,
        ty,
        label: Some(label),
    })
}

fn mem(v: &Value) -> Option<Mem> {
    let via = match text(v, "via")? {
        "memory" => "memory",
        "slabs" => "slabs",
        _ => "refused",
    };
    Some(Mem {
        mb: v.get("mb")?.f64()?,
        via,
    })
}

/// The outcome of one job from its report; a report with `error_key` is a failure with its values in `params`.
fn outcome(ctx: &Ctx, id: &JobId, r: &Value) -> Result<Outcome> {
    if let Some(key) = text(r, "error_key") {
        let mut e = Error::new(leak(key));
        for (k, v) in r.get("params").map_or(&[][..], Value::entries) {
            e = e.with(leak(k), v.str().unwrap_or(""));
        }
        return Err(e);
    }
    let mut o = Outcome::default();
    for x in r.get("outputs").map_or(&[][..], Value::arr) {
        let port = text(x, "port").ok_or_else(bad)?.to_string();
        o.outputs.push((port, artifact(ctx, id, x)?));
    }
    for x in r.get("estimates").map_or(&[][..], Value::arr) {
        let name = text(x, "name").ok_or_else(bad)?.to_string();
        o.estimates.push((
            name,
            x.get("value").cloned().unwrap_or(Value::Null),
            x.get("proof").cloned().unwrap_or(Value::Null),
        ));
    }
    o.ms = r.get("ms").and_then(Value::f64).unwrap_or(0.0);
    o.heap_peak_bytes = r.get("heap_peak_bytes").and_then(Value::f64).map(|b| b as u64);
    o.mem = r.get("mem").and_then(mem);
    o.warnings = r
        .get("warnings")
        .map_or(&[][..], Value::arr)
        .iter()
        .filter_map(|w| w.str().map(str::to_string))
        .collect();
    o.cached = r.get("cached").and_then(Value::bool).unwrap_or(false);
    o.file = r.get("file").filter(|f| !f.is_null()).cloned();
    Ok(o)
}

/// The record of `plan_text` run elsewhere, and the plan as `flow run` writes it (`plan.json`, whose SHA-256 the record names). `jobs` is the array of reports
/// `{"instance","item"?, ...}` (see `outcome`); a job with no report counts as failed (`err.flow.worker`), as in `record`. The plan is checked as `execute` checks it.
pub fn record_jobs(plan_text: &[u8], registry: &[Step], jobs: &str) -> Result<(Run, String)> {
    let plan = parse(plan_text)?;
    if let Some(p) = check(&plan, registry, Platform::current()).first() {
        return Err(Error::new("err.flow.check").with("instance", &p.instance).with("error_key", p.error_key));
    }
    let sched = Sched::new(&plan, registry, env!("CARGO_PKG_VERSION"), &[]);
    let ctx = Ctx::new(&plan, registry, &sched)?;
    let reports = parse_json(jobs.as_bytes())?;
    let mut results = Results::new();
    for r in reports.arr() {
        let id = JobId::new(text(r, "instance").unwrap_or(""), text(r, "item"));
        let o = outcome(&ctx, &id, r);
        results.insert(id, o);
    }
    let ids = ctx.ids().to_vec();
    Ok((record(&plan, registry, &ids, results)?, plan_json(&plan)))
}

/// `op` `prov`: `{"plan": text, "registry": [step/1], "jobs": [...]}` gives the `prov/1` text; `corsa`: `{"prov": text, "uscite": [{"name","text"}]}` gives the
/// `corsa/1` text of that record with the `.json` files the steps made. `agent` is the program that ran (the caller knows its version).
pub fn route(op: &str, body: &str, agent: &Agent) -> Result<String> {
    let b = parse_json(body.as_bytes())?;
    let field = |k: &str| text(&b, k).ok_or_else(bad);
    match op {
        "prov" => {
            let plan = field("plan")?;
            let registry = registry_from_value(b.get("registry").ok_or_else(bad)?)?;
            let jobs = crate::flow::value_text(b.get("jobs").ok_or_else(bad)?);
            let (run, text) = record_jobs(plan.as_bytes(), &registry, &jobs)?;
            Ok(prov_json(agent, Some(&sha256_hex(text.as_bytes())), &run))
        }
        "corsa" => {
            let prov = parse_json(field("prov")?.as_bytes())?;
            let kept: Vec<(&str, &str)> = b
                .get("uscite")
                .map_or(&[][..], Value::arr)
                .iter()
                .filter_map(|u| Some((text(u, "name")?, text(u, "text")?)))
                .collect();
            let uscite: Vec<Uscita> = kept
                .iter()
                .map(|(n, t)| Uscita { name: n, bytes: t.as_bytes() })
                .collect();
            Ok(corsa_build(&prov, &uscite)?.json())
        }
        _ => Err(Error::new("err.route.unavailable")),
    }
}

#[cfg(test)]
#[path = "run_web_tests.rs"]
mod tests;
