//! Check of a whole plan against the step descriptors, before anything starts: unknown steps and versions, steps or algorithms that cannot run here,
//! parameters out of range, ports (types, conversions, cardinality, facets) and cycles. It returns every problem, not the first.

use super::canon::{declared, effective};
use super::order::{cyclic, plan_order};
use super::{Instance, Observed, Plan, Ref};
use crate::json::Value;
use crate::step::{find, DataType, Kind, Param, Port, Status, Step};

/// Where the plan is going to run: a step with no module for it is refused (`err.flow.unavailable`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Native,
    Wasm,
}

impl Platform {
    pub fn name(self) -> &'static str {
        match self {
            Platform::Native => "native",
            Platform::Wasm => "wasm",
        }
    }
    /// The platform this build runs on.
    pub fn current() -> Platform {
        if cfg!(target_family = "wasm") {
            Platform::Wasm
        } else {
            Platform::Native
        }
    }
    fn runs(self, step: &Step) -> bool {
        match self {
            Platform::Native => step.runs_in.native,
            Platform::Wasm => step.runs_in.wasm.is_some(),
        }
    }
}

/// One thing wrong in a plan. `error_key` is an i18n key (`err.flow.*`) and `params` its values, as in `Error`; `port` is empty when the problem is not
/// about a port.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub instance: String,
    pub port: String,
    pub error_key: &'static str,
    pub params: Vec<(&'static str, String)>,
}

/// The shortest chain of registered conversion steps that turns `from` into `to` (empty when they are the same type); None when there is none.
pub fn conversion_path(registry: &[Step], from: DataType, to: DataType) -> Option<Vec<&Step>> {
    let mut seen = vec![(from, Vec::<&Step>::new())];
    let mut at = 0;
    while at < seen.len() {
        let (ty, path) = seen[at].clone();
        if ty == to {
            return Some(path);
        }
        for s in registry {
            if let Some((a, b)) = s.converts {
                if a == ty && !seen.iter().any(|(t, _)| *t == b) {
                    let mut p = path.clone();
                    p.push(s);
                    seen.push((b, p));
                }
            }
        }
        at += 1;
    }
    None
}

struct Ctx<'a> {
    plan: &'a Plan,
    registry: &'a [Step],
    platform: Platform,
    problems: Vec<Problem>,
}

/// A requirement is not met when something is known about the data and not all of what is required is in it (unknown = checked when the step starts).
fn misses<T, H>(want: &[T], have: &[H], has: impl Fn(&T, &[H]) -> bool) -> bool {
    !have.is_empty() && !want.iter().all(|w| has(w, have))
}

fn param_error(p: &Param, v: &Value) -> Option<&'static str> {
    let num = |x: &Value| x.f64().filter(|n| n.is_finite());
    let out_of = |n: f64| p.min.is_some_and(|m| n < m) || p.max.is_some_and(|m| n > m);
    match &p.kind {
        Kind::Number => match num(v) {
            None => Some("err.flow.param.type"),
            Some(n) if out_of(n) => Some("err.flow.param.range"),
            Some(_) => None,
        },
        Kind::Integer => match num(v) {
            Some(n) if n.fract() != 0.0 => Some("err.flow.param.type"),
            None => Some("err.flow.param.type"),
            Some(n) if out_of(n) => Some("err.flow.param.range"),
            Some(_) => None,
        },
        Kind::Bool => v.bool().is_none().then_some("err.flow.param.type"),
        Kind::Pair(min) => match (v.at(0).and_then(num), v.at(1).and_then(num), v.arr().len()) {
            (Some(a), Some(b), 2) if a >= *min && b >= a => None,
            (Some(_), Some(_), 2) => Some("err.flow.param.range"),
            _ => Some("err.flow.param.type"),
        },
        Kind::Choice(all) => (!all.contains(v)).then_some("err.flow.param.range"),
    }
}

fn add(c: &mut Ctx, i: &Instance, port: &str, key: &'static str, params: &[(&'static str, &str)]) {
    let p = Problem {
        instance: i.id.clone(),
        port: port.to_string(),
        error_key: key,
        params: params.iter().map(|(k, v)| (*k, v.to_string())).collect(),
    };
    if !c.problems.contains(&p) {
        c.problems.push(p);
    }
}

