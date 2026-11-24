//! `mzlab-cli sample import`: a CSV/TSV of file name -> sample name, role and group laid over the samples of a set of files (`samples::import`, the same
//! bytes as the site's `samples` op `import`). The findings go to stderr one line each, the document to stdout; `--dry-run` writes nothing.

use std::path::{Path, PathBuf};

use mzlab_core::error::Error;
use mzlab_core::json::{parse, Value};

use crate::Outcome;

pub const HELP: &str = "mzlab-cli sample import: a table of file name -> sample name, role and group laid over the samples of a set of files (sample-import/1).

usage: mzlab-cli sample import META.csv (FILE|DIR)... [--samples SAMPLES.json] [--out SAMPLES.json] [--dry-run] [--json]

  META.csv       CSV, TSV or semicolons (detected); columns `file` (the key; also `name`, `file_name`), `sample` (`sample_name`: the sample name), `role` (dark, blank, std, wash, QC, ...)
                 and `group`; any other column is ignored and logged
  FILE|DIR       the real files, named by their file name and hashed (SHA-256); a DIR gives the mzML, mzXML, .wiff, .raw, .d and similar files in it, sorted. The roles and times
                 the names propose (`sample table`) are the start and the rows overwrite them
  --samples S    start from this samples/1 instead of FILE|DIR (its files are the ones the rows are matched to)
  --out PATH     also write the samples/1 there, whole or not at all (temporary file, then rename); not with --dry-run, and never a path that is also META.csv, --samples or a FILE
  --dry-run      report only: the sample-import/1 document on stdout, nothing written
  --json         the sample-import/1 document on stdout ({report, samples, schema}); without it the samples/1 alone, so `> samples.json` works as with `sample table`
A row is a record of the CSV (the header is row 1; empty lines are not counted). It is matched to a file by exact name, then by case, then without the extension; two files for one key is `ambiguous` and nothing is guessed. `wash` is a blank of kind wash.
Stderr: one line per finding (`unmatched row`, `unmatched file`, `duplicate`, `unknown role`) with its evidence and one `auto:` line per automatic choice (a match that was not exact, the first of
duplicate rows, a role or name changed, with the value before and after). Nothing is refused for a finding: the user edits the CSV or the samples/1 and runs it again.
Exit codes: 0 every row matched and every file has one; 1 findings (the documents are still written); 2 usage (also --out naming an input); 3 the CSV or the samples file is not valid; 4 a file missing or unreadable; 6 --out cannot be written.";

/// The kinds of data file a DIR contributes (a `.d` or `.raw` may be a folder).
const DATA: [&str; 8] = ["mzml", "mzxml", "wiff", "raw", "d", "cdf", "mgf", "mzdata"];

fn text(path: &str) -> Result<String, Outcome> {
    std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::InvalidData {
            Outcome::fail_error(
                &Error::new("error.badCsv")
                    .with("row", 1)
                    .with("column", "file"),
            )
        } else {
            Outcome::fail_error(&Error::new("err.session.fileNotFound").with("name", path))
        }
    })
}

/// `(name, SHA-256)` of the files named: a path is one file, a folder holds its data files (a `.d` or `.raw` folder is itself one, not hashed).
fn files(paths: &[String]) -> Result<Vec<(String, String)>, Outcome> {
    let mut list: Vec<PathBuf> = Vec::new();
    for p in paths {
        let path = PathBuf::from(p);
        let is_data = |q: &Path| {
            q.extension()
                .is_some_and(|e| DATA.contains(&e.to_string_lossy().to_lowercase().as_str()))
        };
        if path.is_dir() && !is_data(&path) {
            let mut found: Vec<PathBuf> = std::fs::read_dir(&path)
                .map_err(|_| {
                    Outcome::fail_error(&Error::new("err.session.fileNotFound").with("name", p))
                })?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|q| is_data(q))
                .collect();
            found.sort();
            list.extend(found);
        } else if path.exists() {
            list.push(path);
        } else {
            return Err(Outcome::fail_error(
                &Error::new("err.session.fileNotFound").with("name", p),
            ));
        }
    }
    list.iter()
        .map(|q| {
            let name = q
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned());
            let sha = if q.is_file() {
                crate::flow_hash::file_sha256(q)
                    .map(|(_, s)| s)
                    .map_err(|e| Outcome::fail(&format!("{}: {e}", q.display())))?
            } else {
                String::new()
            };
            Ok((name, sha))
        })
        .collect()
}

fn str_of<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::str).unwrap_or("")
}

fn num_of(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::f64).unwrap_or(0.0) as i64
}

