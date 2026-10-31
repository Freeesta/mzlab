//! `mzlab-cli validate FILE...`: is this mzML valid? Structure, PSI-MS vocabulary and, for `indexedmzML`, offsets and SHA-1 (`mzlab_core::mzmlvalida`).
//! One line per finding on the standard output; the summary of each file on the standard error. Exit 0: no error in any file; 1: errors; 2: usage or a
//! file that cannot be read.

use std::fs::File;
use std::path::PathBuf;

use mzlab_core::json::Out;
use mzlab_core::mzmlvalida::{json_file, valida, Problem, Report, Severity};

use crate::Outcome;

pub const HELP: &str = "mzlab-cli validate: is the mzML valid (structure, PSI-MS vocabulary, index and SHA-1)?

usage: mzlab-cli validate FILE.mzML... [--json] [--help]

  FILE...   one or more mzML (plain or indexedmzML)
  --json    one JSON array on the standard output, an object per file: {file, valid, indexed, spectra, chromatograms, problems: [{severity, path, code, message}]}

Without --json each finding is a line `error|warning PATH MESSAGE` (PATH is the XML path, MESSAGE starts with the code of the check); with
several files each line starts with `FILE: `. The summary of every file goes to the standard error. Exit code: 0 no error in any file,
1 at least one error, 2 usage or a file that cannot be read. Warnings (obsolete terms, a value without its unit) never change the code.";

struct Args {
    files: Vec<PathBuf>,
    json: bool,
}

fn parse(args: impl Iterator<Item = String>) -> Result<Option<Args>, String> {
    let mut a = Args {
        files: Vec::new(),
        json: false,
    };
    for x in args {
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--json" => a.json = true,
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ => a.files.push(PathBuf::from(x)),
        }
    }
    if a.files.is_empty() {
        return Err("validate needs at least one file".into());
    }
    Ok(Some(a))
}

fn line(file: Option<&str>, p: &Problem) -> String {
    let path = if p.path.is_empty() { "-" } else { &p.path };
    let prefix = file.map(|f| format!("{f}: ")).unwrap_or_default();
    format!(
        "{prefix}{} {path} {}: {}\n",
        p.severity.as_str(),
        p.code,
        p.message
    )
}

fn summary(file: &str, r: &Report) -> String {
    let w = r
        .problems
        .iter()
        .filter(|p| p.severity == Severity::Warning)
        .count();
    let verdict = if r.is_valid() { "valid" } else { "INVALID" };
    let kind = if r.indexed { "indexed" } else { "plain" };
    format!(
        "{file}: {verdict} ({kind}, {} spectra, {} chromatograms, {} errors, {w} warnings)\n",
        r.spectra,
        r.chromatograms,
        r.errors()
    )
}

pub fn run(args: impl Iterator<Item = String>) -> Outcome {
    let a = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let (mut text, mut json, mut err) = (String::new(), Out::default(), String::new());
    let (mut bad, mut unreadable) = (false, false);
    json.raw("[");
    for (k, f) in a.files.iter().enumerate() {
        let name = f.file_name().map_or(f.display().to_string(), |n| {
            n.to_string_lossy().into_owned()
        });
        let read = File::open(f)
            .map_err(|e| e.to_string())
            .and_then(|h| valida(h).map_err(|e| e.to_string()));
        let Ok(r) = read else {
            err += &format!(
                "{name}: cannot be read: {}\n",
                read.err().unwrap_or_default()
            );
            unreadable = true;
            continue;
        };
        bad |= !r.is_valid();
        if k > 0 && json.0.len() > 1 {
            json.raw(",");
        }
        json_file(&mut json, &name, &r);
        let prefix = (a.files.len() > 1).then_some(name.as_str());
        text += &r
            .problems
            .iter()
            .map(|p| line(prefix, p))
            .collect::<String>();
        err += &summary(&name, &r);
    }
    json.raw("]\n");
    Outcome {
        code: if unreadable { 2 } else { u8::from(bad) },
        stdout: if a.json { json.0 } else { text }.into_bytes(),
        stderr: err,
    }
}
