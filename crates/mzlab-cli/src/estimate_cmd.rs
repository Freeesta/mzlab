//! `mzlab-cli estimate FILE... [--axis da|ppm] [--polarity +|-] [--emit-set PLAN.json] [--json]` (AU-6): the parameters of detection measured on the files
//! (`features.estimate`, the function `flow run` and the page call) and put together by `mzlab_core::estimates` (`estimate/1`): per value the median over the
//! files with its figures, and the plan parameters it would set, each with the evidence. Nothing is written to a plan: `--emit-set` prints the `--set` lines.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::value_text;
use mzlab_core::json::{self, Value};
use mzlab_core::jsonb::{text, write, Obj};
use mzlab_ops::run::feat::run_estimate::estimate_of;

use crate::extract::{self, Parsed};
use crate::Outcome;

const HELP: &str = "mzlab-cli estimate: the detection parameters measured on the data of one or more files (noise floor, minimum height, m/z tolerance, RT tolerance, mass
defect), put together over the files with the figures behind each value, and the plan parameters they would set. An estimate is a measurement of THESE files, never a
setting: nothing is applied, the lines are printed for you to pass with `--set`.

usage: mzlab-cli estimate FILE... [--axis da|ppm] [--polarity +|-] [--sample ID] [--emit-set PLAN.json] [--json] [--out PATH]

  FILE               an mzML, or any file the app reads; each file is measured on its own, the values are the median over the files
  --axis da|ppm      the m/z axis to measure with; default the one the file decides (unit resolution: da) and said in `files`
  --polarity +|-      the polarity to measure (1 and -1 also work); default the one the file decides
  --sample ID        one sample of a file with several (.wiff); default the first
  --emit-set PLAN    print one `--set INSTANCE.NAME=VALUE` line per parameter of the plan's steps the estimates set, each with its evidence as a comment; the plan is not written
  --json             estimate/1: summary, files (their estimates/1), values (median, min, max, points, per file), params (set, current value, rule), skipped, choices, notes
  --out PATH         write there instead of the standard output

files that do not agree on polarity and axis: the group most files share gives the values, the others are listed in `left_out`; fewer than 3 files give a note, never a refusal.
exit 0 done, 1 a file gave no estimate (the others are answered), 2 usage, 3 the plan is not a plan, 4 a file or the plan is missing, 5 unknown sample, 6 output not written";

/// The members of the `--set` comment of one proposed parameter.
fn comment(p: &Value) -> String {
    let n = |k: &str| p.get(k).map(value_text).unwrap_or_default();
    let now = match p.get("current") {
        Some(Value::Null) | None => "the plan does not write it".to_string(),
        Some(v) => format!("the plan has {}", value_text(v)),
    };
    format!(
        "{}: median of {} file(s), min {}, max {}; {now}",
        p.get("from").and_then(Value::str).unwrap_or(""),
        n("files"),
        n("min"),
        n("max")
    )
}

/// `--set` lines of the proposals, one per line, each with its evidence.
fn set_lines(doc: &Value) -> String {
    doc.get("params")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|p| {
            format!(
                "--set {}   # {}\n",
                p.get("set").and_then(Value::str).unwrap_or(""),
                comment(p)
            )
        })
        .collect()
}

/// The values and the proposals as an aligned table.
fn table(doc: &Value) -> String {
    let n = |v: &Value, k: &str| v.get(k).filter(|x| !x.is_null()).map(value_text);
    let mut out = String::from("value        median        min .. max   files  unit\n");
    for v in doc.get("values").map_or(&[][..], Value::arr) {
        let name = v.get("name").and_then(Value::str).unwrap_or("");
        let med = n(v, "median").unwrap_or_else(|| "-".into());
        let range = match (n(v, "min"), n(v, "max")) {
            (Some(a), Some(b)) => format!("{a} .. {b}"),
            _ => "-".into(),
        };
        let files = n(v, "files").unwrap_or_default();
        let unit = v.get("unit").and_then(Value::str).unwrap_or("");
        out += &format!("{name:<12} {med:<13} {range:<12} {files:<6} {unit}\n");
    }
    let lines = set_lines(doc);
    if !lines.is_empty() {
        out += "\nwould set (pass with --set; --emit-set PLAN.json names your instances):\n";
        out += &lines;
    }
    out
}

/// The `estimates/1` of one file as an entry, with what the file decided on its own (`decided`).
fn entry(f: &Path, p: &Parsed, given: &[(String, Value)]) -> Result<(Value, String)> {
    extract::with_tmp("estimate", |tmp| {
        let sample = p
            .sample(f)
            .map_err(|m| Error::new("err.cli.usage").with("message", m))?;
        let mut s = extract::open_sample(f, sample, tmp)?;
        let (doc, decided) = estimate_of(&mut s.routes.idx, &s.name, given)?;
        let mut v = json::parse(doc.as_bytes())?;
        if let Value::Obj(members) = &mut v
            && !decided.is_empty()
        {
            let list = decided
                .into_iter()
                .map(|(k, val, proof)| {
                    Obj::new()
                        .put("name", text(&k))
                        .put("value", val)
                        .put("proof", proof)
                        .done()
                })
                .collect();
            members.push(("decided".into(), Value::Arr(list)));
        }
        Ok((v, s.note))
    })
}

