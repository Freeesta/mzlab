//! `mzlab-cli quant targets`: a table of (m/z, RT window) targets integrated in many files, one pass per file (`mzlab_feat::quant_targets::run`, the
//! function of the Flow step `quant.targets@1` and of the browser). Files are mzML, maker's files and folders the readers take, or `.mzidx`; their
//! indexes are built once per run (or taken from `--cache DIR`), files on `--workers` threads. Progress goes to stderr, the long table (or `quant-targets/1` with `--json`) to stdout.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

use mzlab_feat::quant_targets::MAX_WORKERS;

use crate::extract;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli quant targets: every target of a table integrated in every file, one pass per file.

usage: mzlab-cli quant targets --targets FILE INPUT... [--da D] [--min-sn N] [--offset 0|DA|auto|auto:MZ[@RT]] [--blank-pattern GLOBS]
                               [--rt-ref off|auto|ID[@RT]]
                               [--set NAME=VALUE...] [--log PATH] [--format csv|json] [--json] [--dry-run] [--out PATH] [--workers N] [--cache DIR]

  --targets FILE   CSV (id,mz,rt_min,rt_max[,polarity][,role]) or JSON ([{...}] or {\"targets\":[...]}); polarity + or - (empty: every scan),
                   role target, standard or parent (where --offset auto measures the offset)
  INPUT            mzML files, maker's files or folders (.d, .raw, .wiff), folders holding them, or .mzidx indexes; every file
                   name must be new (two files of one name are refused), and a .mzidx beside its source file is skipped
  --da D           half-window of every XIC, Da, 0.001 to 5 (default 0.5)
  --min-sn N       lowest S/N of a detected peak (default 3)
  --offset         m/z offset (observed - true): 0 (default), a number of Da (|D| <= 0.6), auto (measured per file and polarity on the first
                   standard/parent row; none: 0, logged) or auto:MZ[@RT] (measured on that ion)
  --rt-ref         keep the RT windows on an internal reference when batches drift: off (default), auto (first standard/parent row) or a
                   target id, each with @RT where it is expected (else its window centre); per file every window moves by the apex found
                   minus that (never on a weak peak), written in the rt_shift_min column and the log; the search half-width is rt_span (0.6 min)
  --blank-pattern  comma list of case-insensitive globs over the file names: the blanks; a target detected in a blank flags the samples in_blank
  --set NAME=VALUE any parameter of the step quant.targets@1 (repeatable): min_points (5) and min_shape (0.5, Gaussian correlation) of an
                   acceptable shape, ms_level (1), smooth_half (0 = automatic per trace) and valley (10 %) of the peak auto-lr engine
  --log PATH       write the log (quant-targets-log/1: parameters, the offset of every file with its evidence, automatic choices, warnings)
  --format F       csv (default: file,id,area,height,sn,rt,flag,state,points,shape,start_rt,end_rt,peaks_in_window,smooth_half,mz_offset_da) or json
                   (quant-targets/1: the same rows as objects and the log); --json is --format json
  --dry-run        print quant-targets-plan/1 (files, blanks, targets, rows) and read no data
  --out PATH       write the table there instead of the standard output
  --workers N      files read at once, 1 to 8 (default: one per core, at most 8); the bytes written do not depend on it

Every row is written: area counts x s, height counts, rt min. State: detected (S/N >= --min-sn and an acceptable shape),
forced (a peak under a threshold, numbers kept and flagged) or absent (no peak, no scan). Flags (;-joined, ok when none): no_scans (no scan of that level and polarity),
no_peak, below_threshold (S/N under --min-sn, numbers kept), bad_shape, edge (apex on the first or last scan of the window), in_blank.
Exit codes: 0 ok; 2 usage (bad option or parameter); 4 targets table or a file missing or unreadable; 6 --out or --log cannot be written.";

struct Job {
    targets: PathBuf,
    inputs: Vec<PathBuf>,
    params: Vec<(String, Value)>,
    log: Option<PathBuf>,
    json: bool,
    dry: bool,
    out: Option<PathBuf>,
    workers: Option<usize>,
}

/// The flags that are step parameters: flag, parameter name.
const PARAMS: [(&str, &str); 5] = [
    ("--da", "da"),
    ("--min-sn", "min_sn"),
    ("--offset", "offset"),
    ("--blank-pattern", "blank_pattern"),
    ("--rt-ref", "rt_ref"),
];

fn job(args: Vec<String>) -> std::result::Result<Option<Job>, String> {
    let json = args.iter().any(|a| a == "--json");
    let mut valued: Vec<&str> = PARAMS.iter().map(|p| p.0).collect();
    valued.extend([
        "--targets",
        "--set",
        "--log",
        "--format",
        "--out",
        "--workers",
    ]);
    let Some(p) = extract::parse(args, &valued, &["--dry-run"])? else {
        return Ok(None);
    };
    let targets = p
        .get("--targets")
        .ok_or("quant targets needs --targets FILE")?;
    if p.pos.is_empty() {
        return Err("quant targets needs at least one file or folder".into());
    }
    let mut params: Vec<(String, Value)> = PARAMS
        .iter()
        .filter_map(|(f, k)| p.get(f).map(|v| (k.to_string(), Value::Str(v.to_string()))))
        .collect();
    for s in p.all("--set") {
        let (k, v) = s
            .split_once('=')
            .ok_or(format!("--set takes NAME=VALUE, not {s}"))?;
        params.push((k.trim().to_string(), Value::Str(v.to_string())));
    }
    let csv = match p.get("--format") {
        None => !json,
        Some("csv") if json => return Err("--json and --format csv exclude each other".into()),
        Some("csv") => true,
        Some("json") => false,
        Some(_) => return Err("--format takes csv or json".into()),
    };
    let workers = match p.get("--workers") {
        None => None,
        Some(v) => Some(
            v.parse::<usize>()
                .ok()
                .filter(|n| (1..=MAX_WORKERS).contains(n))
                .ok_or(format!(
                    "--workers {v}: expected a whole number from 1 to {MAX_WORKERS}"
                ))?,
        ),
    };
    Ok(Some(Job {
        workers,
        targets: targets.into(),
        inputs: p.pos.iter().map(PathBuf::from).collect(),
        params,
        log: p.get("--log").map(Into::into),
        json: !csv,
        dry: p.flag("--dry-run"),
        out: p.out(),
    }))
}

fn input_error(path: &Path, e: impl std::fmt::Display) -> Error {
    Error::new("err.cli.file").with("path", format!("{}: {e}", path.display()))
}

/// The files the inputs name, each once, in the order given (a folder: its files in name order). The rows name a file by its file name, so two
/// files of one name (in two subfolders, say) are refused; a `.mzidx` beside the file it was built from (same folder and stem) is skipped,
/// so that file is not integrated twice.
fn files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut known = mzlab_lettori::estensioni();
    known.extend([".mzml", ".mzidx"]);
    let mut out: Vec<PathBuf> = Vec::new();
    for p in inputs {
        if !p.exists() {
            return Err(input_error(p, "no such file or folder"));
        }
        if p.is_dir() && !crate::flow_set::is_set_folder(p) {
            crate::convert::scan(p, Path::new(""), &known, &mut out)
                .map_err(|e| input_error(p, e))?;
        } else {
            out.push(p.clone());
        }
    }
    let mut seen: Vec<PathBuf> = Vec::new();
    out.retain(|p| {
        let c = p.canonicalize().unwrap_or_else(|_| p.clone());
        !seen.contains(&c) && {
            seen.push(c);
            true
        }
    });
    let stem = |p: &Path| p.with_extension("");
    let sources: Vec<PathBuf> = out
        .iter()
        .filter(|p| !is_index(p))
        .map(|p| stem(p))
        .collect();
    out.retain(|p| {
        let beside = is_index(p) && sources.contains(&stem(p));
        if beside {
            eprintln!("note: {} skipped: its source file is an input", p.display());
        }
        !beside
    });
    let mut names: Vec<String> = Vec::new();
    for p in &out {
        let n = name_of(p);
        if names.contains(&n) {
            return Err(Error::new("err.cli.input.twice").with("input", n));
        }
        names.push(n);
    }
    if out.is_empty() {
        return Err(Error::new("err.quant.targets.noFiles"));
    }
    Ok(out)
}

fn name_of(p: &Path) -> String {
    p.file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned())
}