/// The text of a `lang/en` key with each `{name}` filled in, in one pass (a value that holds `{x}` stays as it is).
fn say(key: &str, vals: &[(&str, String)]) -> String {
    let t = crate::error_text::catalog_text(key).unwrap_or(key);
    let mut out = String::new();
    let mut rest = t;
    while let Some(i) = rest.find('{') {
        out += &rest[..i];
        let tail = &rest[i + 1..];
        match tail.find('}').and_then(|j| {
            vals.iter()
                .find(|(k, _)| *k == &tail[..j])
                .map(|(_, v)| (j, v))
        }) {
            Some((j, v)) => {
                out += v;
                rest = &tail[j + 1..];
            }
            None => {
                out.push('{');
                rest = tail;
            }
        }
    }
    out + rest
}

/// The strings of a JSON array of the report.
fn texts(v: &Value, k: &str) -> Vec<String> {
    v.get(k)
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|x| {
            x.str()
                .map_or_else(|| x.f64().unwrap_or(0.0).to_string(), String::from)
        })
        .collect()
}

/// The stderr lines of a report: the findings with their evidence, then the automatic choices (the texts are `err.cli.import.*` of `lang/en`).
fn lines(report: &Value) -> String {
    let each = |k: &str| report.get(k).map_or(&[][..], Value::arr);
    let mut out = String::new();
    for r in each("unmatched_rows") {
        let near = texts(r, "candidates").join(", ");
        let key = if near.is_empty() {
            "err.cli.import.unmatchedRow"
        } else {
            "err.cli.import.unmatchedRowNear"
        };
        let (n, f) = row_file(r);
        out += &say(
            key,
            &[
                ("row", n),
                ("file", f),
                ("why", str_of(r, "why").into()),
                ("near", near),
            ],
        );
        out += "\n";
    }
    for f in each("unmatched_files").iter().filter_map(Value::str) {
        out += &say("err.cli.import.unmatchedFile", &[("file", f.into())]);
        out += "\n";
    }
    for d in each("duplicates") {
        let key = if d.get("conflict") == Some(&Value::Bool(false)) {
            "err.cli.import.duplicateSame"
        } else {
            "err.cli.import.duplicateDiffer"
        };
        let vals = [
            ("file", str_of(d, "file").into()),
            ("rows", texts(d, "rows").join(", ")),
            ("kept", num_of(d, "kept").to_string()),
        ];
        out += &say(key, &vals);
        out += "\n";
    }
    for u in each("unknown_roles") {
        let (n, f) = row_file(u);
        out += &say(
            "err.cli.import.unknownRole",
            &[
                ("row", n),
                ("file", f),
                ("value", str_of(u, "value").into()),
            ],
        );
        out += "\n";
    }
    for c in each("shared_names") {
        let files = texts(c, "files");
        out += &say(
            "err.cli.import.sharedName",
            &[
                ("name", str_of(c, "name").into()),
                ("count", files.len().to_string()),
                ("files", files.join(", ")),
            ],
        );
        out += "\n";
    }
    for c in each("choices") {
        let (n, f) = row_file(c);
        let vals = [
            ("what", str_of(c, "what").into()),
            ("row", n),
            ("file", f),
            ("from", str_of(c, "from").into()),
            ("to", str_of(c, "to").into()),
        ];
        out += &say("err.cli.import.auto", &vals);
        out += "\n";
    }
    out
}

fn row_file(v: &Value) -> (String, String) {
    (num_of(v, "row").to_string(), str_of(v, "file").to_string())
}

/// A CSV or samples file the engine refuses is exit 3 here (a missing one is exit 4).
fn engine(e: &Error) -> Outcome {
    let mut o = Outcome::fail_error(e);
    if matches!(
        e.key,
        "error.badCsv" | "error.badSamples" | "error.badRequest"
    ) {
        o.code = crate::exit::INTEGRITY;
    }
    o
}

/// The request of the engine: the CSV text plus the `samples/1` document or the files (name and hash) it goes over.
fn request(csv: &str, samples: Option<&str>, paths: &[String]) -> Result<String, Outcome> {
    let mut body = mzlab_core::json::Out::default();
    body.raw("{\"text\":").str(csv);
    match samples {
        Some(s) => {
            let t = text(s)?;
            if parse(t.as_bytes()).is_err() {
                return Err(engine(&Error::new("error.badSamples")));
            }
            body.raw(",\"samples\":").raw(&t);
        }
        None => {
            body.raw(",\"files\":[");
            for (i, (n, sha)) in files(paths)?.iter().enumerate() {
                body.raw(if i > 0 { "," } else { "" })
                    .raw("{\"name\":")
                    .str(n);
                body.raw(",\"sha256\":").str(sha).raw("}");
            }
            body.raw("]");
        }
    }
    body.raw("}");
    Ok(body.0)
}

