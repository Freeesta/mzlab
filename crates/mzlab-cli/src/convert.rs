//! `mzlab-cli file convert INPUT... --out FOLDER`: vendor files, maker folders and folders of them to indexed mzML. A thin caller of the executor of the step
//! `convert.mzml` (`flow_base::convert_job`): it builds the one-step plan, runs it with the cache of the folder and places `NAME.mzML` beside it.
//! Exit codes: 0 done, 1 some file failed (the others are written), 2 usage, 3 plan not valid, 4 an input is missing. Rules: `docs/agenti/cli.md`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mzlab_core::flow::{file_roots, parse, plan_json, Run};
use mzlab_core::json::Out;

use crate::convert_entries::{entries, Entry};
pub(crate) use crate::convert_help::HELP;
use crate::convert_report::fail;
use crate::convert_report::{compression_sets, head_of, summary, write_report, Item};
use crate::flow::{steps, DoneFn, FileRunner};
use crate::flow_base::convert_file;
use crate::flow_run::{apply_set, fill_inputs, prepare, problem_text, run_checked};
use crate::flow_set::{digest_known, is_set_folder, known_sha};
use crate::Outcome;

#[derive(Default)]
pub(crate) struct Args {
    inputs: Vec<PathBuf>,
    out: Option<PathBuf>,
    sets: Vec<String>,
    report: Option<PathBuf>,
    /// `--workers`; `None` = the default of `workers_for`
    pub(crate) workers: Option<usize>,
    fresh: bool,
    /// `--sample`: the ids to convert of a dataset with several samples
    sample: Option<Vec<u32>>,
    /// `--json`: the `convert/1` document on stdout instead of the list of files
    json: bool,
    /// `--dry-run`: say what would be converted, write nothing
    dry_run: bool,
    /// `--name sample`: the mzML is called after the sample, not after its file
    name_by_sample: bool,
}

fn value(it: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    it.next().ok_or(format!("{name} needs a value"))
}

pub(crate) fn parse_args(args: Vec<String>) -> Result<Option<Args>, String> {
    let mut a = Args::default();
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => a.out = Some(value(&mut it, &x)?.into()),
            "--report" => a.report = Some(value(&mut it, &x)?.into()),
            "--zlib" => a
                .sets
                .push(format!("conv.zlib_level={}", value(&mut it, &x)?)),
            "--numpress" => a.sets.push("conv.numpress=true".into()),
            "--compression" => a.sets.extend(compression_sets(&value(&mut it, &x)?)?),
            "--peaks" => {
                let v = value(&mut it, &x)?;
                if !["file", "profile", "mzlab"].contains(&v.as_str()) {
                    return Err(format!("--peaks {v}: expected file, profile or mzlab"));
                }
                a.sets.push(format!("conv.peaks={v}"));
            }
            "--set" => a.sets.push(value(&mut it, &x)?),
            "--fresh" => a.fresh = true,
            "--json" => a.json = true,
            "--dry-run" => a.dry_run = true,
            "--name" => match value(&mut it, &x)?.as_str() {
                "sample" => a.name_by_sample = true,
                "file" => a.name_by_sample = false,
                v => return Err(format!("--name {v}: expected file or sample")),
            },
            "--sample" => {
                a.sample = Some(
                    crate::sample_select::parse(&value(&mut it, &x)?)
                        .map_err(|e| crate::error_text::human_error(&e))?,
                )
            }
            "--workers" => {
                let v = value(&mut it, &x)?;
                a.workers = Some(
                    v.parse()
                        .ok()
                        .filter(|n| *n >= 1)
                        .ok_or(format!("--workers {v}: expected a whole number from 1"))?,
                );
            }
            _ if x.starts_with("--") => return Err(format!("unknown option {x}")),
            _ => a.inputs.push(x.into()),
        }
    }
    if a.inputs.is_empty() {
        return Err("no input: pass a file, a maker's folder or a folder that holds them".into());
    }
    Ok(Some(a))
}

fn lower(p: &Path) -> String {
    p.file_name()
        .map_or(String::new(), |n| n.to_string_lossy().to_lowercase())
}

/// Is the file `p` one a reader of the registry takes (its `.wiff.scan` comes with its `.wiff`)?
fn wanted(p: &Path, known: &[&str]) -> bool {
    let n = lower(p);
    !n.ends_with(".wiff.scan") && known.iter().any(|e| n.ends_with(e))
}

