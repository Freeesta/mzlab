//! `mzlab-cli llm inspect FILE|DIR...` and `mzlab-cli llm params FILE|DIR...|--inspection FILE|-`: the `inspection/1` and the `param-proposal/1` of mzML
//! files, without any model. The files are indexed in parallel (`par.rs`) and joined in order, so the bytes do not depend on the workers; `params` can
//! reuse an inspection instead of indexing again. The sources are the answers of the engine routes the page asks for the same files (items of `Item.info()`, `/api/scan_stats`, `/api/mrm`,
//! `/api/metodo`, `samples/1 propose`), assembled in the same order as `flow/lazy/flow-llm-facts.js`, so the page and the CLI give the same bytes.

use std::path::{Path, PathBuf};

use mzlab_core::idxread::Index;
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::lr::{open_items, Sample};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::samples;
use mzlab_core::upload::prepare;
use mzlab_llm::inspect::build;
use mzlab_llm::params_rules::propose;

use crate::flow::steps;
use crate::Outcome;

/// The largest method document read from a header (the page's `DOC_MAX`).
const DOC_MAX: u64 = 1 << 20;
/// The default budget of the inspection, in tokens (the page's).
pub(crate) const BUDGET: u64 = 30000;

fn quoted(s: &str) -> String {
    let mut o = Out::default();
    o.str(s);
    o.0
}

/// The mzML files of `dir`, sorted by name (the order the page opens them in the tests).
fn files_of(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("mzml"))
        })
        .collect();
    v.sort();
    Ok(v)
}

/// The files the arguments name: a folder gives its mzML files sorted by name, a file stays where it was given (the order of the series is the user's).
pub(crate) fn inputs(args: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut v = Vec::new();
    for a in args {
        let p = Path::new(a);
        if p.is_dir() {
            v.extend(files_of(p)?);
        } else {
            std::fs::metadata(p).map_err(|e| format!("{a}: {e}"))?;
            v.push(p.to_path_buf());
        }
    }
    Ok(v)
}

/// The source of one file: `{name, items, scan_stats, mrm?, method?, sample}`, or `{name, error}` when the engine cannot open it.
fn file_source(path: &Path, sample: &Value, order: usize) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let open = || -> mzlab_core::error::Result<String> {
        let bytes = std::fs::read(path)
            .map_err(|_| mzlab_core::error::Error::new("err.file.unreadable"))?;
        let mut src = bytes.clone();
        let p = prepare(&mut src, bytes.len() as u64, &name)?;
        let (idx, _) = build_mzml_to_vec(&p.table, &bytes, 256 << 20)?;
        let len = idx.len() as u64;
        let mut idx = Index::open(idx, len)?;
        let mut m = Mzml::new(bytes.clone(), bytes.len() as u64);
        let items: Vec<String> = open_items(&mut idx, &mut m, &name, &Sample::default())?
            .into_iter()
            .map(|i| i.info)
            .collect();
        let srm = items.iter().any(|i| {
            parse(i.as_bytes())
                .ok()
                .and_then(|v| v.get("srm").and_then(Value::f64))
                .is_some_and(|n| n > 0.0)
        });
        let mut o = format!(
            "{{\"name\":{},\"items\":[{}],\"scan_stats\":{}",
            quoted(&name),
            items.join(","),
            mzlab_core::scanstats::scan_stats_json(&idx.scans)
        );
        if srm {
            let end = idx.scans.last().map_or(0, |s| s.end);
            let chroms = m.chromatograms(end)?;
            o += &format!(
                ",\"mrm\":{}",
                mzlab_core::fileroutes::mrm_transitions(&mut m, &chroms)?
            );
        }
        if let Some(doc) = m.method(&name, DOC_MAX)? {
            o += &format!(",\"method\":{doc}");
        }
        Ok(o)
    };
    let kind = sample.get("kind").and_then(Value::str).unwrap_or("sample");
    let tail = format!(
        ",\"sample\":{{\"tipo\":{},\"ordine_iniezione\":{order}}}}}",
        quoted(kind)
    );
    match open() {
        Ok(o) => o + &tail,
        Err(e) => format!(
            "{{\"name\":{},\"error\":{{\"error_key\":{}}}}}",
            quoted(&name),
            quoted(e.key)
        ),
    }
}

/// The sources of `inspection/1` for `files`, indexed on `workers` threads.
pub(crate) fn sources_with(
    files: &[PathBuf],
    hide: bool,
    workers: usize,
) -> Result<String, String> {
    let names: Vec<String> = files
        .iter()
        .map(|p| {
            format!(
                "{{\"name\":{}}}",
                quoted(p.file_name().and_then(|n| n.to_str()).unwrap_or(""))
            )
        })
        .collect();
    let proposed = samples::route("propose", &format!("{{\"files\":[{}]}}", names.join(",")))
        .map_err(|e| e.to_string())?;
    let proposed = parse(proposed.as_bytes()).map_err(|e| e.to_string())?;
    let samples = proposed.get("samples").map_or(&[][..], Value::arr);
    let parts = crate::par::par_map_with(workers, files.len(), |k| {
        file_source(&files[k], samples.get(k).unwrap_or(&Value::Null), k + 1)
    });
    Ok(format!(
        "{{\"profile\":\"research\",\"hide_names\":{hide},\"files\":[{}]}}",
        parts.join(",")
    ))
}

