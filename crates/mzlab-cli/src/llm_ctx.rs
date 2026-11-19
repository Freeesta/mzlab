//! `mzlab-cli llm context RUNDIR` writes the `context/1` the assistant would get from a run folder; `mzlab-cli llm eval CASES_DIR --replay DIR` scores
//! the evaluation cases against their recordings (CI), `--provider ID --key-file F` is for trying a real model by hand (no transport yet).
//! The logic lives in `mzlab-llm`; this file only reads folders and prints.

use std::path::Path;

use mzlab_core::budget::sha256_hex;
use mzlab_core::flow::Platform;
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::Error;
use mzlab_llm::context::build;
use mzlab_llm::eval::{run, score, Case};
use mzlab_llm::fake::Replay;
use mzlab_llm::plan::problems;
use mzlab_llm::tools::{catalog, UI_TOOLS};

use crate::flow::{executable, steps};
use crate::Outcome;

fn json(path: &Path) -> Option<Value> {
    parse(&std::fs::read(path).ok()?).ok()
}

/// The plan as the assistant sees it: the SHA-256 of `plan.json`, its instances and the problems `flow::check` finds.
fn plan_source(dir: &Path) -> Option<Value> {
    let bytes = std::fs::read(dir.join("plan.json")).ok()?;
    let plan = parse(&bytes).ok()?;
    let found = problems(
        &Value::Obj(vec![("plan".into(), plan.clone())]),
        &steps(),
        Platform::Native,
    )
    .ok()?;
    let instances = plan
        .get("steps")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|s| {
            let get = |k: &str| s.get(k).cloned().unwrap_or(Value::Null);
            Value::Obj(vec![
                ("id".into(), get("id")),
                ("step".into(), get("step")),
                ("params".into(), get("params")),
            ])
        });
    Some(Value::Obj(vec![
        ("sha256".into(), Value::Str(sha256_hex(&bytes))),
        ("instances".into(), Value::Arr(instances.collect())),
        (
            "problems".into(),
            parse(found.1.as_bytes())
                .ok()?
                .get("problems")
                .cloned()
                .unwrap_or(Value::Null),
        ),
    ]))
}

fn sources(dir: &Path, hide: bool) -> Value {
    let mut o: Vec<(String, Value)> = vec![
        ("profile".into(), Value::Str("research".into())),
        ("hide_names".into(), Value::Bool(hide)),
    ];
    let run = json(&dir.join("report.json"));
    if let Some(c) = &run {
        let files = c.get("inputs").map_or(&[][..], Value::arr).iter().map(|i| {
            Value::Obj(vec![(
                "name".into(),
                i.get("name").cloned().unwrap_or(Value::Null),
            )])
        });
        o.push(("files".into(), Value::Arr(files.collect())));
        o.push(("run".into(), c.clone()));
    }
    o.extend(plan_source(dir).map(|p| ("plan".to_string(), p)));
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    names.sort();
    for p in names {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        let v = if ext == "json" { json(&p) } else { None };
        let format = v
            .as_ref()
            .and_then(|v| v.get("format").or_else(|| v.get("schema")))
            .and_then(Value::str)
            .map(String::from);
        match (ext, format.as_deref(), v) {
            ("csv", _, _) if !o.iter().any(|(k, _)| k == "features_csv") => {
                o.extend(
                    std::fs::read_to_string(&p)
                        .ok()
                        .map(|t| ("features_csv".to_string(), Value::Str(t))),
                );
            }
            ("json", Some("annot/1"), Some(v)) => o.push(("annot".into(), v)),
            ("json", Some("msn-graph/1"), Some(v)) => o.push(("msn".into(), v)),
            _ => {}
        }
    }
    Value::Obj(o)
}

pub(crate) fn context(args: Vec<String>) -> Outcome {
    let (mut dir, mut budget, mut hide) = (None, 8000u64, false);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--hide-names" => hide = true,
            "--budget" => match it.next().and_then(|b| b.parse().ok()) {
                Some(b) => budget = b,
                None => return Outcome::fail("--budget needs a number"),
            },
            x if !x.starts_with('-') && dir.is_none() => dir = Some(x.to_string()),
            x => return Outcome::fail(&format!("unknown option {x}")),
        }
    }
    let Some(dir) = dir else {
        return Outcome::fail("usage: mzlab-cli llm context RUNDIR [--budget N] [--hide-names]");
    };
    if !Path::new(&dir).is_dir() {
        return Outcome::fail_error(&mzlab_core::Error::new("err.cli.file").with("path", &dir));
    }
    match build(&sources(Path::new(&dir), hide), budget) {
        Ok(t) => Outcome::ok((t + "\n").into_bytes()),
        Err(e) => Outcome::fail_error(&e),
    }
}