fn step_of<'a>(c: &'a Ctx, i: &Instance) -> Option<&'a Step> {
    find(c.registry, &i.step).filter(|s| s.version == i.version)
}

/// The step, algorithm and preset an instance names; a problem for anything unknown or unavailable.
fn step_level(c: &mut Ctx, i: &Instance) -> Option<Step> {
    let Some(known) = find(c.registry, &i.step) else {
        add(c, i, "", "err.flow.step.unknown", &[("step", &i.step)]);
        return None;
    };
    if known.version != i.version {
        let (have, want) = (known.version.to_string(), i.version.to_string());
        add(
            c,
            i,
            "",
            "err.flow.step.version",
            &[("step", &i.step), ("version", &want), ("available", &have)],
        );
        return None;
    }
    let step = known.clone();
    if !c.platform.runs(&step) {
        add(
            c,
            i,
            "",
            "err.flow.unavailable",
            &[("step", &i.step), ("platform", c.platform.name())],
        );
    }
    match effective(i, &step) {
        Ok((Some(a), _)) if a.status == Status::Planned => {
            add(
                c,
                i,
                "",
                "err.flow.algorithm.unavailable",
                &[("step", &i.step), ("algorithm", a.id)],
            );
        }
        Ok(_) => {}
        Err(e) => {
            let value = e.params.first().map(|p| p.1.clone()).unwrap_or_default();
            let name = if e.key.ends_with("preset.unknown") {
                "preset"
            } else {
                "algorithm"
            };
            add(c, i, "", e.key, &[(name, &value)]);
        }
    }
    Some(step)
}

fn params(c: &mut Ctx, i: &Instance, step: &Step) {
    let algorithm = effective(i, step).ok().and_then(|(a, _)| a);
    let known: Vec<&Param> = declared(step, algorithm).collect();
    for (name, v) in &i.params {
        let Some(p) = known.iter().find(|p| p.name == name) else {
            add(c, i, "", "err.flow.param.unknown", &[("param", name)]);
            continue;
        };
        if let Some(key) = param_error(p, v) {
            add(c, i, "", key, &[("param", name)]);
        }
    }
}

/// The type a reference gives: of an input file (when declared) or of an output of another instance.
fn source_type(c: &mut Ctx, i: &Instance, port: &Port, r: &Ref) -> Option<(DataType, bool)> {
    match r {
        Ref::File(name) => {
            let Some(f) = c.plan.input(name) else {
                add(c, i, port.name, "err.flow.input.unknown", &[("ref", name)]);
                return None;
            };
            Some((f.ty.as_deref().and_then(DataType::from_name)?, false))
        }
        Ref::Out {
            instance,
            port: out,
        } => {
            let text = format!("{instance}.{out}");
            let Some(src) = c.plan.instance(instance) else {
                add(c, i, port.name, "err.flow.ref.unknown", &[("ref", &text)]);
                return None;
            };
            // an unknown step of the source is already a problem of the source
            let step = step_of(c, src)?;
            let Some(o) = step.outs.iter().find(|o| o.name == out) else {
                add(c, i, port.name, "err.flow.port.unknown", &[("ref", &text)]);
                return None;
            };
            Some((o.ty, o.many))
        }
    }
}

fn ports(c: &mut Ctx, i: &Instance, step: &Step, seen: &[Observed]) {
    for (name, refs) in &i.ins {
        let Some(port) = step.ins.iter().find(|p| p.name == name) else {
            add(c, i, name, "err.flow.port.unknown", &[("port", name)]);
            continue;
        };
        if !port.many && refs.len() > 1 {
            add(
                c,
                i,
                name,
                "err.flow.cardinality",
                &[("port", name), ("count", &refs.len().to_string())],
            );
        }
        for r in refs {
            let Some((ty, list)) = source_type(c, i, port, r) else {
                continue;
            };
            if list && !port.many {
                add(
                    c,
                    i,
                    name,
                    "err.flow.cardinality",
                    &[("port", name), ("count", "many")],
                );
            }
            let fits = ty == port.ty || port.accepts.contains(&ty);
            if !fits && conversion_path(c.registry, ty, port.ty).is_none() {
                add(
                    c,
                    i,
                    name,
                    "err.flow.type",
                    &[("from", ty.name()), ("to", port.ty.name())],
                );
            }
        }
    }
    for port in step.ins.iter().filter(|p| !p.optional) {
        if !i.ins.iter().any(|(n, r)| n == port.name && !r.is_empty()) {
            add(
                c,
                i,
                port.name,
                "err.flow.port.missing",
                &[("port", port.name)],
            );
        }
    }
    facets(c, i, step, seen);
}

