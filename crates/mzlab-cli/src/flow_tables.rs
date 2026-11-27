//! `mzlab-cli flow tables RUNDIR [--json]`: the feature tables (`.mzfeat`) of a `flow run` folder with the facts of each, the `tables/1` entries that the
//! Flow column shows (`mzlab_feat::tables`, the same function as the site). The step of a table comes from the `prov.json` beside it when there is one.

use std::path::Path;

use mzlab_core::error::Error;
use mzlab_core::json::{parse, Value};
use mzlab_feat::store::Slice;
use mzlab_feat::tables::{document, entry, instance_of, Made};

use crate::Outcome;

const HELP: &str = "mzlab-cli flow tables: the feature tables of a run folder.

usage: mzlab-cli flow tables RUNDIR [--json]

  (no option)  one line per table: file, rows, files, polarity, step, status, ms
  --json       tables/1: per table name, bytes, rows, files, polarity (when its recipe says), identity, tp, recipe_hash, crate and, with a prov.json, instance, step, status, ms, warnings
  --help       this text

A .mzfeat that cannot be read is not listed as a table: it is under \"skipped\" with its error key and the exit code is 1.
exit codes: 0 done (also with no table), 1 a table could not be read, 2 usage, 4 RUNDIR unreadable";

fn unreadable(path: &Path, e: std::io::Error) -> Outcome {
    Outcome::fail_error(
        &Error::new("err.cli.file").with("path", format!("{}: {e}", path.display())),
    )
}

/// `(name, text)` pairs: a table's entry, or the error key of a file that is not a table.
pub type Named = Vec<(String, String)>;

/// What `flow tables` finds in `dir`: `(name, entry)` of every table and, for files that are not tables, `(name, error_key)`.
pub fn scan(dir: &Path) -> std::io::Result<(Named, Named)> {
    let prov: Option<Value> = std::fs::read(dir.join("prov.json"))
        .ok()
        .and_then(|b| parse(&b).ok());
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "mzfeat"))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let (mut tables, mut skipped) = (Vec::new(), Vec::new());
    for name in names {
        let bytes = std::fs::read(dir.join(&name))?;
        let made = prov
            .as_ref()
            .and_then(|p| instance_of(p, &name).and_then(|i| Made::from_prov(p, &i)));
        match entry(&name, &mut Slice(&bytes), bytes.len() as u64, made.as_ref()) {
            Ok(e) => tables.push((name, e)),
            Err(e) => skipped.push((name, e.key.to_string())),
        }
    }
    Ok((tables, skipped))
}

/// The `tables/1` document of `dir` and the files that could not be read (the app's route `flow/live/tables` answers it; no citation, as the other routes).
pub fn document_of(dir: &Path) -> std::io::Result<(String, Named)> {
    let (tables, skipped) = scan(dir)?;
    let texts: Vec<String> = tables.into_iter().map(|(_, e)| e).collect();
    Ok((document(&texts, &skipped), skipped))
}

fn line(name: &str, e: &Value) -> String {
    let n = |k: &str| e.get(k).and_then(|v| v.f64()).unwrap_or(0.0);
    let s = |k: &str| e.get(k).and_then(|v| v.str()).unwrap_or("-").to_string();
    let pol = match n("polarity") as i64 {
        1 => "+",
        -1 => "-",
        _ => "?",
    };
    format!(
        "{name}  {} rows  {} files  {pol}  {}  {}  {} ms\n",
        n("rows"),
        n("files"),
        s("step"),
        s("status"),
        n("ms")
    )
}

pub(crate) fn run(args: Vec<String>) -> Outcome {
    if args.iter().any(|a| a == "--help") {
        return Outcome::ok(format!("{HELP}\n").into_bytes());
    }
    let json = args.iter().any(|a| a == "--json");
    let rest: Vec<&String> = args.iter().filter(|a| *a != "--json").collect();
    if let Some(o) = rest.iter().find(|a| a.starts_with("--")) {
        return Outcome::fail(&format!("unknown option {o}"));
    }
    let [dir] = rest.as_slice() else {
        return Outcome::fail("usage: mzlab-cli flow tables RUNDIR [--json]");
    };
    let dir = Path::new(dir.as_str());
    let (tables, skipped) = match scan(dir) {
        Ok(x) => x,
        Err(e) => return unreadable(dir, e),
    };
    let text = if json {
        let texts: Vec<String> = tables.iter().map(|(_, e)| e.clone()).collect();
        let doc = document(&texts, &skipped);
        let body = doc.strip_suffix('}').unwrap_or(&doc);
        format!("{body},{}}}\n", crate::citation::member())
    } else {
        let mut t = String::new();
        for (name, e) in &tables {
            if let Ok(v) = parse(e.as_bytes()) {
                t.push_str(&line(name, &v));
            }
        }
        t
    };
    let mut out = Outcome::ok(text.into_bytes());
    if !skipped.is_empty() {
        out.code = crate::exit::DATA;
        for (name, key) in &skipped {
            out.stderr.push_str(&format!("{name}: {key}\n"));
        }
    }
    out
}

#[cfg(test)]
#[path = "flow_tables_tests.rs"]
mod tests;
