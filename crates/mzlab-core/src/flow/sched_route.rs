//! The live scheduler as one JSON route (`route(op, body)`), for the pages that cannot hold a Rust object: the engine `.wasm` of the site holds one scheduler
//! per session in this module and the Flow page talks to it with `api/flow/sched/<op>`. Texts in and out are JSON; a job is `{"instance","item"}`.
//! The registry is the `step/1` array the page already holds (`step::registry_from_value`). Design: `docs/agenti/flow-live.md`.

use super::sched::{Command, Done, JobId, JobSpec, Landed, Limits, NodeView, Sched};
use super::{parse, value_text, Plan};
use crate::error::{Error, Result};
use crate::json::{self, Out, Value};
use crate::step::registry_from_value;
use std::cell::RefCell;

thread_local! {
    static SCHED: RefCell<Option<Sched>> = const { RefCell::new(None) };
}

fn no_sched() -> Error {
    Error::new("err.flow.sched")
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

/// `plan` is an `mzflow/2` object or its text.
fn read_plan(body: &Value) -> Result<Plan> {
    match body.get("plan") {
        Some(Value::Str(t)) => parse(t.as_bytes()),
        Some(v) => parse(value_text(v).as_bytes()),
        None => Err(Error::new("err.flow.plan")),
    }
}

fn job_of(body: &Value) -> JobId {
    let j = body.get("job");
    let text = |k: &str| j.and_then(|j| j.get(k)).and_then(Value::str);
    JobId::new(text("instance").unwrap_or(""), text("item"))
}

fn id_json(o: &mut Out, id: &JobId) {
    o.raw("\"instance\":").str(&id.instance).raw(",\"item\":");
    match &id.item {
        Some(n) => o.str(n),
        None => o.raw("null"),
    };
}

fn list<T>(items: &[T], each: impl Fn(&mut Out, &T)) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, x) in items.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        each(&mut o, x);
    }
    o.raw("]").0.clone()
}

fn spec_json(o: &mut Out, j: &JobSpec) {
    o.raw("{");
    id_json(o, &j.id);
    o.raw(",\"key\":").str(&j.key).raw(",\"mb\":").num(j.mb);
    o.raw(",\"resources\":[");
    for (k, r) in j.resources.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(r);
    }
    o.raw("]}");
}

fn node_json(o: &mut Out, n: &NodeView) {
    o.raw("{\"instance\":").str(&n.instance);
    o.raw(",\"state\":").str(n.state.name());
    o.raw(",\"frac\":").opt(n.frac);
    o.raw(",\"parte\":").int(n.parte as i64);
    o.raw(",\"di\":").int(n.di as i64);
    o.raw(",\"ms\":").num(n.ms).raw("}");
}

/// `{"ms"}` is a job that is done, `{"error_key","params":{}}` one that failed.
fn result_of(r: &Value) -> std::result::Result<Done, Error> {
    let Some(k) = r.get("error_key").and_then(Value::str) else {
        return Ok(Done {
            ms: r.get("ms").and_then(Value::f64).unwrap_or(0.0),
        });
    };
    let mut e = Error::new(leak(k));
    if let Some(Value::Obj(kv)) = r.get("params") {
        for (name, v) in kv {
            e = e.with(leak(name), v.str().unwrap_or(""));
        }
    }
    Err(e)
}

fn start(body: &Value) -> Result<String> {
    let ids = match body.get("identities") {
        Some(Value::Obj(kv)) => kv
            .iter()
            .filter_map(|(k, v)| v.str().map(|t| (k.clone(), t.to_string())))
            .collect(),
        _ => Vec::new(),
    };
    let registry = registry_from_value(body.get("registry").ok_or_else(no_sched)?)?;
    let version = body.get("version").and_then(Value::str).unwrap_or("");
    let sched = Sched::new(&read_plan(body)?, &registry, version, &ids);
    SCHED.with(|s| *s.borrow_mut() = Some(sched));
    Ok("{}".to_string())
}