/// The datasets in the folder `dir`, in name order; the folder `skip` (the output) is not looked into.
fn scan(dir: &Path, skip: &Path, known: &[&str], out: &mut Vec<PathBuf>) -> Result<(), String> {
    let list = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut entries: Vec<PathBuf> = list.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.canonicalize().is_ok_and(|c| c == skip) {
            continue;
        }
        if is_set_folder(&p) || (p.is_file() && wanted(&p, known)) {
            out.push(p);
        } else if p.is_dir() {
            scan(&p, skip, known, out)?;
        }
    }
    Ok(())
}

/// The datasets the inputs name, each once: files and maker's folders as given, other folders looked through.
fn gather(inputs: &[PathBuf], skip: &Path) -> Result<Vec<PathBuf>, String> {
    let known = mzlab_lettori::estensioni();
    let mut out = Vec::new();
    for p in inputs {
        if !p.exists() {
            return Err(format!("{}: no such file or folder", p.display()));
        }
        if p.is_dir() && !is_set_folder(p) {
            scan(p, skip, &known, &mut out)?;
        } else {
            out.push(p.clone());
        }
    }
    let mut seen = Vec::new();
    out.retain(|p| {
        let c = p.canonicalize().unwrap_or_else(|_| p.clone());
        !seen.contains(&c) && {
            seen.push(c);
            true
        }
    });
    if out.is_empty() {
        return Err("nothing to convert in the inputs".into());
    }
    Ok(out)
}

/// The plan of the one step: every entry an input of the port `raw`.
fn plan_text(entries: &[Entry], version: u32) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":\"mzflow/2\",\"inputs\":[");
    for (i, e) in entries.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(&e.label)
            .raw(",\"type\":\"raw\"}");
    }
    o.raw("],\"steps\":[{\"id\":\"conv\",\"step\":")
        .str(&format!("convert.mzml@{version}"));
    o.raw(",\"in\":{\"raw\":[");
    for (i, e) in entries.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .str(&format!("file:{}", e.label));
    }
    o.raw("]}}]}");
    o.0
}

/// How each job ended: milliseconds, taken from the cache, the error.
type Ended = (f64, bool, Option<String>);

/// The callback of the runner: one progress line per file, the outcome kept for the report.
fn progress(names: Vec<String>, ended: Arc<Mutex<Vec<Option<Ended>>>>) -> DoneFn {
    let (total, count) = (names.len(), AtomicUsize::new(0));
    Arc::new(move |_, item, ms, cached, err| {
        let k = item.unwrap_or(0);
        let n = count.fetch_add(1, Ordering::Relaxed) + 1;
        let what = match (&err, cached) {
            (Some(e), _) => format!("failed: {}", crate::error_text::human(e)),
            (None, true) => "skipped (already converted)".to_string(),
            (None, false) => format!("done in {:.1} s", ms / 1e3),
        };
        eprintln!("[{n}/{total}] {} ... {what}", names[k]);
        ended.lock().unwrap_or_else(|e| e.into_inner())[k] = Some((ms, cached, err));
    })
}

/// A calm notice: its catalogue key and parameters (`lang/*/errors-*.js`).
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    pub key: &'static str,
    pub params: Vec<(&'static str, String)>,
}

/// The notice of a dataset whose data file holds sections the container's index does not list (`recovered` of them rebuilt as samples, `skipped` left out), or `None`.
pub fn unindexed_sections(recovered: usize, skipped: usize) -> Option<Notice> {
    (recovered + skipped > 0).then(|| Notice {
        key: "warn.wiff.unindexed",
        params: vec![
            ("n", (recovered + skipped).to_string()),
            ("recovered", recovered.to_string()),
            ("skipped", skipped.to_string()),
        ],
    })
}

/// The notice of a memory budget `auto_mb` that runs fewer than `wanted` conversions at once, or `None`: the desktop queue gives it to its
/// page, `convert` prints it (`testo_errore::avviso`).
pub fn fewer_workers(auto_mb: f64, wanted: usize) -> Option<Notice> {
    let workers = mzlab_core::budget_auto::convert_workers(auto_mb, wanted);
    (workers < wanted).then(|| Notice {
        key: "warn.convert.fewerWorkers",
        params: vec![
            ("workers", workers.to_string()),
            ("wanted", wanted.to_string()),
            ("budget_mb", format!("{}", auto_mb.floor())),
        ],
    })
}