/// A usage error (exit 2) of this command: a key `err.cli.import.*` and its parameters.
fn usage(key: &'static str, params: &[(&'static str, String)]) -> Outcome {
    let mut e = Error::new(key);
    for (k, v) in params {
        e = e.with(k, v);
    }
    Outcome::fail_error(&e)
}

/// `a` and `b` name one file once symlinks and `..` are resolved (a file that is not there yet is its folder plus its name).
fn same_file(a: &str, b: &str) -> bool {
    let real = |p: &str| {
        let path = Path::new(p);
        path.canonicalize().ok().or_else(|| {
            let dir = path
                .parent()
                .filter(|d| !d.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            Some(dir.canonicalize().ok()?.join(path.file_name()?))
        })
    };
    matches!((real(a), real(b)), (Some(x), Some(y)) if x == y)
}

/// Writes `text` at `path` through a temporary file of the same folder and a rename, so a failed write leaves the old file as it was.
fn write_out(path: &str, text: &str) -> std::io::Result<()> {
    let part = format!("{path}.part");
    let done = std::fs::write(&part, text).and_then(|_| std::fs::rename(&part, path));
    if done.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    done
}

/// The arguments checked: (META.csv, files, --samples, --out), or the outcome that ends the command (help, usage error).
type Parsed = (String, Vec<String>, Option<String>, Option<String>);

fn arguments(args: Vec<String>, dry: bool) -> Result<Parsed, Outcome> {
    let rest: Vec<String> = args.into_iter().filter(|a| a != "--json").collect();
    let p = match crate::extract::parse(rest, &["--samples", "--out"], &["--dry-run"]) {
        Ok(Some(p)) => p,
        Ok(None) => return Err(Outcome::ok(format!("{HELP}\n").into_bytes())),
        Err(e) => return Err(Outcome::fail(&e)),
    };
    let (samples, out) = (
        p.get("--samples").map(String::from),
        p.get("--out").map(String::from),
    );
    let Some((meta, paths)) = p.pos.split_first() else {
        return Err(usage("err.cli.import.needsInput", &[]));
    };
    if samples.is_some() == !paths.is_empty() {
        return Err(usage("err.cli.import.oneSource", &[]));
    }
    if dry && out.is_some() {
        return Err(usage("err.cli.import.dryOut", &[]));
    }
    if let Some(o) = &out {
        let inputs = std::iter::once(meta).chain(samples.iter()).chain(paths);
        if inputs.into_iter().any(|i| same_file(i, o)) {
            return Err(usage("err.cli.import.outIsInput", &[("path", o.clone())]));
        }
    }
    Ok((meta.clone(), paths.to_vec(), samples, out))
}

/// The engine's document, the report in it and the bare `samples/1` it holds.
fn answer(body: &str) -> Result<(String, Value, String), Outcome> {
    let doc = mzlab_core::samples::route_all("import", body).map_err(|e| engine(&e))?;
    let Ok(v) = parse(doc.as_bytes()) else {
        let mut o = usage("err.cli.import.engine", &[]);
        o.code = crate::exit::INTERNAL;
        return Err(o);
    };
    let mut o = mzlab_core::json::Out::default();
    mzlab_core::json::write_value(&mut o, v.get("samples").unwrap_or(&Value::Null));
    Ok((doc, v.get("report").cloned().unwrap_or(Value::Null), o.0))
}

pub(crate) fn run(args: Vec<String>) -> Outcome {
    let json = args.iter().any(|a| a == "--json");
    let dry = args.iter().any(|a| a == "--dry-run");
    let (meta, paths, samples, out) = match arguments(args, dry) {
        Ok(p) => p,
        Err(o) => return o,
    };
    let done = text(&meta)
        .and_then(|csv| request(&csv, samples.as_deref(), &paths))
        .and_then(|b| answer(&b));
    let (doc, report, samples_json) = match done {
        Ok(d) => d,
        Err(o) => return o,
    };
    let wrote = out.as_ref().map(|path| {
        write_out(path, &format!("{samples_json}\n")).map_err(|e| format!("{path}: {e}"))
    });
    if let Some(Err(e)) = wrote {
        return Outcome::fail_error(&Error::new("err.cli.out").with("path", e));
    }
    let complete = str_of(&report, "status") == "complete";
    let shown = if json || dry { doc } else { samples_json };
    Outcome {
        code: u8::from(!complete),
        stdout: format!("{shown}\n").into_bytes(),
        stderr: lines(&report),
        error: None,
    }
}

#[cfg(test)]
#[path = "samples_import_tests.rs"]
pub(crate) mod tests;
