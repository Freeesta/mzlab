//! The job graph of a plan for the live scheduler: which jobs exist (one per instance, or one per item of a `per_item` step), what each depends on, and the
//! `plan_key` of each, a SHA-256 over what the job reads and how it is set, known before anything runs. Used by `sched`; pure, no files, no clock.

use crate::budget::sha256_hex;
use crate::error::Result;
use crate::flow::{canonical, plan_order, Instance, Plan, Ref};
use crate::step::Step;

impl Walk<'_> {
    fn instance(&mut self, k: usize) {
        let plan = self.plan;
        let inst = &plan.steps[k];
        let resolved = self.steps[k].map(|s| canonical(inst, s));
        let (Some(step), Some(Ok(resolved))) = (self.steps[k], resolved) else {
            let why = format!("invalid\n{}\n{}", inst.id, inst.step_ref());
            self.built[k].jobs.push(self.nodes.len());
            self.nodes.push(Node {
                id: JobId::new(&inst.id, None),
                inst: k,
                key: sha256_hex(why.as_bytes()),
                deps: Vec::new(),
                depth: 0,
                item_order: usize::MAX,
                invalid: true,
            });
            return;
        };
        // the driver is the first `many` port that reads something, as in `run::jobs`
        let mut driver: Option<(&str, Option<Vec<String>>)> = None;
        for p in step.ins.iter().filter(|p| p.many) {
            let refs = refs_of(inst, p.name);
            let names = item_names(plan, &self.steps, &self.built, refs);
            if names.as_ref().is_none_or(|n| !n.is_empty()) && !refs.is_empty() {
                driver = Some((p.name, names));
                break;
            }
        }
        let items = match (&driver, step.per_item) {
            (Some((_, Some(names))), true) => Some(names.clone()),
            _ => None,
        };
        let origin = self.origin(inst);
        let recipe = if RECIPE_STEPS.contains(&inst.step.as_str()) {
            recipe_job(plan, &self.built, &self.nodes, inst, origin.first())
        } else {
            None
        };
        let algorithm = resolved
            .algorithm
            .as_ref()
            .map_or("-".to_string(), |(id, v)| format!("{id}@{v}"));
        let head = format!(
            "sched/1\nstep:{}\nalgorithm:{algorithm}\nparams:{}\nprogram:{}\n",
            inst.step_ref(),
            resolved.params_sha256(),
            self.program
        );
        let shape = Shape {
            k,
            step,
            inst,
            head,
            driver: driver.as_ref().map(|d| d.0),
            items,
            recipe,
        };
        match &shape.items {
            Some(v) => v.iter().for_each(|n| self.job(&shape, Some(n))),
            None => self.job(&shape, None),
        }
        self.built[k].items = shape.items;
        self.built[k].origin = origin;
        self.built[k].recipe = recipe;
    }

    /// The plan inputs the data of `inst` comes from, in the order of `plan.inputs`.
    fn origin(&self, inst: &Instance) -> Vec<String> {
        let mut origin: Vec<String> = Vec::new();
        for r in inst.ins.iter().flat_map(|(_, refs)| refs) {
            let from: Vec<String> = match r {
                Ref::File(n) if self.plan.input(n).is_some() => vec![n.clone()],
                Ref::Out { instance, .. } => self
                    .plan
                    .steps
                    .iter()
                    .position(|x| x.id == *instance)
                    .map(|s| self.built[s].origin.clone())
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            for n in from {
                if !origin.contains(&n) {
                    origin.push(n);
                }
            }
        }
        crate::sortc::slice_by_key(&mut origin, |n| position(self.plan, n));
        origin
    }

    /// One job of an instance: its key text (what it reads, in the order the step declares its ports) and the jobs it waits for.
    fn job(&mut self, sh: &Shape, item: Option<&String>) {
        let mut text = sh.head.clone();
        // the name keeps two files with the same fingerprint (an index still loading) from sharing a key, and with it the file the job's result is kept in
        if let Some(n) = item {
            text.push_str(&format!("item:{n}\n"));
        }
        let mut deps: Vec<usize> = Vec::new();
        for p in &sh.step.ins {
            text.push_str(&format!("in:{}\n", p.name));
            let driving = sh.items.is_some() && Some(p.name) == sh.driver;
            for r in refs_of(sh.inst, p.name) {
                match r {
                    Ref::File(n) => {
                        if self.plan.input(n).is_some() && (!driving || item == Some(n)) {
                            text.push_str(&format!("f:{}\n", (self.identity)(n)));
                        }
                    }
                    Ref::Out { instance, port } => {
                        let Some(s) = self.plan.steps.iter().position(|x| x.id == *instance) else {
                            continue;
                        };
                        for &j in &self.built[s].jobs {
                            let mine = self.nodes[j].id.item.as_ref();
                            if driving && self.built[s].items.is_some() && mine != item {
                                continue;
                            }
                            text.push_str(&format!("k:{port}:{}\n", self.nodes[j].key));
                            if !deps.contains(&j) {
                                deps.push(j);
                            }
                        }
                    }
                }
            }
        }
        if let Some(j) = sh.recipe {
            text.push_str(&format!("r:{}\n", self.nodes[j].key));
            if !deps.contains(&j) {
                deps.push(j);
            }
        }
        let depth = deps
            .iter()
            .map(|&j| self.nodes[j].depth + 1)
            .max()
            .unwrap_or(0);
        self.built[sh.k].jobs.push(self.nodes.len());
        let node = Node {
            id: JobId::new(&sh.inst.id, item.map(String::as_str)),
            inst: sh.k,
            key: sha256_hex(text.as_bytes()),
            deps: deps.iter().map(|&j| self.nodes[j].id.clone()).collect(),
            depth,
            item_order: item.map_or(usize::MAX, |n| position(self.plan, n)),
            invalid: false,
        };
        self.nodes.push(node);
    }
}

