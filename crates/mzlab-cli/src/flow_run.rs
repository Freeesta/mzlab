//! `mzlab-cli flow run PLAN.json`: a whole `mzflow/2` plan on files, with every parameter of the descriptors reachable by `--set`, the inputs found by
//! name with `--in`, one progress line per step on stderr and the run folder (`plan.json`, the files of each step, `prov.json`, `corsa.json`).
//! Exit codes: 0 done, 1 some step failed, 2 usage, 3 plan not valid, 4 input files. Rules: `docs/agenti/cli.md`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mzlab_core::budget::sha256_hex;
use mzlab_core::flow::{
    canonical, execute, parse, plan_order, prov_json, Agent, Job, Plan, Platform, Problem, Run,
};
use mzlab_core::json::Value;
use mzlab_core::step::{Kind, Step};

use crate::flow::{executable, steps, DoneFn, FileRunner, Meter};
use crate::Outcome;

pub(crate) const HELP: &str = "mzlab-cli flow run: runs a whole mzflow/2 plan (the plan of the Flow page) on files.

usage: mzlab-cli flow run PLAN.json [--in FILE|FOLDER...] [--out FOLDER] [--set INSTANCE.NAME=VALUE...] [--workers N] [--budget-mb N] [--dry-run] [--fresh]

  PLAN.json                the plan; its inputs are files named in it
  --in FILE|FOLDER...      where the inputs are: a file is matched by its name, a folder holds files with those names (default: the folder of the plan)
  --out FOLDER             the run folder: plan.json (the plan as run), one file per step, prov.json and corsa.json
  --set INSTANCE.NAME=V    a parameter of a step instance (INSTANCE is its id, or the step id when the plan has one instance of it);
                           NAME may also be `algorithm` or `preset`; repeat for more
  --workers N              threads for the steps of a level and the files of a per-file step (default 1; the outputs do not depend on it)
  --budget-mb N            working memory of the alignment in MB (default: a share of the RAM); a series that does not fit in memory is aligned
                           in slabs of m/z, and the outputs are the same bytes whatever N is
  --dry-run                checks the plan, the settings and the inputs and prints what would run; writes nothing
  --fresh                  computes every step again; without it a step whose inputs, parameters and program are those of a run already in
                           FOLDER is reused (`... reused`, `cached` in prov.json)
  --help                   this text

exit codes: 0 done, 1 a step failed (the others are written), 2 usage, 3 the plan is not valid, 4 an input is missing or does not match";

/// Arguments of `flow run`.
#[derive(Default)]
struct Args {
    plan: Option<PathBuf>,
    ins: Vec<PathBuf>,
    out: Option<PathBuf>,
    sets: Vec<String>,
    workers: usize,
    dry: bool,
    fresh: bool,
    budget_mb: Option<f64>,
}

fn parse_args(args: Vec<String>) -> Result<Option<Args>, String> {
    let mut a = Args {
        workers: 1,
        ..Args::default()
    };
    let mut it = args.into_iter().peekable();
    while let Some(x) = it.next() {
        let mut value = |what: &str| it.next().ok_or(format!("{x} needs {what}"));
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => a.out = Some(value("a folder")?.into()),
            "--set" => a.sets.push(value("INSTANCE.NAME=VALUE")?),
            "--workers" => {
                let n = value("a number")?;
                a.workers = n
                    .parse()
                    .ok()
                    .filter(|&n| n > 0)
                    .ok_or(format!("--workers {n}: not a positive number"))?;
            }
            "--budget-mb" => {
                let n = value("a number of MB")?;
                a.budget_mb = Some(
                    n.parse()
                        .ok()
                        .filter(|&n: &f64| n.is_finite() && n > 0.0)
                        .ok_or(format!("--budget-mb {n}: not a positive number"))?,
                );
            }
            "--dry-run" => a.dry = true,
            "--fresh" => a.fresh = true,
            "--in" => {
                a.ins.push(value("a file or a folder")?.into());
                while let Some(more) = it.next_if(|n| !n.starts_with("--")) {
                    a.ins.push(more.into());
                }
            }
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            _ if a.plan.is_none() => a.plan = Some(x.into()),
            _ => return Err(format!("one plan only ({x}); inputs go after --in")),
        }
    }
    if a.plan.is_none() {
        return Err("no plan file".into());
    }
    if a.out.is_none() && !a.dry {
        return Err("--out FOLDER is needed (or --dry-run)".into());
    }
    Ok(Some(a))
}

