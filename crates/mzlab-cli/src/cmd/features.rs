//! `features`: the feature table of LC-MS files as CSV; the arguments are parsed here.

use super::Entry;
use crate::{features_of, Outcome};
use std::path::PathBuf;

struct Args {
    files: Vec<PathBuf>,
    out: Option<PathBuf>,
    params: String,
    /// `--sample`: the ids to take of every file with several samples
    sample: Option<Vec<u32>>,
}

fn parse(mut a: impl Iterator<Item = String>) -> std::result::Result<Option<Args>, String> {
    let mut r = Args {
        files: Vec::new(),
        out: None,
        params: String::new(),
        sample: None,
    };
    let mut set = false;
    while let Some(x) = a.next() {
        let mut value = |what: &str| a.next().ok_or(format!("{x} needs {what}"));
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => r.out = Some(value("a path")?.into()),
            "--preset" | "--params" if set => {
                return Err("use --preset or --params, not both".into())
            }
            "--preset" => {
                r.params = match value("auto, pos or neg")?.as_str() {
                    "auto" => String::new(),
                    "pos" => "{\"polarity\":1}".into(),
                    "neg" => "{\"polarity\":-1}".into(),
                    p => return Err(format!("unknown preset {p}")),
                };
                set = true;
            }
            "--sample" => {
                r.sample = Some(
                    crate::sample_select::parse(&value("a list such as 1,3,5")?)
                        .map_err(|e| crate::error_text::human_error(&e))?,
                )
            }
            "--params" => {
                let p = value("a JSON file")?;
                r.params = std::fs::read_to_string(&p).map_err(|e| format!("{p}: {e}"))?;
                set = true;
            }
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            _ => r.files.push(x.into()),
        }
    }
    if r.files.is_empty() {
        return Err("no input file".into());
    }
    Ok(Some(r))
}

/// The files and the sample labels of the series: with `--sample`, a file with several samples gives one member per selected sample (`NAME@ID`, in the
/// order of the file), any other file is one member as it is; with no file of several samples the selection has nothing to apply to.
fn expand(a: &Args) -> mzlab_core::error::Result<(Vec<PathBuf>, Vec<String>)> {
    let Some(want) = &a.sample else {
        return Ok((a.files.clone(), vec![String::new(); a.files.len()]));
    };
    let (mut files, mut labels, mut applied) = (Vec::new(), Vec::new(), false);
    for f in &a.files {
        let present = crate::sample_select::present(f)?;
        if present.len() < 2 {
            files.push(f.clone());
            labels.push(String::new());
            continue;
        }
        applied = true;
        let name = f
            .file_name()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned());
        for id in mzlab_core::readers::selection::resolve(&present, want)? {
            files.push(f.clone());
            labels.push(format!("{name}@{id}"));
        }
    }
    if !applied {
        return Err(mzlab_core::error::Error::new("err.sample.single"));
    }
    Ok((files, labels))
}

/// Where `features-notes/1` goes for `--out`: `features-notes.json` in a folder, `NAME.notes.json` beside a file `NAME.csv`.
fn notes_target(out: &std::path::Path) -> PathBuf {
    if out.is_dir() {
        out.join("features-notes.json")
    } else {
        out.with_extension("notes.json")
    }
}

/// With `--out`, the notes of the series (`features-notes/1`, kept in `tmp` next to the first index) are copied beside the table; without it they
/// stay on stderr.
fn keep_notes(a: &Args, tmp: &std::path::Path) -> std::result::Result<(), String> {
    let Some(out) = &a.out else { return Ok(()) };
    let to = notes_target(out);
    // written by `features_note::report` next to `0.mzidx`; a run that failed has none and reports its own error
    match std::fs::copy(tmp.join("0.notes.json"), &to) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("{}: {e}", to.display())),
    }
}

/// `mzlab-cli features ...`: the feature table in CSV on the standard output or at `--out`.
fn run(args: Vec<String>) -> Outcome {
    let a = match parse(args.into_iter()) {
        Ok(Some(a)) => a,
        Ok(None) => return super::features_help(),
        Err(e) => return Outcome::fail(&e),
    };
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-feat{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(crate::io)
        .and_then(|_| expand(&a))
        .and_then(|(files, labels)| features_of(&files, &labels, &a.params, &tmp));
    let notes = keep_notes(&a, &tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    let csv = match r {
        Ok(c) => c,
        Err(e) if e.key.starts_with("err.sample.") => return crate::sample_select::fail(&e),
        // a file that cannot be listed is a data error (1) with its own message, not a usage error
        Err(e) => return Outcome::fail_error(&e),
    };
    if let Err(e) = notes {
        return Outcome::fail(&e);
    }
    match &a.out {
        None => Outcome::ok(csv.into_bytes()),
        Some(p) => {
            let to = if p.is_dir() {
                p.join("features.csv")
            } else {
                p.clone()
            };
            std::fs::write(to, csv).map_or_else(
                |e| Outcome::fail(&e.to_string()),
                |_| Outcome::ok(Vec::new()),
            )
        }
    }
}

pub(crate) static TABLE: &[Entry] = &[Entry::new(
    "features",
    "mzlab-cli features FILE... [--sample LIST] [--out PATH] [--preset auto|pos|neg | --params FILE.json]",
    run,
)];
