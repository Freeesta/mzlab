//! Evaluation of the assistant on small cases: a request, the context the user has, and the properties the answer must have (steps present or
//! absent, a plan that passes the check, at most N tool calls, words in the reply). `run` drives the agent against a recording; `score` counts the
//! properties. In CI the recordings prove the scoring; real models are tried by hand, outside CI (`mzlab-cli llm eval --provider`).

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};
use mzlab_core::step::Step;

use crate::agent::{step, Input, Output, State};
use crate::context::build;
use crate::fake::Replay;
use crate::policy::{Level, Profile};
use crate::provider::builtin;
use crate::tools::Tool;

/// The properties a case asks for; an absent one is not checked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Expect {
    pub has_steps: Vec<String>,
    pub no_steps: Vec<String>,
    pub max_calls: Option<u32>,
    pub accepted: Option<bool>,
    pub tries: Option<u32>,
    pub text_contains: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    pub id: String,
    pub request: String,
    pub level: Level,
    /// Sources for `contesto/1` (the profile `ricerca` is implied).
    pub context: Option<Value>,
    /// What each read tool answers, by tool name; any other tool answers `{"ok":true}`.
    pub tool_results: Vec<(String, Value)>,
    pub expect: Expect,
}

/// What a run produced.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    /// The (instance, step id) pairs of the plan the assistant built.
    pub steps: Vec<(String, String)>,
    pub calls: u32,
    pub tries: u32,
    pub accepted: bool,
    pub text: String,
    pub log: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    pub checks: Vec<(String, bool)>,
}

impl Score {
    pub fn passed(&self) -> usize {
        self.checks.iter().filter(|(_, ok)| *ok).count()
    }
}

fn bad(field: &str) -> Error {
    Error::new("err.llm.eval.case").with("field", field)
}

fn names(v: Option<&Value>) -> Vec<String> {
    v.map_or(&[][..], Value::arr)
        .iter()
        .filter_map(Value::str)
        .map(String::from)
        .collect()
}

impl Case {
    pub fn parse(text: &str) -> Result<Case> {
        let v = parse(text.as_bytes()).map_err(|_| bad("json"))?;
        if v.get("schema").and_then(Value::str) != Some("llm-eval/1") {
            return Err(bad("schema"));
        }
        let text = |k: &str| {
            v.get(k)
                .and_then(Value::str)
                .map(String::from)
                .ok_or_else(|| bad(k))
        };
        let e = v.get("expect");
        let num = |k: &str| {
            e.and_then(|e| e.get(k))
                .and_then(Value::f64)
                .map(|n| n as u32)
        };
        let level = v
            .get("level")
            .and_then(Value::str)
            .map_or(Some(Level::L2), Level::parse);
        let results = match v.get("tool_results") {
            Some(Value::Obj(o)) => o.clone(),
            _ => Vec::new(),
        };
        Ok(Case {
            id: text("id")?,
            request: text("request")?,
            level: level.ok_or_else(|| bad("level"))?,
            context: v.get("context").cloned(),
            tool_results: results,
            expect: Expect {
                has_steps: names(e.and_then(|e| e.get("has_steps"))),
                no_steps: names(e.and_then(|e| e.get("no_steps"))),
                max_calls: num("max_calls"),
                accepted: e.and_then(|e| e.get("accepted")).and_then(Value::bool),
                tries: num("tries"),
                text_contains: names(e.and_then(|e| e.get("text_contains"))),
            },
        })
    }
}

fn bare(step_ref: &str) -> &str {
    step_ref.split('@').next().unwrap_or(step_ref)
}

/// What the page would do with an allowed call, enough to know the plan the assistant ends with.
fn apply(plan: &mut Plan, name: &str, args: &Value) -> Value {
    match name {
        "flow_add_step" => {
            let id = format!("s{}", plan.steps.len() + 1);
            let step = args.get("step").and_then(Value::str).unwrap_or_default();
            plan.steps.push((id.clone(), step.to_string()));
            Value::Obj(vec![("instance".into(), Value::Str(id))])
        }
        "flow_remove_step" => {
            let id = args
                .get("instance")
                .and_then(Value::str)
                .unwrap_or_default();
            plan.steps.retain(|(i, _)| i != id);
            Value::Obj(vec![("ok".into(), Value::Bool(true))])
        }
        "flow_propose_plan" => {
            let list = args
                .get("plan")
                .and_then(|p| p.get("steps"))
                .map_or(&[][..], Value::arr);
            plan.steps = list
                .iter()
                .map(|s| {
                    let f = |k: &str| {
                        s.get(k)
                            .and_then(Value::str)
                            .unwrap_or_default()
                            .to_string()
                    };
                    (f("id"), f("step"))
                })
                .collect();
            plan.accepted = true;
            Value::Obj(vec![("shown".into(), Value::Bool(true))])
        }
        _ => Value::Obj(vec![("ok".into(), Value::Bool(true))]),
    }
}

