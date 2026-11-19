//! `mzlab-cli sample design propose|validate|canonical`: the experimental design (`design/1`) that `table.stats@1` and `table.drift@1` take as input,
//! from the command line. The bytes are the ones `samples::route_all` gives the site (`design_propose`, `design_validate`, `design_canonical`,
//! `design_digest`) and the app; this file only reads the arguments, lists the faults by key and writes the documents.

use std::path::Path;

use mzlab_core::error::Error;
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::samples::{self, Sample};

use crate::Outcome;

pub const HELP: &str = "mzlab-cli sample design: the experimental design of a series (design/1), proposed, checked and written canonically.

usage: mzlab-cli sample design propose (--samples SAMPLES.json | FILE...) [--out DESIGN.json] [--json]
       mzlab-cli sample design validate DESIGN.json [--samples SAMPLES.json] [--json]
       mzlab-cli sample design canonical DESIGN.json [--out PATH] [--json]

  propose    the design the names suggest, as design_proposal/1: the design, each automatic choice with its reason and the evidence (the files of each
             condition); the user decides, edit the file or keep it
  validate   exit 0: design_check/1 (the SHA-256 and the warnings, codes only, when --samples is given); exit 3: error/1 whose `problems` name each bad key
             (`conditions[1].replicates`, `reference`, ...) and, with --samples, each blocking fault (emptyComparison, refMissing)
  canonical  design_canonical/1: the canonical bytes (keys in alphabetical order, factors and batches sorted) as `design` and their `sha256`, the one `prov/1` records
  --samples FILE  a samples/1 document (`sample table`); with propose, FILE... are real files named and hashed as `sample table` does
  --out PATH      propose and canonical: also write the bare design/1 there (what a plan takes as its `design` input), ending with a newline
Streams: the document on stdout, errors on stderr. A plan: `sample table FILE... > samples.json`, `sample design propose --samples samples.json --out design.json`.
Exit codes: 0 done; 2 usage; 3 the design or the samples file is not valid; 4 a file missing or unreadable; 6 --out cannot be written.";

