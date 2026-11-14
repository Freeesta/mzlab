//! `features`: the feature table of LC-MS files as CSV; the arguments are parsed here.

use super::Entry;
use crate::{features as run_features, Outcome};
use std::path::PathBuf;

struct Args {
    files: Vec<PathBuf>,
    out: Option<PathBuf>,
    params: String,
}

fn parse(mut a: impl Iterator<Item = String>) -> std::result::Result<Option<Args>, String> {
    let mut r = Args {
        files: Vec::new(),
        out: None,
        params: String::new(),
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

/// `mzlab-cli features ...`: the feature table in CSV on the standard output or at `--out`.
fn run(args: Vec<String>) -> Outcome {
    let a = match parse(args.into_iter()) {
        Ok(Some(a)) => a,
        Ok(None) => return super::help(),
        Err(e) => return Outcome::fail(&e),
    };
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-feat{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(crate::io)
        .and_then(|_| run_features(&a.files, &a.params, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    let csv = match r {
        Ok(c) => c,
        Err(e) => return Outcome::fail_error(&e),
    };
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

pub(crate) static TABLE: &[Entry] = &[Entry {
    name: "features",
    aliases: &[],
    help: "mzlab-cli features FILE... [--out PATH] [--preset auto|pos|neg | --params FILE.json]",
    run,
}];