/// The conversions run at once: `--workers` as given, else the default lowered by the budget `auto_mb`, with its notice line.
pub(crate) fn workers_for(auto_mb: f64, given: Option<usize>) -> (usize, Option<String>) {
    if let Some(n) = given {
        return (n, None);
    }
    let wanted = mzlab_core::budget_auto::convert_wanted();
    (
        mzlab_core::budget_auto::convert_workers(auto_mb, wanted),
        fewer_workers(auto_mb, wanted).map(|n| format!("note: {}", crate::error_text::warning(&n))),
    )
}

/// `to` made from `from` without a second copy of the bytes when the folder allows it.
fn place(from: &Path, to: &Path) -> std::io::Result<()> {
    let _ = std::fs::remove_file(to);
    std::fs::hard_link(from, to).or_else(|_| std::fs::copy(from, to).map(|_| ()))
}

/// The `--report` item of an entry (its file-name root in the work folder is `root`) after its job `ended` in `run`, with the mzML placed in `out`.
fn item(
    root: &str,
    e: &Entry,
    (ended, run): (Option<&Ended>, &Run),
    (work, out): (&Path, &Path),
) -> Item {
    let input = e.input.display().to_string();
    let (seconds, skipped) = match ended {
        Some((ms, cached, None)) => (ms / 1e3, *cached),
        Some((_, _, Some(cause))) => {
            return Item::Failed {
                input,
                cause: cause.clone(),
            }
        }
        None => {
            return Item::Failed {
                input,
                cause: "err.flow.worker".into(),
            }
        }
    };
    let to = out.join(&e.out);
    let made_file = work.join(convert_file("conv", root));
    // the SHA-256 the writer took while it wrote the file; a file taken from the cache is hashed
    let known = known_sha(&made_file);
    let made = place(&made_file, &to)
        .map_err(|err| err.to_string())
        .and_then(|_| digest_known(&to, known).map_err(|err| err.to_string()));
    match made {
        Ok((_, sha256)) => {
            let (spectra, conv, notes) = head_of(&to);
            Item::Done {
                input,
                output: to.display().to_string(),
                sha256,
                spectra,
                seconds,
                skipped,
                conv,
                notes,
                flagged: crate::convert_report::flagged_of(run, &e.label),
                lost: crate::convert_report::lost_of(run, &e.label),
            }
        }
        Err(cause) => Item::Failed { input, cause },
    }
}

/// The plan of `entries` with the settings of `sets`, checked and ready to run.
fn prepared(
    entries: &[Entry],
    sets: &[String],
) -> Result<
    (
        mzlab_core::flow::Plan,
        mzlab_core::flow::Plan,
        HashMap<String, PathBuf>,
    ),
    Outcome,
> {
    let reg = steps();
    let version = reg
        .iter()
        .find(|s| s.id == "convert.mzml")
        .map_or(1, |s| s.version);
    let mut plan = parse(plan_text(entries, version).as_bytes())
        .map_err(|e| fail(3, &e.to_string(), "this is a bug of mzlab-cli file convert"))?;
    for set in sets {
        apply_set(&mut plan, &reg, set)
            .map_err(|e| fail(2, &e, "mzlab-cli file convert --help"))?;
    }
    let found: HashMap<String, PathBuf> = entries
        .iter()
        .map(|e| (e.label.clone(), e.input.clone()))
        .collect();
    fill_inputs(&mut plan, |n| found[n].clone())
        .map_err(|e| fail(4, &e.to_string(), "check that the inputs can be read"))?;
    match prepare(plan.clone(), &reg) {
        Ok(Ok(ready)) => Ok((plan, ready, found)),
        Ok(Err(problems)) => {
            let list: Vec<String> = problems
                .iter()
                .map(|p| format!("  {}", problem_text(p)))
                .collect();
            Err(fail(
                2,
                &format!("the settings are not valid:\n{}", list.join("\n")),
                "see mzlab-cli file convert --help",
            ))
        }
        Err(e) => Err(fail(
            3,
            &e.to_string(),
            "this is a bug of mzlab-cli file convert",
        )),
    }
}

