//! Checks of what a subagent of the assistant wrote: `lettura/1` (findings, questions, actions) and `param-scelte/1` (the choices on the parameters
//! proposed by the rules). Every `evidence` key must exist in the input the subagent was given, every number of a sentence must be in the input, actions
//! must name real parameters inside the `step/1` limits. The problems go back to the subagent once; then the reading is dropped. Used by the agent.

use std::collections::HashSet;

use mzlab_core::json::{Out, Value};
use mzlab_core::step::{find, Step};

use crate::params_rules::{fit, target_param};
use crate::text::numbers;

pub const MAX_FINDINGS: usize = 8;
pub const MAX_QUESTIONS: usize = 5;
const MAX_OPTIONS: usize = 8;
const KINDS: [&str; 3] = ["acquisition", "chromatography", "table"];
const CONFIDENCE: [&str; 3] = ["high", "medium", "low"];
const ACTIONS: [&str; 6] = [
    "none",
    "ask",
    "set_param",
    "add_step",
    "split_series",
    "chart",
];

/// One thing wrong in a reading: `err.llm.reading.<reason>` and where.
pub struct Problem {
    pub code: String,
    pub at: String,
}

type Found = Vec<Problem>;

fn flag(found: &mut Found, reason: &str, at: &str) {
    found.push(Problem {
        code: format!("err.llm.reading.{reason}"),
        at: at.to_string(),
    });
}

/// The problems as JSON text for the subagent: `[{"code","at"}]`.
pub fn problems_json(problems: &[Problem]) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, p) in problems.iter().enumerate() {
        o.raw(if i == 0 { "{\"code\":" } else { ",{\"code\":" });
        o.str(&p.code).raw(",\"at\":").str(&p.at).raw("}");
    }
    o.raw("]");
    o.0
}

/// Checks `reading` (a `lettura/1` or a `param-scelte/1`) against `input`, the data blocks the subagent saw as one object keyed by source
/// (`{"ispezione/1": ..., "answers": ...}`); `registry` gives the limits of the parameters.
pub fn validate(reading: &Value, input: &Value, registry: &[Step]) -> Result<(), Found> {
    let mut found = Found::new();
    let known = known_numbers(input);
    let cx = Cx {
        input,
        registry,
        known: &known,
    };
    match reading.get("schema").and_then(Value::str) {
        Some("lettura/1") => cx.reading(reading, &mut found),
        Some("param-scelte/1") => cx.choices(reading, &mut found),
        _ => flag(&mut found, "schema", "schema"),
    }
    if found.is_empty() {
        Ok(())
    } else {
        Err(found)
    }
}

struct Cx<'a> {
    input: &'a Value,
    registry: &'a [Step],
    known: &'a HashSet<String>,
}

/// Every number of the input as a canonical string, from numeric values and from the text of the strings.
fn known_numbers(v: &Value) -> HashSet<String> {
    fn walk(v: &Value, out: &mut HashSet<String>) {
        match v {
            Value::Num(x) => {
                out.insert(x.to_string());
            }
            Value::Str(s) => out.extend(numbers(s)),
            Value::Arr(a) => a.iter().for_each(|e| walk(e, out)),
            Value::Obj(o) => o.iter().for_each(|(_, e)| walk(e, out)),
            _ => {}
        }
    }
    let mut out = HashSet::new();
    walk(v, &mut out);
    out
}

/// One step of an evidence path: a key of an object, or the element of an array whose `id`, `alias`, `name` or `step` is `seg` (else its position).
/// An object without the key lends its arrays (`tabcheck.blank` is the check `blank` of `checks`, `ispezione.f1` the file `f1` of `files`).
fn step_in<'a>(v: &'a Value, seg: &str) -> Option<&'a Value> {
    match v {
        Value::Arr(a) => a
            .iter()
            .find(|e| {
                ["id", "alias", "name", "step"]
                    .iter()
                    .any(|k| e.get(k).and_then(Value::str) == Some(seg))
            })
            .or_else(|| seg.parse::<usize>().ok().and_then(|i| a.get(i))),
        Value::Obj(o) => v.get(seg).or_else(|| {
            o.iter()
                .filter(|(_, c)| matches!(c, Value::Arr(_)))
                .find_map(|(_, c)| {
                    step_in(c, seg).filter(|_| {
                        c.arr()
                            .iter()
                            .all(|e| e.get("id").is_some() || e.get("alias").is_some())
                    })
                })
        }),
        _ => None,
    }
}