/// `--axis` and `--polarity` as the values every file is measured with, or the usage error.
fn given_of(p: &Parsed) -> std::result::Result<Vec<(String, Value)>, &'static str> {
    let mut given = Vec::new();
    match p.get("--axis").map(str::to_lowercase).as_deref() {
        None => {}
        Some("da") => given.push(("axis".into(), Value::Str("Da".into()))),
        Some("ppm") => given.push(("axis".into(), Value::Str("ppm".into()))),
        Some(_) => return Err("--axis takes da or ppm"),
    }
    match p.get("--polarity") {
        None => {}
        Some("+" | "1") => given.push(("polarity".into(), Value::Num(1.0))),
        Some("-" | "-1") => given.push(("polarity".into(), Value::Num(-1.0))),
        Some(_) => return Err("--polarity takes + or -"),
    }
    Ok(given)
}

/// The plan `--emit-set` names (`null` without it), or the failure to report.
fn plan_of(p: &Parsed) -> std::result::Result<Value, Outcome> {
    let Some(path) = p.get("--emit-set") else {
        return Ok(Value::Null);
    };
    let bytes = std::fs::read(path).map_err(|e| {
        let err = Error::new("err.flow.io").with("detail", format!("{path}: {e}"));
        Outcome::fail_error(&err)
    })?;
    match json::parse(&bytes) {
        Ok(v) if v.get("steps").is_some_and(|s| matches!(s, Value::Arr(_))) => Ok(v),
        Ok(_) => Err(Outcome::fail("err.flow.shape path=steps expected=array")),
        Err(_) => Err(Outcome::fail("err.flow.json")),
    }
}

pub(crate) fn run(args: Vec<String>) -> Outcome {
    let as_json = args.iter().any(|a| a == "--json");
    let p = match extract::parse(
        args,
        &["--axis", "--polarity", "--sample", "--emit-set", "--out"],
        &[],
    ) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let files: Vec<PathBuf> = p.pos.iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Outcome::fail("estimate needs a file");
    }
    let given = match given_of(&p) {
        Ok(g) => g,
        Err(m) => return Outcome::fail(m),
    };
    if let Some(e) = crate::raw_cmd::missing(&files) {
        return crate::sample_select::fail(&e);
    }
    let plan = match plan_of(&p) {
        Ok(v) => v,
        Err(o) => return o,
    };
    let (mut entries, mut notes, mut bad) = (Vec::new(), String::new(), false);
    for f in &files {
        match entry(f, &p, &given) {
            Ok((v, note)) => {
                entries.push(v);
                notes += &note;
            }
            Err(e) if crate::raw_cmd::whole_run(&e) => return crate::sample_select::fail(&e),
            Err(e) => {
                bad = true;
                let t = crate::raw_cmd::error_entry(f, &e, "estimate");
                entries.push(json::parse(t.as_bytes()).unwrap_or(Value::Null));
            }
        }
    }
    answer(&p, entries, plan, notes, bad, as_json)
}

/// Puts the entries together (`mzlab_core::estimates::route`) and prints the form asked for; exit 1 when a file gave no estimate or nothing was measured.
fn answer(
    p: &Parsed,
    entries: Vec<Value>,
    plan: Value,
    mut notes: String,
    bad: bool,
    as_json: bool,
) -> Outcome {
    let body = write(
        &Obj::new()
            .put("files", Value::Arr(entries))
            .put("plan", plan)
            .done(),
    );
    let answer = match mzlab_core::estimates::route(&body) {
        Ok(a) => a,
        Err(e) => return Outcome::fail_error(&e),
    };
    let doc = match json::parse(answer.as_bytes()) {
        Ok(d) => d,
        Err(e) => return Outcome::fail_error(&e),
    };
    let no_estimate = doc
        .get("summary")
        .and_then(|s| s.get("no_estimate"))
        .and_then(Value::f64)
        .unwrap_or(0.0);
    let empty = doc
        .get("values")
        .map_or(&[][..], Value::arr)
        .iter()
        .all(|v| v.get("median").is_none_or(Value::is_null));
    for n in doc.get("notes").map_or(&[][..], Value::arr) {
        notes += &format!("{}\n", value_text(n));
    }
    let out = if as_json {
        // the engine's document and the citation member, nothing else
        let body = answer.trim_end().strip_suffix('}').unwrap_or(&answer);
        format!("{body},{}}}\n", crate::citation::member())
    } else if p.get("--emit-set").is_some() {
        set_lines(&doc)
    } else {
        table(&doc)
    };
    let mut o = crate::emit(out, &p.out());
    if o.code == 0 {
        o.stderr = notes;
        if bad || no_estimate > 0.0 || empty {
            o.code = crate::exit::DATA;
        }
    }
    o
}