/// `{"slots","mem_mb","mb":{"<instance>|<item>": estimate},"mb_default"}`.
fn next(s: &Sched, body: &Value) -> String {
    let num = |k: &str, d: f64| body.get(k).and_then(Value::f64).unwrap_or(d);
    let limits = Limits {
        slots: num("slots", 99.0) as usize,
        mem_mb: num("mem_mb", 1e9),
    };
    let fallback = num("mb_default", 1.0);
    let estimate = |id: &JobId| {
        let name = id
            .item
            .as_ref()
            .map_or(id.instance.clone(), |n| format!("{}|{n}", id.instance));
        body.get("mb")
            .and_then(|t| t.get(&name))
            .and_then(Value::f64)
            .unwrap_or(fallback)
    };
    list(&s.next(&limits, estimate), spec_json)
}

fn finished(s: &mut Sched, body: &Value) -> String {
    let result = result_of(body.get("result").unwrap_or(&Value::Null));
    match s.finished(&job_of(body), result) {
        Landed::Accepted => "\"accepted\"".to_string(),
        Landed::Discarded { key } => format!("\"discarded:{key}\""),
        Landed::Unknown => "\"unknown\"".to_string(),
    }
}

/// Acts on the scheduler; None when `op` is not one of the calls that only change it.
fn act(s: &mut Sched, op: &str, body: &Value) -> Option<String> {
    let flag = |b: bool| if b { "true" } else { "false" }.to_string();
    let num = |k: &str| body.get(k).and_then(Value::f64).unwrap_or(0.0);
    Some(match op {
        "update" => list(
            &s.update(&read_plan(body).ok()?),
            |o, Command::Cancel(id)| {
                o.raw("{");
                id_json(o, id);
                o.raw("}");
            },
        ),
        "started" => flag(s.started(&job_of(body))),
        "cached" => flag(s.cached(&job_of(body))),
        "block" => flag(s.block(
            &job_of(body),
            body.get("by").and_then(Value::str).unwrap_or(""),
        )),
        "progress" => {
            s.progress(&job_of(body), num("frac"));
            "{}".to_string()
        }
        "unblock" => {
            s.unblock(&job_of(body));
            "{}".to_string()
        }
        "retry" => {
            s.retry(&job_of(body));
            "{}".to_string()
        }
        "pause" => {
            s.pause();
            "{}".to_string()
        }
        "resume" => {
            s.resume();
            "{}".to_string()
        }
        _ => return None,
    })
}

/// Reads the scheduler; None when `op` is not one of the questions.
fn ask(s: &Sched, op: &str, body: &Value) -> Option<String> {
    Some(match op {
        "next" => next(s, body),
        "jobs" => list(&s.jobs(), |o, (id, st)| {
            o.raw("{");
            id_json(o, id);
            o.raw(",\"state\":").str(st.name()).raw("}");
        }),
        "nodes" => list(&s.nodes(), node_json),
        "events" => s.events_since(body.get("since").and_then(Value::f64).unwrap_or(0.0) as u64),
        "rev" => s.rev().to_string(),
        _ => return None,
    })
}

/// One call. `new` builds the scheduler of the session (`plan`, `registry`, `version`, `identities`); `update` takes `plan`; the other ops are the methods of
/// `Sched` (`next`, `started`, `progress`, `finished`, `cached`, `block`, `unblock`, `retry`, `pause`, `resume`, `jobs`, `nodes`, `events`, `rev`,
/// `set_identity`, `set_resources`); `drop` forgets it.
pub fn route(op: &str, body: &str) -> Result<String> {
    let body = json::parse(body.as_bytes())?;
    match op {
        "new" => return start(&body),
        "drop" => {
            SCHED.with(|s| *s.borrow_mut() = None);
            return Ok("{}".to_string());
        }
        _ => {}
    }
    SCHED.with(|cell| {
        let mut guard = cell.borrow_mut();
        let s = guard.as_mut().ok_or_else(no_sched)?;
        match op {
            "finished" => Ok(finished(s, &body)),
            "set_identity" => {
                let text = |k: &str| body.get(k).and_then(Value::str).unwrap_or("");
                s.set_identity(text("name"), text("text"));
                Ok("{}".to_string())
            }
            "set_resources" => {
                let names = body.get("res").map(Value::arr).unwrap_or_default();
                s.set_resources(
                    &job_of(&body),
                    names
                        .iter()
                        .filter_map(Value::str)
                        .map(str::to_string)
                        .collect(),
                );
                Ok("{}".to_string())
            }
            _ => act(s, op, &body)
                .or_else(|| ask(s, op, &body))
                .ok_or_else(|| Error::new("err.route.unavailable")),
        }
    })
}

#[cfg(test)]
#[path = "sched_route_tests.rs"]
mod tests;
