//! `mzlab-cli llm inspect DIR` and `mzlab-cli llm params DIR`: the `ispezione/1` and the `proposta-param/1` of the mzML files of a folder, without any
//! model. The sources are the answers of the engine routes the page asks for the same files (items of `Item.info()`, `/api/scan_stats`, `/api/mrm`,
//! `/api/metodo`, `campioni/2 proponi`), assembled in the same order as `flow/lazy/flow-llm-facts.js`, so the page and the CLI give the same bytes.

use std::path::{Path, PathBuf};

use mzlab_core::campioni;
use mzlab_core::idxread::Index;
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::lr::{open_items, Sample};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzmlhead::Mzml;
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
        if let Some(doc) = m.metodo(&name, DOC_MAX)? {
            o += &format!(",\"method\":{doc}");
        }
        Ok(o)
    };
    let tipo = sample.get("tipo").and_then(Value::str).unwrap_or("sample");
    let tail = format!(
        ",\"sample\":{{\"tipo\":{},\"ordine_iniezione\":{order}}}}}",
        quoted(tipo)
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

/// The sources of `ispezione/1` for the mzML files of `dir`.
pub(crate) fn sources(dir: &Path, hide: bool) -> Result<String, String> {
    let files = files_of(dir)?;
    let names: Vec<String> = files
        .iter()
        .map(|p| {
            format!(
                "{{\"name\":{}}}",
                quoted(p.file_name().and_then(|n| n.to_str()).unwrap_or(""))
            )
        })
        .collect();
    let proposed = campioni::route("proponi", &format!("{{\"files\":[{}]}}", names.join(",")))
        .map_err(|e| e.to_string())?;
    let proposed = parse(proposed.as_bytes()).map_err(|e| e.to_string())?;
    let samples = proposed.get("campioni").map_or(&[][..], Value::arr);
    let parts: Vec<String> = files
        .iter()
        .enumerate()
        .map(|(k, p)| file_source(p, samples.get(k).unwrap_or(&Value::Null), k + 1))
        .collect();
    Ok(format!(
        "{{\"profile\":\"ricerca\",\"hide_names\":{hide},\"files\":[{}]}}",
        parts.join(",")
    ))
}

/// The `ispezione/1` of the mzML files of `dir`.
pub(crate) fn inspection(dir: &Path, hide: bool, budget: u64) -> Result<String, String> {
    let src = parse(sources(dir, hide)?.as_bytes()).map_err(|e| e.to_string())?;
    build(&src, budget).map_err(|e| e.to_string())
}

struct Opts {
    dir: Option<String>,
    hide: bool,
    budget: u64,
    answers: Option<String>,
}

fn opts(args: Vec<String>, usage: &str) -> Result<Opts, Outcome> {
    let mut o = Opts {
        dir: None,
        hide: false,
        budget: BUDGET,
        answers: None,
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--hide-names" => o.hide = true,
            "--budget" => match it.next().and_then(|b| b.parse().ok()) {
                Some(b) => o.budget = b,
                None => return Err(Outcome::fail("--budget needs a number")),
            },
            "--answers" => match it.next() {
                Some(p) => o.answers = Some(p),
                None => return Err(Outcome::fail("--answers needs a file")),
            },
            x if !x.starts_with('-') && o.dir.is_none() => o.dir = Some(x.to_string()),
            x => return Err(Outcome::fail(&format!("unknown option {x}"))),
        }
    }
    if o.dir.is_none() {
        return Err(Outcome::fail(usage));
    }
    Ok(o)
}

/// `llm inspect DIR [--budget N] [--hide-names]`.
pub(crate) fn inspect(args: Vec<String>) -> Outcome {
    let o = match opts(
        args,
        "usage: mzlab-cli llm inspect DIR [--budget N] [--hide-names]",
    ) {
        Ok(o) => o,
        Err(e) => return e,
    };
    match inspection(Path::new(o.dir.as_deref().unwrap_or("")), o.hide, o.budget) {
        Ok(t) => Outcome::ok((t + "\n").into_bytes()),
        Err(e) => Outcome::fail(&e),
    }
}

/// `llm params DIR [--answers FILE] [--budget N] [--hide-names]`: the rules on the inspection of the same files.
pub(crate) fn params(args: Vec<String>) -> Outcome {
    let o = match opts(
        args,
        "usage: mzlab-cli llm params DIR [--answers FILE] [--budget N] [--hide-names]",
    ) {
        Ok(o) => o,
        Err(e) => return e,
    };
    let go = || -> Result<String, String> {
        let insp = inspection(Path::new(o.dir.as_deref().unwrap_or("")), o.hide, o.budget)?;
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