fn facets(c: &mut Ctx, i: &Instance, step: &Step, seen: &[Observed]) {
    let have = observed(c, i, seen);
    for port in &step.ins {
        let want = &port.requires;
        if misses(&want.ms_levels, &have.ms_levels, |w, h| h.contains(w)) {
            add(c, i, port.name, "err.flow.facet", &[("facet", "ms_levels")]);
        }
        if misses(&want.polarity, &have.polarity, |w, h| {
            *w == 0 || h.contains(w)
        }) {
            add(c, i, port.name, "err.flow.facet", &[("facet", "polarity")]);
        }
        if misses(&want.resolution, &have.resolution, |w, h| {
            h.iter().any(|x| x == w)
        }) {
            add(
                c,
                i,
                port.name,
                "err.flow.facet",
                &[("facet", "resolution")],
            );
        }
    }
}

/// What is known of the data an instance reads: the union of its sources.
fn observed(c: &Ctx, i: &Instance, seen: &[Observed]) -> Observed {
    let mut o = Observed::default();
    for r in i.ins.iter().flat_map(|(_, r)| r) {
        let s = match r {
            Ref::File(n) => c.plan.input(n).map(|f| f.observed.clone()),
            Ref::Out { instance, .. } => c
                .plan
                .steps
                .iter()
                .position(|x| x.id == *instance)
                .map(|k| seen[k].clone()),
        };
        let s = s.unwrap_or_default();
        o.ms_levels.extend(s.ms_levels);
        o.polarity.extend(s.polarity);
        o.resolution.extend(s.resolution);
    }
    o
}

/// Every problem of `plan` for `platform`, in the order of the instances of the file (cycles last).
pub fn check(plan: &Plan, registry: &[Step], platform: Platform) -> Vec<Problem> {
    let mut c = Ctx {
        plan,
        registry,
        platform,
        problems: Vec::new(),
    };
    for (k, i) in plan.steps.iter().enumerate() {
        if plan.steps[..k].iter().any(|x| x.id == i.id) {
            add(&mut c, i, "", "err.flow.instance.duplicate", &[]);
        }
    }
    let order = plan_order(plan).unwrap_or_else(|_| (0..plan.steps.len()).collect());
    let mut seen = vec![Observed::default(); plan.steps.len()];
    let duplicates = std::mem::take(&mut c.problems);
    let mut by_instance: Vec<Vec<Problem>> = vec![Vec::new(); plan.steps.len()];
    for &k in &order {
        let i = &plan.steps[k];
        let before = c.problems.len();
        if let Some(step) = step_level(&mut c, i) {
            params(&mut c, i, &step);
            ports(&mut c, i, &step, &seen);
            // the facets of the data pass through an instance unchanged, to the outputs that still are the scans
            if step
                .outs
                .iter()
                .any(|o| matches!(o.ty, DataType::Raw | DataType::Mzml | DataType::Mzidx))
            {
                seen[k] = observed(&c, i, &seen);
            }
        }
        by_instance[k] = c.problems.split_off(before);
    }
    // report in the order of the file, not the order of the run
    let mut all: Vec<Problem> = duplicates
        .into_iter()
        .chain(by_instance.into_iter().flatten())
        .collect();
    crate::sortc::slice_by_key(&mut all, |p| {
        plan.steps.iter().position(|x| x.id == p.instance)
    });
    c.problems = all;
    for k in cyclic(plan) {
        let i = &plan.steps[k];
        add(&mut c, i, "", "err.flow.cycle", &[]);
    }
    c.problems
}