/// The value of `text` read with the type of `kind`.
fn typed(kind: &Kind, text: &str) -> Option<Value> {
    let num = |s: &str| s.trim().parse::<f64>().ok().map(Value::Num);
    match kind {
        Kind::Number | Kind::Integer => num(text),
        Kind::Bool => match text {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            _ => None,
        },
        Kind::Text => Some(Value::Str(text.into())),
        Kind::Pair(_) => {
            let (a, b) = text.split_once(',')?;
            Some(Value::Arr(vec![num(a)?, num(b)?]))
        }
        Kind::Choice(list) => {
            let as_num = list.iter().any(|v| matches!(v, Value::Num(_)));
            if as_num {
                num(text)
            } else {
                Some(Value::Str(text.into()))
            }
        }
    }
}

/// The instance `who` names: its id, or the id of a step the plan has once.
fn instance_index(plan: &Plan, who: &str) -> Result<usize, String> {
    if let Some(k) = plan.steps.iter().position(|s| s.id == who) {
        return Ok(k);
    }
    let of_step: Vec<usize> = (0..plan.steps.len())
        .filter(|&k| plan.steps[k].step == who)
        .collect();
    match of_step.as_slice() {
        [k] => Ok(*k),
        [] => {
            let ids: Vec<&str> = plan.steps.iter().map(|s| s.id.as_str()).collect();
            Err(format!(
                "--set {who}: no instance or step of that name (instances: {})",
                ids.join(", ")
            ))
        }
        _ => Err(format!(
            "--set {who}: the plan has more than one instance of that step; use the instance id"
        )),
    }
}

/// Applies one `--set` (`INSTANCE.NAME=VALUE`) to `plan`, its value typed by the descriptor.
pub(crate) fn apply_set(plan: &mut Plan, reg: &[Step], set: &str) -> Result<(), String> {
    let bad = || format!("--set {set}: expected INSTANCE.NAME=VALUE");
    let (lhs, text) = set.split_once('=').ok_or_else(bad)?;
    let (who, name) = lhs.rsplit_once('.').ok_or_else(bad)?;
    let k = instance_index(plan, who)?;
    let inst = &mut plan.steps[k];
    match name {
        "algorithm" => inst.algorithm = Some(text.into()),
        "preset" => inst.preset = Some(text.into()),
        _ => {
            let step = reg
                .iter()
                .find(|s| s.id == inst.step && s.version == inst.version);
            let step = step.ok_or(format!("--set {set}: unknown step {}", inst.step_ref()))?;
            let alg = inst.algorithm.as_deref().or(step.default_algorithm);
            let param = step
                .param(name)
                .or_else(|| alg.and_then(|a| step.algorithm_param(a, name)));
            let Some(param) = param else {
                let own = step.params.iter().map(|p| p.name);
                let of_alg = step
                    .algorithms
                    .iter()
                    .filter(|a| Some(a.id) == alg)
                    .flat_map(|a| a.params.iter().map(|p| p.name));
                let names: Vec<&str> = own.chain(of_alg).collect();
                return Err(format!(
                    "--set {set}: {} has no parameter {name} (parameters: {})",
                    inst.id,
                    names.join(", ")
                ));
            };
            let v = typed(&param.kind, text)
                .ok_or(format!("--set {set}: {text} is not a value of {name}"))?;
            inst.params.retain(|(n, _)| n != name);
            inst.params.push((name.into(), v));
        }
    }
    Ok(())
}