/// A job: the instance and, for a `per_item` step, the NAME of the plan input that drives it (never an index, so adding a file does not renumber).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobId {
    pub instance: String,
    pub item: Option<String>,
}

impl JobId {
    pub fn new(instance: &str, item: Option<&str>) -> JobId {
        JobId {
            instance: instance.to_string(),
            item: item.map(str::to_string),
        }
    }
}

/// Steps whose recipe is estimated from the first index of the series (`recipe_of_layers`): their jobs depend on the index job of the first input.
const RECIPE_STEPS: [&str; 3] = ["features.detect", "features.align", "features.gapfill"];

/// One job of the graph, in dependency order.
pub(super) struct Node {
    pub id: JobId,
    /// Position of the instance in `plan.steps` (the tie-break of `next`).
    pub inst: usize,
    pub key: String,
    pub deps: Vec<JobId>,
    pub depth: usize,
    /// Position of the item in `plan.inputs` (`usize::MAX` for a job with no item).
    pub item_order: usize,
    /// The step or its parameters did not resolve: the job cannot run until the plan is fixed.
    pub invalid: bool,
}

/// What a built instance offers to the instances that read it.
#[derive(Default)]
struct Built {
    /// Positions in the node list.
    jobs: Vec<usize>,
    /// Item names of its `many` outputs when it is `per_item` with a known driver; None = one job, count of outputs unknown.
    items: Option<Vec<String>>,
    /// Plan inputs the data comes from, in the order of `plan.inputs`.
    origin: Vec<String>,
    /// The index job that the recipe of an HR step is estimated from.
    recipe: Option<usize>,
}

/// What a port reads, by name: Some(names) or None when a source has an unknown number of items.
fn item_names(
    plan: &Plan,
    steps: &[Option<&Step>],
    built: &[Built],
    refs: &[Ref],
) -> Option<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    for r in refs {
        let found: Vec<String> = match r {
            Ref::File(n) => plan.input(n).map(|_| n.clone()).into_iter().collect(),
            Ref::Out { instance, port } => {
                let Some(s) = plan.steps.iter().position(|x| x.id == *instance) else {
                    continue;
                };
                let many =
                    steps[s].is_some_and(|st| st.outs.iter().any(|p| p.name == port && p.many));
                match (&built[s].items, many) {
                    (Some(v), true) => v.clone(),
                    _ => return None,
                }
            }
        };
        for n in found {
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    Some(names)
}

fn refs_of<'a>(inst: &'a Instance, port: &str) -> &'a [Ref] {
    inst.ins
        .iter()
        .find(|(p, _)| p == port)
        .map_or(&[][..], |(_, r)| r.as_slice())
}

fn position(plan: &Plan, name: &str) -> usize {
    plan.inputs
        .iter()
        .position(|f| f.name == name)
        .unwrap_or(usize::MAX)
}

/// What the build reads while it walks the instances in order.
struct Walk<'a> {
    plan: &'a Plan,
    steps: Vec<Option<&'a Step>>,
    program: &'a str,
    identity: &'a dyn Fn(&str) -> String,
    built: Vec<Built>,
    nodes: Vec<Node>,
}

/// What one instance shares among its jobs.
struct Shape<'a> {
    k: usize,
    step: &'a Step,
    inst: &'a Instance,
    /// SHA-256 of the parameters and the algorithm line, as written in the key text.
    head: String,
    driver: Option<&'a str>,
    items: Option<Vec<String>>,
    recipe: Option<usize>,
}

/// Builds the nodes of `plan`; an error is a cycle. `identity(name)` is the fingerprint of a plan input.
pub(super) fn build(
    plan: &Plan,
    registry: &[Step],
    program: &str,
    identity: &dyn Fn(&str) -> String,
) -> Result<Vec<Node>> {
    let order = plan_order(plan)?;
    let steps = plan
        .steps
        .iter()
        .map(|i| {
            registry
                .iter()
                .find(|s| s.id == i.step && s.version == i.version)
        })
        .collect();
    let built = plan.steps.iter().map(|_| Built::default()).collect();
    let mut w = Walk {
        plan,
        steps,
        program,
        identity,
        built,
        nodes: Vec::new(),
    };
    for k in order {
        w.instance(k);
    }
    Ok(w.nodes)
}

/// The index job of the first input of the series: the one on the `index` port of the step, or the one its HR source already depends on.
fn recipe_job(
    plan: &Plan,
    built: &[Built],
    nodes: &[Node],
    inst: &Instance,
    first: Option<&String>,
) -> Option<usize> {
    let source = |r: &Ref| match r {
        Ref::Out { instance, .. } => plan.steps.iter().position(|x| x.id == *instance),
        Ref::File(_) => None,
    };
    for r in refs_of(inst, "index") {
        if let Some(s) = source(r) {
            let pick = built[s]
                .jobs
                .iter()
                .copied()
                .find(|&j| built[s].items.is_none() || nodes[j].id.item.as_ref() == first);
            if pick.is_some() {
                return pick;
            }
        }
    }
    inst.ins
        .iter()
        .flat_map(|(_, refs)| refs.iter())
        .filter_map(source)
        .find_map(|s| built[s].recipe)
}
