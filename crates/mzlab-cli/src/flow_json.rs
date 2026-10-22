//! JSON answers of `mzlab-cli` for agents (`--json`): `flowcheck/1`, `flowdry/1`, `flowrun/1`, `errore/1` and the JSONL progress line, written beside
//! the text forms of `flow_run.rs` and `flow_out.rs`, which stay as they are (stderr never changes). Also `mzlab-cli flow check`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mzlab_core::error::Error;
use mzlab_core::flow::{canonical, parse, plan_order, Job, Plan, Problem, Run};
use mzlab_core::json::Out;
use mzlab_core::step::Step;

use crate::flow::steps;
use crate::flow_run::{apply_set, prepare, problem_text};
use crate::Outcome;

/// Last element of the instance order that asks `progress_line` for JSONL: the instances come first, so their positions do not move.
pub(crate) const JSONL_MARK: &str = "\u{1}jsonl";

/// How a command answers: `json` puts one document on stdout; `jsonl` writes the progress lines of a run as JSON.
#[derive(Clone, Copy, Default)]
pub(crate) struct Style {
    pub json: bool,
    pub jsonl: bool,
    /// `--jsonl`: the `flowlive/1` events of the run on stdout.
    pub events: bool,
}

fn params_obj(o: &mut Out, params: &[(&str, String)]) {
    o.raw("{");
    for (k, (name, v)) in params.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .str(name)
            .raw(":")
            .str(v);
    }
    o.raw("}");
}

fn problem(o: &mut Out, p: &Problem) {
    o.raw("{\"instance\":")
        .str(&p.instance)
        .raw(",\"port\":")
        .str(&p.port)
        .raw(",\"error_key\":")
        .str(p.error_key)
        .raw(",\"params\":");
    params_obj(o, &p.params);
    o.raw("}");
}

fn problem_list(o: &mut Out, problems: &[Problem]) {
    o.raw("[");
    for (k, p) in problems.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" });
        problem(o, p);
    }
    o.raw("]");
}

/// `flowcheck/1`: whether the plan is valid and every problem, in the order `flow::check` gives them.
pub(crate) fn check_json(problems: &[Problem]) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"flowcheck/1\",\"valid\":")
        .raw(if problems.is_empty() { "true" } else { "false" })
        .raw(",\"problems\":");
    problem_list(&mut o, problems);
    o.raw("}\n");
    o.0
}

/// `errore/1`: the exit code, the key and parameters of the error and the hint; `problems` (plan not valid) only when there are some.
fn error_json(code: u8, err: &Error, message: &str, hint: &str, problems: &[Problem]) -> String {
    let params: Vec<(&str, String)> = err.params.iter().map(|(k, v)| (*k, v.clone())).collect();
    let mut o = Out::default();
    o.raw("{\"schema\":\"errore/1\",\"code\":")
        .int(i64::from(code))
        .raw(",\"error_key\":")
        .str(err.key)
        .raw(",\"params\":");
    params_obj(&mut o, &params);
    o.raw(",\"message\":")
        .str(message)
        .raw(",\"hint\":")
        .str(hint);
    if !problems.is_empty() {
        o.raw(",\"problems\":");
        problem_list(&mut o, problems);
    }
    o.raw("}\n");
    o.0
}

/// The error outcome of `flow_run::fail` (stderr as ever); with `--json` the `errore/1` document is also on stdout.
pub(crate) fn fail(st: Style, code: u8, err: &Error, msg: &str, hint: &str) -> Outcome {
    let mut o = crate::flow_run::fail(code, msg, hint);
    if st.json {
        o.stdout = error_json(code, err, msg, hint, &[]).into_bytes();
    }
    o
}

/// Exit 3 for a plan that is not valid: one problem per line on stderr, and with `--json` the `errore/1` document with the list.
pub(crate) fn invalid(st: Style, problems: &[Problem]) -> Outcome {
    let list: Vec<String> = problems.iter().map(problem_text).collect();
    let lines: Vec<String> = list.iter().map(|l| format!("  {l}")).collect();
    let hint = "fix the plan in the Flow page or with --set";
    let msg = format!("the plan is not valid:\n{}", lines.join("\n"));
    let mut o = crate::flow_run::fail(3, &msg, hint);
    if st.json {
        let err = Error::new("err.flow.check").with("problems", list.join(" "));
        o.stdout = error_json(3, &err, &msg, hint, problems).into_bytes();
    }
    o
}