/// Where each input of `plan` is: a `--in` file of that name, else a `--in` folder that has it, else the folder of the plan.
fn locate(
    plan: &Plan,
    ins: &[PathBuf],
    plan_dir: &Path,
) -> Result<HashMap<String, PathBuf>, String> {
    let mut found = HashMap::new();
    for f in &plan.inputs {
        let files = ins.iter().filter(|p| {
            p.is_file() && p.file_name().is_some_and(|n| n.to_string_lossy() == f.name)
        });
        let dirs = ins
            .iter()
            .filter(|p| p.is_dir())
            .map(|d| d.join(&f.name))
            .filter(|p| p.exists());
        let mut hits: Vec<PathBuf> = files.cloned().chain(dirs).collect();
        if ins.is_empty() && plan_dir.join(&f.name).exists() {
            hits.push(plan_dir.join(&f.name));
        }
        match hits.len() {
            1 => found.insert(f.name.clone(), hits.remove(0)),
            0 => {
                return Err(format!(
                    "input {} not found{}",
                    f.name,
                    if ins.is_empty() {
                        " beside the plan"
                    } else {
                        " in the --in paths"
                    }
                ))
            }
            _ => {
                return Err(format!(
                    "input {} found more than once: {}",
                    f.name,
                    hits.iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            }
        };
    }
    Ok(found)
}

/// The plan ready to run: comparisons checked as written, then expanded, then the whole plan checked again, every step with a native executor;
/// else the problems. A comparison that cannot be expanded is the error itself.
pub(crate) fn prepare(
    plan: Plan,
    reg: &[Step],
) -> mzlab_core::Result<std::result::Result<Plan, Vec<Problem>>> {
    let problems = crate::flow_compare::check(&plan, reg, Platform::current());
    if !problems.is_empty() {
        return Ok(Err(problems));
    }
    let plan = crate::flow_compare::expand(plan, reg)?;
    let mut problems = mzlab_core::flow::check(&plan, reg, Platform::current());
    for s in plan.steps.iter().filter(|s| !executable(&s.step)) {
        problems.push(Problem {
            instance: s.id.clone(),
            port: String::new(),
            error_key: "err.flow.step.native",
            params: vec![("step", s.step.clone())],
        });
    }
    Ok(if problems.is_empty() {
        Ok(plan)
    } else {
        Err(problems)
    })
}

/// `instance.port:key` for each problem, the form of `err.flow.check`.
pub(crate) fn problem_text(p: &Problem) -> String {
    let dot = if p.port.is_empty() { "" } else { "." };
    format!("{}{dot}{}:{}", p.instance, p.port, p.error_key)
}

/// The program that ran: version of the crate and, when the build knows it (`MZLAB_COMMIT`), the commit.
fn agent() -> Agent {
    Agent {
        name: "mzLab".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        commit: option_env!("MZLAB_COMMIT").map(str::to_string),
        platform: Platform::current().name(),
    }
}

/// Runs a checked `plan` with `runner`; returns the run and its `prov/1` text (`plan_sha256` of the text the plan was read from).
pub(crate) fn run_checked(
    plan: &Plan,
    text: &[u8],
    runner: &FileRunner,
    workers: usize,
) -> mzlab_core::Result<(Run, String)> {
    let run = execute(plan, &steps(), runner, workers)?;
    runner.save_cache()?;
    let prov = prov_json(&agent(), Some(&sha256_hex(text)), &run);
    Ok((run, prov))
}

/// The progress line of one job: `[3/9] features.table (feat) ... done in 12.4 s` (`item 2` for a per-file step), `... reused` when it came from
/// the cache, or the error.
pub(crate) fn progress_line(
    order: &[String],
    job: &Job,
    ms: f64,
    cached: bool,
    err: Option<&mzlab_core::Error>,
) -> String {
    let n = order
        .iter()
        .position(|id| *id == job.instance.id)
        .map_or(0, |k| k + 1);
    let item = job
        .item
        .map(|k| format!(" item {}", k + 1))
        .unwrap_or_default();
    let what = format!(
        "[{n}/{}] {} ({}){item}",
        order.len(),
        job.step.id,
        job.instance.id
    );
    match err {
        None if cached => format!("{what} ... reused"),
        None => format!("{what} ... done in {:.1} s", ms / 1e3),
        Some(e) => format!("{what} ... failed: {e}"),
    }
}

pub(crate) fn fail(code: u8, msg: &str, hint: &str) -> Outcome {
    Outcome {
        code,
        stdout: Vec::new(),
        stderr: format!("error: {msg}\nhint: {hint}\n"),
    }
}

/// What `--dry-run` prints: the inputs where they were found and every instance in order with its canonical parameters.
fn dry_text(plan: &Plan, reg: &[Step], found: &HashMap<String, PathBuf>) -> String {
    let mut s = String::from("inputs:\n");
    for f in &plan.inputs {
        let at = found
            .get(&f.name)
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        s += &format!("  {}  {at}  ({} bytes)\n", f.name, f.bytes.unwrap_or(0));
    }
    s += "steps, in the order they run:\n";
    let order = plan_order(plan).unwrap_or_default();
    for (k, &i) in order.iter().enumerate() {
        let inst = &plan.steps[i];
        let step = reg
            .iter()
            .find(|x| x.id == inst.step && x.version == inst.version);
        let params = step
            .and_then(|st| canonical(inst, st).ok())
            .map(|r| r.text())
            .unwrap_or_default();
        s += &format!(
            "  [{}/{}] {} ({}) {params}\n",
            k + 1,
            order.len(),
            inst.step_ref(),
            inst.id
        );
    }
    s
}

/// The plan of `a` as it will run: read, `--set` applied, inputs found and hashed; the plan as written and its checked, expanded form.
type Ready = (Plan, Plan, HashMap<String, PathBuf>);

fn ready(a: &Args, reg: &[Step]) -> Result<Ready, Outcome> {
    let usage = "mzlab-cli flow run --help";
    let fix = "fix the plan in the Flow page or with --set";
    let plan_path = a.plan.clone().unwrap_or_default();
    let text = std::fs::read(&plan_path).map_err(|e| {
        fail(
            4,
            &format!("{}: {e}", plan_path.display()),
            "check the path of the plan",
        )
    })?;
    let mut plan = parse(&text).map_err(|e| {
        fail(
            3,
            &e.to_string(),
            "the plan must be the mzflow/2 file the Flow page downloads",
        )
    })?;
    let (setting, plain): (Vec<&String>, Vec<&String>) = a
        .sets
        .iter()
        .partition(|s| s.contains(".algorithm=") || s.contains(".preset="));
    for s in setting.into_iter().chain(plain) {
        apply_set(&mut plan, reg, s).map_err(|e| fail(2, &e, usage))?;
    }
    let plan_dir = plan_path
        .parent()
        .map_or(PathBuf::from("."), Path::to_path_buf);
    let found = locate(&plan, &a.ins, &plan_dir).map_err(|e| {
        fail(
            4,
            &e,
            "pass the file, or the folder that holds it, with --in",
        )
    })?;
    let hint = "the file differs from the one the plan names (sha256); remove the sha256 from the plan to accept it";
    fill_inputs(&mut plan, |n| found[n].clone()).map_err(|e| fail(4, &e.to_string(), hint))?;
    match prepare(plan.clone(), reg) {
        Ok(Ok(r)) => Ok((plan, r, found)),
        Ok(Err(problems)) => {
            let list: Vec<String> = problems
                .iter()
                .map(|p| format!("  {}", problem_text(p)))
                .collect();
            Err(fail(
                3,
                &format!("the plan is not valid:\n{}", list.join("\n")),
                fix,
            ))
        }
        Err(e) => Err(fail(3, &e.to_string(), fix)),
    }
}

/// `mzlab-cli flow run ARGS...` (the arguments after `flow run`).
pub fn run(args: Vec<String>, meter: Option<Meter>) -> Outcome {
    let a = match parse_args(args) {
        Ok(Some(a)) => a,
        Ok(None) => {
            return Outcome {
                code: 0,
                stdout: format!("{HELP}\n").into_bytes(),
                stderr: String::new(),
            }
        }
        Err(e) => return fail(2, &e, "mzlab-cli flow run --help"),
    };
    let reg = steps();
    let (plan, ready, found) = match ready(&a, &reg) {
        Ok(r) => r,
        Err(o) => return o,
    };
    if a.dry {
        let stdout = dry_text(&ready, &reg, &found).into_bytes();
        return Outcome {
            code: 0,
            stdout,
            stderr: String::new(),
        };
    }
    let out = a.out.unwrap_or_default();
    let opts = (a.fresh, a.budget_mb);
    crate::flow_out::write_run(&plan, &ready, found, out, a.workers, meter, opts)
}

/// The inputs of the plan that name no hash get it from the file `locate` gives; one that names a hash must match it.
pub fn fill_inputs(plan: &mut Plan, locate: impl Fn(&str) -> PathBuf) -> mzlab_core::Result<()> {
    for f in &mut plan.inputs {
        let bytes = std::fs::read(locate(&f.name))
            .map_err(|e| crate::flow::io(e).with("input", &f.name))?;
        let sha = sha256_hex(&bytes);
        if !f.sha256.is_empty() && f.sha256 != sha {
            return Err(mzlab_core::Error::new("err.flow.input.sha").with("input", &f.name));
        }
        f.sha256 = sha;
        f.bytes = Some(bytes.len() as u64);
    }
    Ok(())
}

/// Reads, checks and runs a workflow file; returns the run and its `prov/1` text. Files of the plan are read from `dir`, outputs written to `work`.
pub fn run_plan(
    text: &[u8],
    dir: &Path,
    work: &Path,
    workers: usize,
    meter: Option<Meter>,
) -> mzlab_core::Result<(Run, String)> {
    run_plan_cached(text, dir, work, workers, meter, None)
}

/// `run_plan` that keeps the cache of `work` (`Some(fresh)`; `fresh` recomputes every step): a step whose key is there is taken again (`cached`).
pub fn run_plan_cached(
    text: &[u8],
    dir: &Path,
    work: &Path,
    workers: usize,
    meter: Option<Meter>,
    fresh: Option<bool>,
) -> mzlab_core::Result<(Run, String)> {
    run_plan_notify(text, dir, work, workers, meter, fresh, None)
}

/// `run_plan_cached` that also calls `done` after each job, for a caller that shows the run while it goes.
pub fn run_plan_notify(
    text: &[u8],
    dir: &Path,
    work: &Path,
    workers: usize,
    meter: Option<Meter>,
    fresh: Option<bool>,
    done: Option<DoneFn>,
) -> mzlab_core::Result<(Run, String)> {
    let mut plan = parse(text)?;
    fill_inputs(&mut plan, |n| dir.join(n))?;
    let plan = prepare(plan, &steps())?.map_err(|problems| {
        let list: Vec<String> = problems.iter().map(problem_text).collect();
        mzlab_core::Error::new("err.flow.check").with("problems", list.join(" "))
    })?;
    let runner = FileRunner::cached(dir, work, meter, fresh).with_done(done);
    run_checked(&plan, text, &runner, workers)
}