fn feed(
    s: &mut State,
    case: &Case,
    plan: &mut Plan,
    replay: &mut Replay,
    first: Input,
) -> Result<()> {
    let mut queue = vec![first];
    for _ in 0..200 {
        let Some(input) = queue.pop() else {
            return Ok(());
        };
        for o in step(s, input) {
            match o {
                Output::Send(r) => {
                    queue.extend(replay.respond(&r)?.into_iter().rev().map(Input::Event))
                }
                Output::CallTool { id, name, args } => {
                    let content = case
                        .tool_results
                        .iter()
                        .find(|(n, _)| *n == name)
                        .map(|(_, v)| v.clone());
                    let content = content.unwrap_or_else(|| apply(plan, &name, &args));
                    queue.push(Input::ToolResult {
                        id,
                        ok: true,
                        content,
                    });
                }
                Output::AskConfirm { id, .. } => queue.push(Input::Confirm { id, yes: true }),
                Output::Show(t) => plan.text.push_str(&t),
                Output::Log(_) | Output::Sub { .. } | Output::Done => {}
            }
        }
    }
    Err(Error::new("err.llm.eval.loop"))
}

/// Runs `case` against `replay` with the real catalog and registry; the tools run as `apply` says.
pub fn run(
    case: &Case,
    replay: &mut Replay,
    tools: Vec<Tool>,
    registry: Vec<Step>,
) -> Result<Plan> {
    let provider = builtin()
        .into_iter()
        .next()
        .ok_or_else(|| bad("provider"))?;
    let model = provider
        .models
        .first()
        .map(|m| m.id.clone())
        .ok_or_else(|| bad("model"))?;
    let mut s = State::new(
        provider,
        &model,
        tools,
        registry,
        case.level,
        Profile::Research,
    );
    if let Some(src) = &case.context {
        let mut sources = src.clone();
        if let Value::Obj(o) = &mut sources {
            o.retain(|(k, _)| k != "profile");
            o.push(("profile".into(), Value::Str("ricerca".into())));
        }
        s.add_context(parse(build(&sources, 8000)?.as_bytes()).map_err(|_| bad("context"))?);
    }
    let mut plan = Plan::default();
    feed(
        &mut s,
        case,
        &mut plan,
        replay,
        Input::User(case.request.clone()),
    )?;
    plan.calls = s
        .log()
        .lines()
        .filter(|l| l.contains("\"ev\":\"call\""))
        .count() as u32;
    plan.tries = s
        .log()
        .lines()
        .filter(|l| {
            l.contains("\"ev\":\"decision\"") && l.contains("\"tool\":\"flow_propose_plan\"")
        })
        .count() as u32;
    plan.log = s.log().to_string();
    Ok(plan)
}

/// Counts the properties of `case.expect` that `plan` has.
pub fn score(plan: &Plan, case: &Case) -> Score {
    let e = &case.expect;
    let mut checks: Vec<(String, bool)> = Vec::new();
    let has = |id: &str| plan.steps.iter().any(|(_, s)| bare(s) == bare(id));
    checks.extend(e.has_steps.iter().map(|s| (format!("has {s}"), has(s))));
    checks.extend(e.no_steps.iter().map(|s| (format!("no {s}"), !has(s))));
    if let Some(n) = e.max_calls {
        checks.push((format!("calls <= {n}"), plan.calls <= n));
    }
    if let Some(a) = e.accepted {
        checks.push(("accepted".into(), plan.accepted == a));
    }
    if let Some(n) = e.tries {
        checks.push((format!("tries = {n}"), plan.tries == n));
    }
    checks.extend(
        e.text_contains
            .iter()
            .map(|t| (format!("says {t}"), plan.text.contains(t.as_str()))),
    );
    Score { checks }
}