/// The `inspection/1` of `files`, indexed on `workers` threads.
pub(crate) fn inspection_with(
    files: &[PathBuf],
    hide: bool,
    budget: u64,
    workers: usize,
) -> Result<String, String> {
    let src = parse(sources_with(files, hide, workers)?.as_bytes()).map_err(|e| e.to_string())?;
    build(&src, budget).map_err(|e| e.to_string())
}

/// The `inspection/1` of `files`, one worker per core.
pub(crate) fn inspection(files: &[PathBuf], hide: bool, budget: u64) -> Result<String, String> {
    let workers = std::thread::available_parallelism().map_or(1, |w| w.get());
    inspection_with(files, hide, budget, workers)
}

struct Opts {
    inputs: Vec<String>,
    hide: Option<bool>,
    budget: Option<u64>,
    answers: Option<String>,
    inspection: Option<String>,
}

fn opts(args: Vec<String>, usage: &str) -> Result<Opts, Outcome> {
    let mut o = Opts {
        inputs: Vec::new(),
        hide: None,
        budget: None,
        answers: None,
        inspection: None,
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--hide-names" => o.hide = Some(true),
            "--budget" => match it.next().and_then(|b| b.parse().ok()) {
                Some(b) => o.budget = Some(b),
                None => return Err(Outcome::fail("--budget needs a number")),
            },
            "--answers" => match it.next() {
                Some(p) => o.answers = Some(p),
                None => return Err(Outcome::fail("--answers needs a file")),
            },
            "--inspection" => match it.next() {
                Some(p) => o.inspection = Some(p),
                None => return Err(Outcome::fail("--inspection needs a file or -")),
            },
            x if !x.starts_with('-') => o.inputs.push(x.to_string()),
            x => return Err(Outcome::fail(&format!("unknown option {x}"))),
        }
    }
    if o.inputs.is_empty() == o.inspection.is_none() {
        return Err(Outcome::fail(usage));
    }
    Ok(o)
}

/// `llm inspect FILE|DIR... [--budget N] [--hide-names]`.
pub(crate) fn inspect(args: Vec<String>) -> Outcome {
    const USAGE: &str = "usage: mzlab-cli llm inspect FILE|DIR... [--budget N] [--hide-names]";
    let o = match opts(args, USAGE) {
        Ok(o) => o,
        Err(e) => return e,
    };
    if o.inspection.is_some() || o.answers.is_some() {
        return Outcome::fail(USAGE);
    }
    let go = || -> Result<String, String> {
        let files = inputs(&o.inputs)?;
        inspection(&files, o.hide.unwrap_or(false), o.budget.unwrap_or(BUDGET))
    };
    go().map_or_else(
        |e| Outcome::fail(&e),
        |t| Outcome::ok((t + "\n").into_bytes()),
    )
}

/// The inspection a file or standard input (`-`) holds.
fn read_inspection(from: &str) -> Result<String, String> {
    let bytes = if from == "-" {
        let mut b = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut b).map_err(|e| format!("-: {e}"))?;
        b
    } else {
        std::fs::read(from).map_err(|e| format!("{from}: {e}"))?
    };
    let text = String::from_utf8(bytes).map_err(|_| format!("{from}: not UTF-8 text"))?;
    let ok = parse(text.as_bytes())
        .ok()
        .and_then(|v| {
            v.get("schema")
                .and_then(Value::str)
                .map(|s| s == "inspection/1")
        })
        .unwrap_or(false);
    if ok {
        Ok(text)
    } else {
        Err(format!(
            "{from}: not an inspection/1 document (the output of llm inspect)"
        ))
    }
}

/// `llm params FILE|DIR... [--answers FILE] [--budget N] [--hide-names]` or `llm params --inspection FILE|- [--answers FILE]`: the rules on the
/// inspection of the same files, or on one already made (no second indexing; its budget and hidden names are the ones it was made with).
pub(crate) fn params(args: Vec<String>) -> Outcome {
    const USAGE: &str = "usage: mzlab-cli llm params FILE|DIR... [--answers FILE] [--budget N] [--hide-names] | --inspection FILE|- [--answers FILE]";
    let o = match opts(args, USAGE) {
        Ok(o) => o,
        Err(e) => return e,
    };
    if o.inspection.is_some() && (o.budget.is_some() || o.hide.is_some()) {
        return Outcome::fail(
            "--budget and --hide-names belong to the inspection: give them to llm inspect",
        );
    }
    let go = || -> Result<String, String> {
        let insp = match &o.inspection {
            Some(from) => read_inspection(from)?,
            None => inspection(
                &inputs(&o.inputs)?,
                o.hide.unwrap_or(false),
                o.budget.unwrap_or(BUDGET),
            )?,
        };
        let answers = match &o.answers {
            Some(p) => parse(&std::fs::read(p).map_err(|e| format!("{p}: {e}"))?)
                .map_err(|e| e.to_string())?,
            None => Value::Null,
        };
        let insp = parse(insp.as_bytes()).map_err(|e| e.to_string())?;
        propose(&insp, &answers, &steps()).map_err(|e| e.to_string())
    };
    go().map_or_else(
        |e| Outcome::fail(&e),
        |t| Outcome::ok((t + "\n").into_bytes()),
    )
}