/// `flowdry/1`: the inputs where they were found and every instance in the order it runs, with its canonical parameters.
pub(crate) fn dry_json(plan: &Plan, reg: &[Step], found: &HashMap<String, PathBuf>) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"flowdry/1\",\"inputs\":[");
    for (k, f) in plan.inputs.iter().enumerate() {
        let at = found.get(&f.name).map(|p| p.display().to_string());
        o.raw(if k > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(&f.name)
            .raw(",\"path\":")
            .str(&at.unwrap_or_default())
            .raw(",\"bytes\":")
            .int(f.bytes.unwrap_or(0) as i64)
            .raw(",\"sha256\":")
            .str(&f.sha256)
            .raw("}");
    }
    o.raw("],\"steps\":[");
    let order = plan_order(plan).unwrap_or_default();
    for (k, &i) in order.iter().enumerate() {
        let inst = &plan.steps[i];
        let step = reg
            .iter()
            .find(|x| x.id == inst.step && x.version == inst.version);
        let resolved = step.and_then(|st| canonical(inst, st).ok());
        o.raw(if k > 0 { ",{\"n\":" } else { "{\"n\":" })
            .int(k as i64 + 1)
            .raw(",\"of\":")
            .int(order.len() as i64)
            .raw(",\"instance\":")
            .str(&inst.id)
            .raw(",\"step\":")
            .str(&inst.step_ref())
            .raw(",\"params\":")
            .raw(&resolved.as_ref().map_or("{}".into(), |r| r.text()))
            .raw(",\"params_sha256\":")
            .str(&resolved.map_or(String::new(), |r| r.params_sha256()))
            .raw("}");
    }
    o.raw("]}\n");
    o.0
}

fn outputs(o: &mut Out, run: &Run, instance: &str, generated: &[String]) {
    o.raw("[");
    let mut first = true;
    for key in generated {
        let Some((_, a)) = run.entities.iter().find(|(k, _)| k == key) else {
            continue;
        };
        o.raw(if first { "{\"port\":" } else { ",{\"port\":" })
            .str(key.strip_prefix(&format!("{instance}.")).unwrap_or(key))
            .raw(",\"file\":")
            .str(a.label.as_deref().unwrap_or(""))
            .raw(",\"sha256\":")
            .str(&a.sha256)
            .raw(",\"bytes\":")
            .int(a.bytes as i64)
            .raw("}");
        first = false;
    }
    o.raw("]");
}

/// `flowrun/1`: how a run ended. Per instance the status (`ok|cached|error|skipped`), `ms` and the files it wrote with their SHA-256, as `prov/1`
/// has them; the instances that did not run come last.
pub(crate) fn run_json(code: u8, out: &Path, plan_sha256: &str, run: &Run) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"flowrun/1\",\"code\":")
        .int(i64::from(code))
        .raw(",\"out\":")
        .str(&out.display().to_string())
        .raw(",\"plan_sha256\":")
        .str(plan_sha256)
        .raw(",\"steps\":[");
    for (k, a) in run.activities.iter().enumerate() {
        let status = match (&a.error, a.cached) {
            (Some(_), _) => "error",
            (None, true) => "cached",
            _ => "ok",
        };
        o.raw(if k > 0 {
            ",{\"instance\":"
        } else {
            "{\"instance\":"
        })
        .str(&a.instance)
        .raw(",\"step\":")
        .str(&a.step)
        .raw(",\"status\":")
        .str(status)
        .raw(",\"ms\":")
        .num((a.ms * 10.0).round() / 10.0)
        .raw(",\"outputs\":");
        outputs(&mut o, run, &a.instance, &a.generated);
        if let Some(e) = &a.error {
            let params: Vec<(&str, String)> = e
                .params
                .iter()
                .map(|(k, v)| (k.as_str(), v.clone()))
                .collect();
            o.raw(",\"error\":{\"error_key\":")
                .str(&e.error_key)
                .raw(",\"params\":");
            params_obj(&mut o, &params);
            o.raw("}");
        }
        o.raw("}");
    }
    for (k, id) in run.skipped.iter().enumerate() {
        let comma = if k > 0 || !run.activities.is_empty() {
            ","
        } else {
            ""
        };
        o.raw(comma)
            .raw("{\"instance\":")
            .str(id)
            .raw(",\"status\":\"skipped\",\"ms\":0,\"outputs\":[]}");
    }
    o.raw("]}\n");
    o.0
}

/// One JSONL progress line of a job: `{"job":3,"of":9,"instance":…,"item":2,"status":"ok|cached|error","ms":12.4}`; `item` is the 1-based file of a
/// per-file step, else null; a failed job adds `error_key`.
pub(crate) fn progress_jsonl(
    order: &[String],
    job: &Job,
    ms: f64,
    cached: bool,
    err: Option<&Error>,
) -> String {
    let total = order.len() - usize::from(order.last().is_some_and(|x| x == JSONL_MARK));
    let n = order
        .iter()
        .position(|id| *id == job.instance.id)
        .map_or(0, |k| k + 1);
    let mut o = Out::default();
    o.raw("{\"job\":")
        .int(n as i64)
        .raw(",\"of\":")
        .int(total as i64)
        .raw(",\"instance\":")
        .str(&job.instance.id)
        .raw(",\"item\":")
        .raw(&job.item.map_or("null".into(), |k| (k + 1).to_string()))
        .raw(",\"status\":")
        .str(match (err, cached) {
            (Some(_), _) => "error",
            (None, true) => "cached",
            _ => "ok",
        })
        .raw(",\"ms\":")
        .num((ms * 10.0).round() / 10.0);
    if let Some(e) = err {
        o.raw(",\"error_key\":").str(e.key);
    }
    o.raw("}");
    o.0
}