/// An evidence key is `source.path...`: the source is a key of the input with or without its `/1`.
fn resolves(input: &Value, key: &str) -> bool {
    let mut segs = key.split('.');
    let (Value::Obj(top), Some(root)) = (input, segs.next()) else {
        return false;
    };
    let Some(mut cur) = top
        .iter()
        .find(|(k, _)| k == root || k.strip_suffix("/1") == Some(root))
        .map(|(_, v)| v)
    else {
        return false;
    };
    for seg in segs {
        match step_in(cur, seg) {
            Some(next) => cur = next,
            None => return false,
        }
    }
    true
}

fn text_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)
        .and_then(Value::str)
        .filter(|s| !s.trim().is_empty())
}

impl Cx<'_> {
    /// Flags every number of `sentence` that the input does not have.
    fn sentence(&self, sentence: &str, at: &str, found: &mut Found) {
        if numbers(sentence).iter().any(|n| !self.known.contains(n)) {
            flag(found, "number", at);
        }
    }

    /// `numbers`: strings (or numbers) that all appear in the input.
    fn numbers_list(&self, r: &Value, found: &mut Found) {
        for (i, n) in r
            .get("numbers")
            .map_or(&[][..], Value::arr)
            .iter()
            .enumerate()
        {
            let canon = match n {
                Value::Num(x) => x.to_string(),
                Value::Str(s) => numbers(s).pop().unwrap_or_default(),
                _ => String::new(),
            };
            if !self.known.contains(&canon) {
                flag(found, "number", &format!("numbers[{i}]"));
            }
        }
    }

    /// Up to `MAX_QUESTIONS` closed questions `{id, text, options}`; returns their ids.
    fn questions(&self, r: &Value, found: &mut Found) -> Vec<String> {
        let list = r.get("questions").map_or(&[][..], Value::arr);
        if list.len() > MAX_QUESTIONS {
            flag(found, "too_many", "questions");
        }
        let mut ids = Vec::new();
        for (i, q) in list.iter().enumerate() {
            let at = format!("questions[{i}]");
            let options = q.get("options").map_or(&[][..], Value::arr);
            let closed = (2..=MAX_OPTIONS).contains(&options.len())
                && options.iter().all(|o| o.str().is_some());
            // the id counts even when the question is refused, so one defect gives one problem
            ids.extend(text_of(q, "id").map(String::from));
            match text_of(q, "text") {
                Some(t) if closed && text_of(q, "id").is_some() => self.sentence(t, &at, found),
                _ => flag(found, "question", &at),
            }
        }
        ids
    }

    fn reading(&self, r: &Value, found: &mut Found) {
        if !KINDS.contains(&r.get("kind").and_then(Value::str).unwrap_or("")) {
            flag(found, "kind", "kind");
        }
        let ids = self.questions(r, found);
        let list = r.get("findings").map_or(&[][..], Value::arr);
        if list.len() > MAX_FINDINGS {
            flag(found, "too_many", "findings");
        }
        for (i, f) in list.iter().enumerate() {
            self.finding(f, &ids, &format!("findings[{i}]"), found);
        }
        self.numbers_list(r, found);
    }

    fn finding(&self, f: &Value, questions: &[String], at: &str, found: &mut Found) {
        let Some(statement) = text_of(f, "statement") else {
            return flag(found, "statement", at);
        };
        self.sentence(statement, at, found);
        let evidence = f.get("evidence").map_or(&[][..], Value::arr);
        if evidence.is_empty() {
            flag(found, "no_evidence", at);
        }
        for (k, e) in evidence.iter().enumerate() {
            if !e.str().is_some_and(|key| resolves(self.input, key)) {
                flag(found, "evidence", &format!("{at}.evidence[{k}]"));
            }
        }
        if !CONFIDENCE.contains(&f.get("confidence").and_then(Value::str).unwrap_or("")) {
            flag(found, "confidence", at);
        }
        self.action(f.get("action"), questions, &format!("{at}.action"), found);
    }

    fn action(&self, a: Option<&Value>, questions: &[String], at: &str, found: &mut Found) {
        let Some(a) = a else {
            return flag(found, "action", at);
        };
        let kind = a.get("type").and_then(Value::str).unwrap_or("");
        let ok = match kind {
            "ask" => text_of(a, "ref").is_some_and(|r| questions.iter().any(|q| q == r)),
            "chart" => text_of(a, "ref").is_some(),
            "set_param" => return self.set_param(a, at, found),
            "add_step" => text_of(a, "step").is_some_and(|s| self.known_step(s)),
            _ => ACTIONS.contains(&kind),
        };
        if !ok {
            flag(found, "action", at);
        }
    }

    fn known_step(&self, step: &str) -> bool {
        find(self.registry, step.split('@').next().unwrap_or("")).is_some()
    }

    /// A parameter that exists, with a value inside the limits of its descriptor.
    fn set_param(&self, a: &Value, at: &str, found: &mut Found) {
        let step = text_of(a, "step").unwrap_or("");
        let name = text_of(a, "name").unwrap_or("");
        let alg = text_of(a, "algorithm").unwrap_or("");
        let Some(p) = target_param(
            self.registry,
            step.split('@').next().unwrap_or(""),
            alg,
            name,
        ) else {
            return flag(found, "unknown_param", at);
        };
        match a.get("value") {
            Some(v) if fit(p, v.clone()).1 => {}
            _ => flag(found, "out_of_range", at),
        }
    }

    /// The proposals of the rules that the subagent was given.
    fn proposals(&self) -> &[Value] {
        self.input
            .get("proposta-param/1")
            .and_then(|p| p.get("proposals"))
            .map_or(&[][..], Value::arr)
    }

    fn choices(&self, r: &Value, found: &mut Found) {
        let proposals = self.proposals();
        let list = r.get("choices").map_or(&[][..], Value::arr);
        for (i, c) in list.iter().enumerate() {
            self.choice(c, &format!("choices[{i}]"), found);
        }
        for p in proposals {
            if !list.iter().any(|c| same_target(c, p)) {
                flag(
                    found,
                    "missing_choice",
                    text_of(p, "name").unwrap_or("choices"),
                );
            }
        }
        self.questions(r, found);
        for (i, s) in r
            .get("skipped")
            .map_or(&[][..], Value::arr)
            .iter()
            .enumerate()
        {
            let at = format!("skipped[{i}]");
            match text_of(s, "why") {
                Some(why) => self.sentence(why, &at, found),
                None => flag(found, "skipped", &at),
            }
        }
        self.numbers_list(r, found);
    }

    fn choice(&self, c: &Value, at: &str, found: &mut Found) {
        let Some(prop) = self.proposals().iter().find(|p| same_target(c, p)) else {
            return flag(found, "unknown_choice", at);
        };
        let (Some(value), Some(reason)) = (c.get("value"), text_of(c, "reason")) else {
            return flag(found, "choice", at);
        };
        self.sentence(reason, at, found);
        let kept = prop.get("value") == Some(value);
        let rule = text_of(prop, "rule").unwrap_or("");
        let source = text_of(c, "source").unwrap_or("");
        // a kept value names its rule; a changed one says "model"
        let want = if kept {
            format!("rule:{rule}")
        } else {
            "model".into()
        };
        if source != want {
            flag(found, "source", at);
        }
        let step = text_of(prop, "step").unwrap_or("");
        let alg = text_of(prop, "algorithm").unwrap_or("");
        let name = text_of(prop, "name").unwrap_or("");
        let id = step.split('@').next().unwrap_or("");
        if target_param(self.registry, id, alg, name).is_some_and(|p| !fit(p, value.clone()).1) {
            flag(found, "out_of_range", at);
        }
    }
}

/// Two entries name the same parameter: same step, algorithm (when any) and name.
fn same_target(a: &Value, b: &Value) -> bool {
    ["step", "algorithm", "name"]
        .iter()
        .all(|k| text_of(a, k) == text_of(b, k))
}