/// An output that would be written over its own input is a mistake of `--out`.
fn overwrites(entries: &[Entry], out: &Path) -> Option<String> {
    entries.iter().find_map(|e| {
        let to = out.join(&e.out);
        let same = to
            .canonicalize()
            .ok()
            .zip(e.input.canonicalize().ok())
            .is_some_and(|(a, b)| a == b);
        same.then(|| to.display().to_string())
    })
}

/// The work folder `work` (inside `out`), or the usage failure that says `out` cannot be written.
fn make_work(work: &Path, out: &Path) -> Result<(), Outcome> {
    std::fs::create_dir_all(work).map_err(|e| {
        fail(
            2,
            &format!("{}: {e}", out.display()),
            "--out must be a folder that can be written",
        )
    })
}

/// `mzlab-cli file convert ARGS...` (the arguments after `convert`).
/// The mzML files to make from the `units`, or the outcome that refuses them (an mzML named as input, a sample selection the datasets do not satisfy).
fn listed(a: &Args, units: &[PathBuf]) -> Result<Vec<Entry>, Outcome> {
    // only a file named on the command line: an mzML met inside a folder is still handled (re-picked with `--peaks`)
    if let Some(o) = crate::convert_dry::refuse_mzml(&a.inputs) {
        return Err(o);
    }
    entries(units, a.sample.as_deref(), a.name_by_sample)
        .map_err(|e| crate::sample_select::fail(&e))
}

pub fn run(args: Vec<String>) -> Outcome {
    let a = match parse_args(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return fail(2, &e, "mzlab-cli file convert --help"),
    };
    let Some(out) = a.out.clone() else {
        return fail(
            2,
            "convert needs --out FOLDER",
            "mzlab-cli file convert --help",
        );
    };
    let work = out.join(".convert");
    // a dry run writes nothing: not even the work folder
    if !a.dry_run
        && let Err(o) = make_work(&work, &out)
    {
        return o;
    }
    let out = out.canonicalize().unwrap_or(out);
    let units = match gather(&a.inputs, &out) {
        Ok(u) => u,
        Err(e) => {
            return fail(
                4,
                &e,
                "pass a file, a maker's folder or a folder that holds them",
            )
        }
    };
    let list = match listed(&a, &units) {
        Ok(l) => l,
        Err(o) => return o,
    };
    let naming = crate::convert_entries::notes(&list);
    if let Some(to) = overwrites(&list, &out) {
        return fail(
            2,
            &format!("{to} would be written over its input"),
            "choose another --out folder",
        );
    }
    let (plan, ready, found) = match prepared(&list, &a.sets) {
        Ok(p) => p,
        Err(o) => return o,
    };
    if a.dry_run {
        return crate::convert_entries::lead(
            &naming,
            crate::convert_dry::outcome(&list, &out, a.json),
        );
    }
    let ended = Arc::new(Mutex::new(vec![None; list.len()]));
    let names = list.iter().map(Entry::shown).collect();
    let mut runner = FileRunner::located(&work, found, Vec::new(), None, a.fresh);
    // The jobs are reported through `with_done`, not as progress lines.
    runner.progress = None;
    let runner = runner.with_done(Some(progress(names, ended.clone())));
    let text = plan_json(&plan);
    let (workers, note) = workers_for(mzlab_core::budget_auto::native_auto_mb(), a.workers);
    if let Some(n) = note.filter(|_| list.len() > workers) {
        eprintln!("{n}");
    }
    let run = match run_checked(&ready, text.as_bytes(), &runner, workers) {
        Ok((run, _)) => run,
        Err(e) => return fail(3, &e.to_string(), "this is a bug of mzlab-cli file convert"),
    };
    let ended = ended.lock().unwrap_or_else(|e| e.into_inner());
    let roots = file_roots(&plan);
    let items: Vec<Item> = list
        .iter()
        .enumerate()
        .map(|(k, e)| item(&roots[k], e, (ended[k].as_ref(), &run), (&work, &out)))
        .collect();
    if let Some(o) = write_report(a.report.as_deref(), &items, &out) {
        return o;
    }
    crate::convert_entries::lead(&naming, summary(&items, &out, a.json))
}