fn is_index(p: &Path) -> bool {
    name_of(p).to_lowercase().ends_with(".mzidx")
}

fn read(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| input_error(path, e))
}

/// Files at once when `--workers` is not given: one per core, at most `MAX_WORKERS` (past a few, one disk only seeks between the files) and at
/// most one per file.
fn default_workers(files: usize) -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(MAX_WORKERS)
        .min(files)
        .max(1)
}

fn run_job(j: &Job) -> Result<(String, String)> {
    let targets = read(&j.targets)?;
    let paths = files(&j.inputs)?;
    let names: Vec<String> = paths.iter().map(|p| name_of(p)).collect();
    if j.dry {
        return Ok((
            mzlab_feat::quant_targets::plan(&names, &targets, &j.params)?,
            String::new(),
        ));
    }
    // parameters and table are checked before any index is built
    let n = paths.len();
    let workers = j.workers.unwrap_or_else(|| default_workers(n));
    let job =
        mzlab_feat::quant_targets::prepare(&names, &targets, &j.params)?.with_workers(workers);
    // the bytes do not depend on the workers, so the count is said on stderr and not in the log
    eprintln!("note: {n} files on {workers} workers");
    let budget =
        mzlab_core::budget_auto::index_budget(mzlab_core::budget_auto::native_auto_mb(), workers);
    extract::with_tmp("quant", |tmp| {
        // each file is one pass on its own thread: its index (built here unless given), then every target; the passes are joined in file order
        let passes = crate::par::par_map_with(workers, n, |k| {
            let p = &paths[k];
            eprintln!("[{}/{n}] {}", k + 1, names[k]);
            if is_index(p) {
                return job.file(k, crate::flow_base::open(p)?);
            }
            let idx = tmp.join(format!("{k}.mzidx"));
            crate::flow_pace::build_index_of(p, "", &idx, budget)?;
            let pass = job.file(k, crate::flow_base::open(&idx)?);
            let _ = std::fs::remove_file(&idx);
            pass
        });
        let out = job.finish(passes.into_iter().collect::<Result<Vec<_>>>()?);
        for w in &out.warnings {
            eprintln!(
                "warning: {} ({w})",
                crate::error_text::catalog_text(w).unwrap_or_default()
            );
        }
        for h in &out.hints {
            eprintln!("{}", hint_line(h));
        }
        Ok((if j.json { out.json() } else { out.csv }, out.log))
    })
}

/// A hint as one stderr line: the catalogue text with its `{name}` words filled, then the key.
fn hint_line(h: &mzlab_feat::quant_targets::Hint) -> String {
    let mut t = crate::error_text::catalog_text(h.key)
        .unwrap_or_default()
        .to_string();
    for (k, v) in &h.params {
        t = t.replace(&format!("{{{k}}}"), v);
    }
    format!("hint: {t} ({})", h.key)
}

pub fn run(args: Vec<String>) -> Outcome {
    let j = match job(args) {
        Ok(Some(j)) => j,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let (table, log) = match run_job(&j) {
        Ok(t) => t,
        Err(e) => return crate::sample_select::fail(&e),
    };
    if let Some(p) = j.log.as_ref().filter(|_| !j.dry)
        && let Err(e) = std::fs::write(p, &log)
    {
        return Outcome::fail_error(
            &Error::new("err.cli.out").with("path", format!("{}: {e}", p.display())),
        );
    }
    crate::emit(table, &j.out)
}