const CHECK_HELP: &str = "mzlab-cli flow check: is a plan valid? Checks it as `flow run` does, without looking for the input files.

usage: mzlab-cli flow check PLAN.json [--set INSTANCE.NAME=VALUE...] [--json]

  --set INSTANCE.NAME=V    as in `flow run`; the setting is checked with the rest of the plan
  --json                   one flowcheck/1 document on stdout: {\"valid\", \"problems\": [{instance, port, error_key, params}]}
  --help                   this text

exit codes: 0 valid, 2 usage, 3 the plan is not valid (also a step that has no native executor: err.flow.step.native), 4 the plan cannot be read";

/// The plan file and the `--set` texts of `flow check`; the outcome to return when the arguments are not usable (or `--help`).
fn check_args(args: Vec<String>, st: Style) -> Result<(String, Vec<String>), Outcome> {
    let usage = |msg: &str| {
        let err = Error::new("err.cli.usage").with("detail", msg);
        fail(st, 2, &err, msg, "mzlab-cli flow check --help")
    };
    let (mut plan_path, mut sets) = (None, Vec::new());
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--help" | "-h" => {
                let help = format!("{CHECK_HELP}\n").into_bytes();
                return Err(Outcome {
                    code: 0,
                    stdout: help,
                    stderr: String::new(),
                });
            }
            "--json" => {}
            "--set" => match it.next() {
                Some(s) => sets.push(s),
                None => return Err(usage("--set needs INSTANCE.NAME=VALUE")),
            },
            s if s.starts_with("--") => return Err(usage(&format!("unknown option {s}"))),
            _ if plan_path.is_none() => plan_path = Some(x),
            _ => return Err(usage(&format!("one plan only ({x})"))),
        }
    }
    plan_path
        .map(|p| (p, sets))
        .ok_or_else(|| usage("no plan file"))
}

/// The plan of `path` read and with the `--set` texts applied, or the outcome that says why not.
fn check_plan(path: &str, sets: &[String], reg: &[Step], st: Style) -> Result<Plan, Outcome> {
    let text = std::fs::read(path).map_err(|e| {
        let err = Error::new("err.cli.file").with("path", path);
        fail(
            st,
            4,
            &err,
            &format!("{path}: {e}"),
            "check the path of the plan",
        )
    })?;
    let hint = "the plan must be the mzflow/2 file the Flow page downloads";
    let mut plan = parse(&text).map_err(|e| fail(st, 3, &e, &e.to_string(), hint))?;
    let (setting, plain): (Vec<&String>, Vec<&String>) = sets
        .iter()
        .partition(|s| s.contains(".algorithm=") || s.contains(".preset="));
    for s in setting.into_iter().chain(plain) {
        let err = Error::new("err.cli.set").with("set", s);
        apply_set(&mut plan, reg, s)
            .map_err(|e| fail(st, 2, &err, &e, "mzlab-cli flow check --help"))?;
    }
    Ok(plan)
}

/// `mzlab-cli flow check ARGS...` (the arguments after `flow check`).
pub(crate) fn check(args: Vec<String>) -> Outcome {
    let st = Style {
        json: args.iter().any(|a| a == "--json"),
        ..Style::default()
    };
    let (path, sets) = match check_args(args, st) {
        Ok(r) => r,
        Err(o) => return o,
    };
    let reg = steps();
    let plan = match check_plan(&path, &sets, &reg, st) {
        Ok(p) => p,
        Err(o) => return o,
    };
    match prepare(plan, &reg) {
        Ok(Ok(_)) => {
            let stdout = if st.json {
                check_json(&[]).into_bytes()
            } else {
                Vec::new()
            };
            Outcome {
                code: 0,
                stdout,
                stderr: "ok: the plan is valid\n".into(),
            }
        }
        Ok(Err(problems)) => {
            let mut o = invalid(Style::default(), &problems);
            if st.json {
                o.stdout = check_json(&problems).into_bytes();
            }
            o
        }
        Err(e) => fail(
            st,
            3,
            &e,
            &e.to_string(),
            "fix the plan in the Flow page or with --set",
        ),
    }
}
