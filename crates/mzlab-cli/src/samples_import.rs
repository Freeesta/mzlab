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
  --out PATH     also write the samples/1 there (not with --dry-run)
  --dry-run      report only: the sample-import/1 document on stdout, nothing written
  --json         the sample-import/1 document on stdout ({report, samples, schema}); without it the samples/1 alone, so `> samples.json` works as with `sample table`
A row is matched to a file by exact name, then by case, then without the extension; two files for one key is `ambiguous` and nothing is guessed. `wash` is a blank of kind wash.
Stderr: one line per finding (`unmatched row`, `unmatched file`, `duplicate`, `unknown role`) with its evidence and one `auto:` line per automatic choice (a match that was not exact, the first of
duplicate rows, a role or name changed, with the value before and after). Nothing is refused for a finding: the user edits the CSV or the samples/1 and runs it again.
Exit codes: 0 every row matched and every file has one; 1 findings (the documents are still written); 2 usage; 3 the CSV or the samples file is not valid; 4 a file missing or unreadable; 6 --out cannot be written.";

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

/// The stderr lines of a report: the findings with their evidence, then the automatic choices.
fn lines(report: &Value) -> String {
    let each = |k: &str| report.get(k).map_or(&[][..], Value::arr);
    let mut out = String::new();
    for r in each("unmatched_rows") {
        let near = r.get("candidates").map_or(&[][..], Value::arr);
        let near: Vec<&str> = near.iter().filter_map(Value::str).collect();
        out += &format!(
            "unmatched row {}: \"{}\" ({}{})\n",
            num_of(r, "row"),
            str_of(r, "file"),
            str_of(r, "why"),
            if near.is_empty() {
                String::new()
            } else {
                format!(": {}", near.join(", "))
            }
        );
    }
    for f in each("unmatched_files").iter().filter_map(Value::str) {
        out += &format!("unmatched file: {f} (no row)\n");
    }
    for d in each("duplicates") {
        let rows: Vec<String> = d
            .get("rows")
            .map_or(&[][..], Value::arr)
            .iter()
            .map(|r| r.f64().unwrap_or(0.0).to_string())
            .collect();
        let same = d.get("conflict") == Some(&Value::Bool(false));
        out += &format!(
            "duplicate: {} in rows {} ({}; row {} kept)\n",
            str_of(d, "file"),
            rows.join(", "),
            if same { "same values" } else { "values differ" },
            num_of(d, "kept")
        );
    }
    for u in each("unknown_roles") {
        out += &format!(
            "unknown role: row {} \"{}\" for {} (the role of the name is kept)\n",
            num_of(u, "row"),
            str_of(u, "value"),
            str_of(u, "file")
        );
    }
    for c in each("shared_names") {
        let files: Vec<&str> = c
            .get("files")
            .map_or(&[][..], Value::arr)
            .iter()
            .filter_map(Value::str)
            .collect();
        out += &format!("note: sample name \"{}\" is on {} files ({}): fine for the two polarities of one sample\n", str_of(c, "name"), files.len(), files.join(", "));
    }
    for c in each("choices") {
        out += &format!(
            "auto: {} row {} {} \"{}\" -> \"{}\"\n",
            str_of(c, "what"),
            num_of(c, "row"),
            str_of(c, "file"),
            str_of(c, "from"),
            str_of(c, "to")
        );
    }
    out
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
        return Err(Outcome::fail("sample import needs META.csv and FILE|DIR... or --samples SAMPLES.json (sample import --help)"));
    };
    if samples.is_some() == !paths.is_empty() {
        return Err(Outcome::fail(
            "sample import takes FILE|DIR... or --samples SAMPLES.json, one of them",
        ));
    }
    if dry && out.is_some() {
        return Err(Outcome::fail("--dry-run writes nothing: drop --out"));
    }
    Ok((meta.clone(), paths.to_vec(), samples, out))
}

/// The engine's document, the report in it and the bare `samples/1` it holds.
fn answer(body: &str) -> Result<(String, Value, String), Outcome> {
    let doc = mzlab_core::samples::route_all("import", body).map_err(|e| engine(&e))?;
    let Ok(v) = parse(doc.as_bytes()) else {
        return Err(Outcome::fail(
            "sample import: the engine answered with a document that is not JSON",
        ));
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
        std::fs::write(path, format!("{samples_json}\n")).map_err(|e| format!("{path}: {e}"))
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