/// A fault of the design file: its key path, its code and the extra parameters of its text.
struct Fault {
    key: String,
    code: &'static str,
    params: Vec<(&'static str, String)>,
}

/// A usage error (exit 2) of this command: a key `err.cli.design.*` and its parameter.
fn usage(key: &'static str, op: &str) -> Outcome {
    Outcome::fail_error(&Error::new(key).with("op", op))
}

/// An engine error as the outcome; a design or samples file the engine refuses is a file that is not valid: exit 3, for this command only.
fn engine(e: &Error) -> Outcome {
    let mut o = Outcome::fail_error(e);
    if matches!(e.key, "error.badDesign" | "error.badSamples") {
        o.code = crate::exit::INTEGRITY;
    }
    o
}

/// The text of a `lang/en` key, its `{name}` filled in (no key suffix: the text goes into a document, not the terminal).
fn say(key: &'static str, name: &str) -> String {
    crate::error_text::catalog_text(key)
        .unwrap_or(key)
        .replace("{name}", name)
}

fn read_text(path: &str) -> Result<String, Outcome> {
    std::fs::read_to_string(path).map_err(|e| Outcome::fail(&format!("{path}: {e}")))
}

/// The `lang/en` key of the text of a fault code.
fn problem_key(code: &'static str) -> &'static str {
    match code {
        "missing" => "err.design.problem.missing",
        "wrongType" => "err.design.problem.wrongType",
        "badNumber" => "err.design.problem.badNumber",
        "wrongSchema" => "err.design.problem.wrongSchema",
        "badKind" => "err.design.problem.badKind",
        "duplicate" => "err.design.problem.duplicate",
        "unknownCondition" => "err.design.problem.unknownCondition",
        "emptyComparison" => "err.design.problem.emptyComparison",
        "refMissing" => "err.design.problem.refMissing",
        c if c.starts_with("err") => c,
        _ => "err.design.problem.notJson",
    }
}

/// The exit 3 of a design with faults: `error/1` whose `problems` list each one (`instance` is `design`, `port` the key path).
fn invalid(faults: &[Fault]) -> Outcome {
    let mut list = String::from("[");
    let mut lines = String::new();
    for (i, f) in faults.iter().enumerate() {
        let key = problem_key(f.code);
        let mut e = Error::new(key).with("key", &f.key);
        for (k, v) in &f.params {
            e = e.with(k, v);
        }
        lines += &format!("  {}\n", crate::error_text::human_error(&e));
        let mut o = Out::default();
        o.raw(if i > 0 {
            ",{\"instance\":"
        } else {
            "{\"instance\":"
        })
        .str("design")
        .raw(",\"port\":")
        .str(&f.key)
        .raw(",\"error_key\":")
        .str(key)
        .raw(",\"params\":{\"key\":")
        .str(&f.key);
        for (k, v) in &f.params {
            o.raw(",").str(k).raw(":").str(v);
        }
        o.raw("}}");
        list += &o.0;
    }
    list.push(']');
    let first = faults.first().map_or(String::new(), |f| f.key.clone());
    let e = Error::new("err.design.invalid")
        .with("count", faults.len())
        .with("key", &first);
    let hint = say("err.design.hint", "");
    let mut o = Outcome::fail_error(&e).with_hint(&hint);
    o.stderr = format!(
        "error: {}\n{lines}hint: {hint}\n",
        crate::error_text::human_error(&e)
    );
    if let Some(f) = o.error.as_mut() {
        f.problems = Some(list);
    }
    o
}

/// The design file, parsed and checked key by key; `Err` is the exit 3 outcome.
fn load_design(path: &str) -> Result<String, Outcome> {
    let text = read_text(path)?;
    let doc = parse(text.as_bytes()).map_err(|_| {
        invalid(&[Fault {
            key: "$".into(),
            code: "notJson",
            params: Vec::new(),
        }])
    })?;
    let faults: Vec<Fault> = mzlab_core::design::problems(&doc)
        .into_iter()
        .map(|p| Fault {
            key: p.key,
            code: p.code,
            params: Vec::new(),
        })
        .collect();
    if faults.is_empty() {
        Ok(text)
    } else {
        Err(invalid(&faults))
    }
}

fn route(op: &str, input: &str) -> Result<String, Outcome> {
    samples::route_all(&format!("design_{op}"), input).map_err(|e| engine(&e))
}

fn jstr(s: &str) -> String {
    let mut o = Out::default();
    o.str(s);
    o.0
}

fn write_out(path: &str, text: &str) -> Result<(), Outcome> {
    std::fs::write(Path::new(path), format!("{text}\n")).map_err(|e| {
        Outcome::fail_error(&Error::new("err.cli.out").with("path", format!("{path}: {e}")))
    })
}

struct Args {
    pos: Vec<String>,
    samples: Option<String>,
    out: Option<String>,
}

fn args_of(args: Vec<String>) -> Result<Option<Args>, String> {
    let p = crate::extract::parse(args, &["--samples", "--out"], &[])?;
    Ok(p.map(|p| Args {
        samples: p.get("--samples").map(String::from),
        out: p.get("--out").map(String::from),
        pos: p.pos,
    }))
}

/// A samples file that is read and is a `samples/1` document; one that is not is exit 3 (a missing one is exit 4, from `read_text`).
fn read_samples(path: &str) -> Result<String, Outcome> {
    let text = read_text(path)?;
    if parse(text.as_bytes()).is_err() {
        return Err(Outcome::fail_error(
            &Error::new("err.design.samplesNotJson").with("path", path),
        ));
    }
    samples::from_json(text.as_bytes()).map_err(|e| engine(&e))?;
    Ok(text)
}

/// The `samples/1` text `propose` starts from: a file, or the table of the names and files given (as `sample table`).
fn samples_text(a: &Args) -> Result<String, Outcome> {
    match (&a.samples, a.pos.is_empty()) {
        (Some(_), false) => Err(usage("err.cli.design.samplesAndFiles", "propose")),
        (None, true) => Err(usage("err.cli.design.needsInput", "propose")),
        (Some(p), true) => read_samples(p),
        (None, false) => {
            let body = crate::samples_cmd::table(a.pos.clone()).map_err(|e| Outcome::fail(&e))?;
            samples::route_all("propose", &body).map_err(|e| engine(&e))
        }
    }
}

fn list(v: &[String]) -> String {
    format!(
        "[{}]",
        v.iter().map(|s| jstr(s)).collect::<Vec<_>>().join(",")
    )
}

/// The automatic choices of `propose`, each with its value, a stable `why` code and the sentence of that code (`err.design.why.<code>`).
fn choices_of(d: &mzlab_core::design::Design) -> Vec<String> {
    let names: Vec<String> = d.conditions.iter().map(|c| c.name.clone()).collect();
    // each choice: a stable `why` code and its text (`err.design.why.<code>` in `lang/en`)
    let choice = |name: &str, value: String, why: &str, text: String| {
        format!(
            "{{\"choice\":\"{name}\",\"value\":{value},\"why\":{},\"reason\":{}}}",
            jstr(why),
            jstr(&text)
        )
    };
    let mut choices = vec![choice(
        "conditions",
        list(&names),
        "series",
        say("err.design.why.series", ""),
    )];
    let reference = d.reference.as_deref();
    let (why, text) = match reference {
        Some(r) => {
            let code = mzlab_core::design::reference_rule(r).unwrap_or("prefixRef");
            let key = match code {
                "containsReference" => "err.design.why.containsReference",
                "containsControl" => "err.design.why.containsControl",
                "containsNoGas" => "err.design.why.containsNoGas",
                _ => "err.design.why.prefixRef",
            };
            (code, say(key, r))
        }
        None => ("noReference", say("err.design.why.noReference", "")),
    };
    choices.push(choice(
        "reference",
        reference.map_or("null".into(), jstr),
        why,
        text,
    ));
    let (value, why, text) = if d.comparisons.is_empty() {
        (
            "none",
            "nothingToCompare",
            say("err.design.why.nothingToCompare", ""),
        )
    } else {
        (
            "vs_reference",
            "allAgainstReference",
            say("err.design.why.allAgainstReference", ""),
        )
    };
    choices.push(choice("comparisons", jstr(value), why, text));
    choices.push(choice(
        "batches",
        jstr(mzlab_core::design::DEFAULT_BATCH),
        "defaultBatch",
        say("err.design.why.defaultBatch", ""),
    ));
    choices
}

/// `propose`'s answer: the design, its automatic choices with the reason of each, and the files of every condition.
fn proposal(cs: &[Sample], design: &str) -> Result<String, Outcome> {
    let d = mzlab_core::design::from_json(design.as_bytes()).map_err(|e| engine(&e))?;
    let choices = choices_of(&d);
    let mut ordered: Vec<&Sample> = cs.iter().filter(|c| c.role() == "sample").collect();
    ordered.sort_by(|a, b| (a.order, &a.file).cmp(&(b.order, &b.file)));
    let conditions: Vec<String> = d
        .conditions
        .iter()
        .map(|c| {
            let files: Vec<String> = ordered
                .iter()
                .filter(|s| s.series == c.name)
                .map(|s| s.file.clone())
                .collect();
            format!(
                "{{\"name\":{},\"replicates\":{},\"files\":{}}}",
                jstr(&c.name),
                c.replicates,
                list(&files)
            )
        })
        .collect();
    Ok(format!(
        "{{\"schema\":\"design_proposal/1\",\"design\":{design},\"choices\":[{}],\"evidence\":{{\"min_replicates\":{},\"sample_files\":{},\"conditions\":[{}]}}}}\n",
        choices.join(","),
        mzlab_core::design::MIN_REPLICATES,
        ordered.len(),
        conditions.join(",")
    ))
}

fn propose(args: Vec<String>) -> Result<String, Outcome> {
    let Some(a) = args_of(args).map_err(|e| Outcome::fail(&e))? else {
        return Err(Outcome::ok(format!("{HELP}\n").into_bytes()));
    };
    let text = samples_text(&a)?;
    let cs = samples::from_json(text.as_bytes()).map_err(|e| engine(&e))?;
    let design = route("propose", &text)?;
    // the whole answer first: a failure leaves no stale design file behind
    let answer = proposal(&cs, &design)?;
    if let Some(p) = &a.out {
        write_out(p, &design)?;
    }
    Ok(answer)
}

fn validate(args: Vec<String>) -> Result<String, Outcome> {
    let Some(a) = args_of(args).map_err(|e| Outcome::fail(&e))? else {
        return Err(Outcome::ok(format!("{HELP}\n").into_bytes()));
    };
    let [file] = a.pos.as_slice() else {
        return Err(usage("err.cli.design.oneFile", "validate"));
    };
    let text = load_design(file)?;
    let sha = route("digest", &text)?;
    let head = format!(
        "{{\"schema\":\"design_check/1\",\"valid\":true,\"sha256\":{},",
        jstr(&sha)
    );
    let Some(sp) = &a.samples else {
        return Ok(format!(
            "{head}\"samples_checked\":false,\"warnings\":[],\"blocking\":false}}\n"
        ));
    };
    let samples_doc = read_samples(sp)?;
    let verdict = route(
        "validate",
        &format!("{{\"design\":{text},\"samples\":{samples_doc}}}"),
    )?;
    let v = parse(verdict.as_bytes()).map_err(|e| engine(&e))?;
    let faults: Vec<Fault> = v
        .get("warnings")
        .map_or(&[][..], Value::arr)
        .iter()
        .filter(|w| w.get("severity").and_then(Value::str) == Some("error"))
        .map(|w| {
            let code = match w.get("code").and_then(Value::str) {
                Some("refMissing") => "refMissing",
                _ => "emptyComparison",
            };
            Fault {
                key: if code == "refMissing" {
                    "reference"
                } else {
                    "comparisons"
                }
                .into(),
                code,
                params: vec![(
                    "condition",
                    w.get("series")
                        .and_then(Value::str)
                        .unwrap_or("")
                        .to_string(),
                )],
            }
        })
        .collect();
    if !faults.is_empty() {
        return Err(invalid(&faults));
    }
    // the rest of the route's own document: its warnings and `blocking`, byte for byte
    let rest = verdict.strip_prefix('{').unwrap_or(&verdict);
    Ok(format!("{head}\"samples_checked\":true,{rest}\n"))
}

fn canonical(args: Vec<String>) -> Result<String, Outcome> {
    let Some(a) = args_of(args).map_err(|e| Outcome::fail(&e))? else {
        return Err(Outcome::ok(format!("{HELP}\n").into_bytes()));
    };
    let [file] = a.pos.as_slice() else {
        return Err(usage("err.cli.design.oneFile", "canonical"));
    };
    let text = load_design(file)?;
    let canon = route("canonical", &text)?;
    let sha = route("digest", &text)?;
    if let Some(p) = &a.out {
        write_out(p, &canon)?;
    }
    Ok(format!(
        "{{\"schema\":\"design_canonical/1\",\"sha256\":{},\"bytes\":{},\"design\":{canon}}}\n",
        jstr(&sha),
        canon.len()
    ))
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let sub = it.next();
    let rest: Vec<String> = it.collect();
    let done = match sub.as_deref() {
        Some("propose") => propose(rest),
        Some("validate") => validate(rest),
        Some("canonical") => canonical(rest),
        Some("--help" | "-h") | None => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Some(c) => return usage("err.cli.design.unknownOp", c),
    };
    match done {
        Ok(t) => Outcome::ok(t.into_bytes()),
        Err(o) => o,
    }
}

#[cfg(test)]
#[path = "design_cmd_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "design_cmd_faults_tests.rs"]
mod faults_tests;
