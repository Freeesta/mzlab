//! `features`: the feature table of LC-MS files as CSV; the arguments are parsed here.

use super::Entry;
use crate::{features_full, features_plan_of, Outcome};
use std::path::PathBuf;

struct Args {
    files: Vec<PathBuf>,
    out: Option<PathBuf>,
    /// `--notes`: where `features-notes/1` goes; without it, beside `--out`
    notes: Option<PathBuf>,
    params: String,
    /// `--sample`: the ids to take of every file with several samples
    sample: Option<Vec<u32>>,
    /// `--centroid auto|on|off`: whether profile scans at unit resolution are centroided before detection
    centroid: Option<String>,
    /// `--mz-offset DA|auto:MZ[@RT]`: the instrument's m/z offset, applied per file before the grouping (`mz_offset` of `features.align`)
    mz_offset: Option<String>,
    /// `--report`: where `features-report/1` goes
    report: Option<PathBuf>,
    /// `--dry-run`: print `features-plan/1`, detect nothing, write nothing
    dry_run: bool,
}

fn parse(mut a: impl Iterator<Item = String>) -> std::result::Result<Option<Args>, String> {
    let mut r = Args {
        files: Vec::new(),
        out: None,
        notes: None,
        params: String::new(),
        sample: None,
        centroid: None,
        mz_offset: None,
        report: None,
        dry_run: false,
    };
    let mut set = false;
    while let Some(x) = a.next() {
        let mut value = |what: &str| a.next().ok_or(format!("{x} needs {what}"));
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => r.out = Some(value("a path")?.into()),
            "--notes" => r.notes = Some(value("a path")?.into()),
            "--report" => r.report = Some(value("a path")?.into()),
            "--dry-run" => r.dry_run = true,
            "--centroid" => {
                r.centroid = match value("auto, on or off")?.as_str() {
                    m @ ("auto" | "on" | "off") => Some(m.to_string()),
                    m => return Err(format!("unknown --centroid mode {m}: use auto, on or off")),
                }
            }
            "--mz-offset" => {
                let t = value("DA or auto:MZ[@RT]")?;
                crate::mz_offset::check(&t)?;
                r.mz_offset = Some(t);
            }
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

/// `params` (the JSON text of the recipe, empty for the defaults) with the string key `key` set to `mode` (`--centroid`, `--mz-offset`).
fn with_key(params: &str, key: &str, mode: Option<&str>) -> String {
    use mzlab_core::json::{parse, Value};
    let Some(mode) = mode else {
        return params.to_string();
    };
    let mut fields = match parse(params.as_bytes()) {
        Ok(Value::Obj(f)) => f,
        _ => Vec::new(),
    };
    fields.retain(|(k, _)| k != key);
    fields.push((key.into(), Value::Str(mode.into())));
    mzlab_core::flow::value_text(&Value::Obj(fields))
}

/// Where `features-notes/1` goes: the `--notes` path, else beside `--out`: `features-notes.json` in a folder, `NAME.notes.json` beside `NAME.csv`.
fn notes_target(a: &Args) -> Option<PathBuf> {
    if let Some(n) = &a.notes {
        return Some(n.clone());
    }
    let out = a.out.as_ref()?;
    Some(if out.is_dir() {
        out.join("features-notes.json")
    } else {
        out.with_extension("notes.json")
    })
}

/// The notes of the series (`features-notes/1`, kept in `tmp` next to the first index) are copied to `--notes` or beside `--out`; with neither they
/// stay on stderr.
fn keep_notes(a: &Args, tmp: &std::path::Path) -> std::result::Result<(), String> {
    let Some(to) = notes_target(a) else {
        return Ok(());
    };
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
        .and_then(|(files, labels)| {
            let params = with_key(&a.params, "centroid", a.centroid.as_deref());
            let params = with_key(&params, "mz_offset", a.mz_offset.as_deref());
            if a.dry_run {
                return features_plan_of(&files, &labels, &params, &tmp)
                    .map(|p| (p, String::new()));
            }
            features_full(&files, &labels, &params, &tmp)
        });
    let notes = keep_notes(&a, &tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    let (csv, report) = match r {
        Ok(c) => c,
        Err(e) if e.key.starts_with("err.sample.") => return crate::sample_select::fail(&e),
        // a file that cannot be listed is a data error (1) with its own message, not a usage error
        Err(e) => return Outcome::fail_error(&e),
    };
    if a.dry_run {
        // the plan is the only output: nothing is written, not even the notes of the indexes
        return Outcome::ok(csv.into_bytes());
    }
    if let Err(e) = notes {
        return Outcome::fail(&e);
    }
    // the CSV first, then the report: a report that cannot be written (exit 6) leaves the CSV where `--out` or the standard output put it
    let mut done = match &a.out {
        None => Outcome::ok(csv.into_bytes()),
        Some(p) => {
            let to = if p.is_dir() {
                p.join("features.csv")
            } else {
                p.clone()
            };
            if let Err(e) = std::fs::write(&to, csv) {
                return out_failure(&to, &e);
            }
            Outcome::ok(Vec::new())
        }
    };
    if let Some(p) = &a.report {
        let to = if p.is_dir() {
            p.join("features-report.json")
        } else {
            p.clone()
        };
        if let Err(e) = std::fs::write(&to, report) {
            let mut failed = out_failure(&to, &e);
            failed.stdout = std::mem::take(&mut done.stdout);
            return failed;
        }
    }
    done
}

/// A file that cannot be written: `err.cli.out`, exit 6.
fn out_failure(to: &std::path::Path, e: &std::io::Error) -> Outcome {
    Outcome::fail_error(
        &mzlab_core::error::Error::new("err.cli.out")
            .with("path", format!("{}: {e}", to.display())),
    )
}

pub(crate) static TABLE: &[Entry] = &[Entry::new(
    "features",
    "mzlab-cli features FILE... [--sample LIST] [--centroid auto|on|off] [--mz-offset DA|auto:MZ[@RT]] [--report PATH] [--dry-run] [--out PATH] [--notes PATH] [--preset auto|pos|neg | --params FILE.json]",
    run,
), Entry::new(
    "table compare",
    "mzlab-cli table compare TABLE --ref SEL --blank SEL [--samples FILE.json] [--quantity area|height] [--factor F] [--pseudocount auto|N] [--format json|csv] [--json] [--dry-run] [--out PATH]   each feature vs the reference and the blank: candidate products",
    crate::table_compare_cmd::run,
)
.schema("compare/1")
.exits(&[0, 2, 4, 5, 6])
.examples(&["mzlab-cli table compare features.csv --ref t0* --blank MQ*"])];