fn cases(dir: &str) -> Result<Vec<Case>, Error> {
    let io = |p: &Path| {
        let p = p.display().to_string();
        move |e: std::io::Error| Error::new("err.cli.file").with("path", format!("{p}: {e}"))
    };
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(io(Path::new(dir)))?
        .flatten()
        .map(|e| e.path())
        .collect();
    files.retain(|p| p.extension().is_some_and(|e| e == "json"));
    files.sort();
    let read = |p: &Path| std::fs::read_to_string(p).map_err(io(p));
    files
        .iter()
        .map(|p| Case::parse(&read(p)?).map_err(|e| e.with("file", p.display().to_string())))
        .collect()
}

/// One case of `evalrun/1`: its id, the checks passed and the total, and the names of those that failed.
struct Scored {
    id: String,
    passed: usize,
    total: usize,
    failed: Vec<String>,
}

/// Every case against its recording: a file that cannot be read is an input error (4), a case or recording of the wrong format an integrity
/// error (3), a recording that does not answer its case a data error (1, as an `error/1`: the verdict document is only for scored cases).
fn evaluate(dir: &str, replay: &str) -> Result<Vec<Scored>, Error> {
    let mut v = Vec::new();
    for case in cases(dir)? {
        let path = Path::new(replay).join(format!("{}.jsonl", case.id));
        let reg = std::fs::read_to_string(&path).map_err(|e| {
            Error::new("err.cli.file").with("path", format!("{}: {e}", path.display()))
        })?;
        let mut rep =
            Replay::parse(&reg, false).map_err(|e| e.with("file", path.display().to_string()))?;
        let tools = catalog(&steps(), UI_TOOLS, &executable);
        let plan =
            run(&case, &mut rep, tools, steps()).map_err(|e| e.with("case", case.id.clone()))?;
        let s = score(&plan, &case);
        v.push(Scored {
            id: case.id.clone(),
            passed: s.passed(),
            total: s.checks.len(),
            failed: s
                .checks
                .iter()
                .filter(|(_, ok)| !ok)
                .map(|(n, _)| n.to_string())
                .collect(),
        });
    }
    Ok(v)
}

/// `evalrun/1`: the score of every case and the total; exit 1 when a check fails (the document stays on stdout, it is the verdict).
fn evalrun_json(cases: &[Scored]) -> String {
    let (p, t) = cases
        .iter()
        .fold((0, 0), |a, c| (a.0 + c.passed, a.1 + c.total));
    let mut o = Out::default();
    o.raw("{\"schema\":\"evalrun/1\",\"passed\":")
        .int(p as i64)
        .raw(",\"total\":")
        .int(t as i64)
        .raw(",\"percent\":")
        .int((100 * p).checked_div(t).unwrap_or(0) as i64)
        .raw(",\"cases\":[");
    for (k, c) in cases.iter().enumerate() {
        if k > 0 {
            o.raw(",");
        }
        o.raw("{\"id\":").str(&c.id);
        o.raw(",\"passed\":").int(c.passed as i64);
        o.raw(",\"total\":").int(c.total as i64);
        o.raw(",\"failed\":[");
        for (j, f) in c.failed.iter().enumerate() {
            if j > 0 {
                o.raw(",");
            }
            o.str(f);
        }
        o.raw("]}");
    }
    o.raw("]}\n");
    o.0
}

pub(crate) fn eval(args: Vec<String>) -> Outcome {
    let (mut dir, mut replay) = (None, None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--replay" => replay = it.next(),
            "--provider" | "--key-file" => {
                it.next();
                return Outcome::fail("err.llm.network: this build has no transport to a provider");
            }
            x if !x.starts_with('-') && dir.is_none() => dir = Some(x.to_string()),
            x => return Outcome::fail(&format!("unknown option {x}")),
        }
    }
    let (Some(dir), Some(replay)) = (dir, replay) else {
        return Outcome::fail("usage: mzlab-cli llm eval CASES_DIR --replay DIR");
    };
    match evaluate(&dir, &replay) {
        Err(e) => Outcome::fail_error(&e),
        Ok(cases) => Outcome {
            code: u8::from(cases.iter().any(|c| !c.failed.is_empty())),
            stdout: evalrun_json(&cases).into_bytes(),
            stderr: String::new(),
            error: None,
        },
    }
}
