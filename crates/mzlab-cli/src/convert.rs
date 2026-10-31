//! `mzlab-cli convert INPUT... --out FOLDER`: vendor files, maker folders and folders of them to indexed mzML. A thin caller of the executor of the step
//! `convert.mzml` (`flow_base::convert_job`): it builds the one-step plan, runs it with the cache of the folder and places `NAME.mzML` beside it.
//! Exit codes: 0 done, 1 some file failed (the others are written), 2 usage, 3 plan not valid, 4 an input is missing. Rules: `docs/agenti/cli.md`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mzlab_core::flow::{file_roots, parse, plan_json};
use mzlab_core::json::Out;
use mzlab_core::lettori::Limiti;

use crate::convert_report::{compression_sets, head_of, report, Item};
use crate::flow::{steps, DoneFn, FileRunner};
use crate::flow_base::convert_file;
use crate::flow_run::{apply_set, fail, fill_inputs, prepare, problem_text, run_checked};
use crate::flow_set::{digest_known, file_set, is_set_folder, known_sha};
use crate::Outcome;

pub(crate) const HELP: &str = "mzlab-cli convert: vendor files to indexed mzML, the step convert.mzml of the Flow page.

usage: mzlab-cli convert INPUT... --out FOLDER [--compression KIND] [--zlib N] [--numpress] [--peaks WHICH] [--workers N] [--report FILE.json] [--fresh] [--set conv.NAME=VALUE...]

  INPUT...             a file, a maker's folder (.D, .d, .raw) or a folder to look through; a .wiff is taken with its .wiff.scan, and a dataset
                       with several samples gives one mzML per sample
  --out FOLDER         where the mzML files are written: NAME.mzML (NAME-ID.mzML for the sample ID of a dataset with several); the cache is in
                       FOLDER/.convert
  --compression KIND   standard (zlib level 2, default), smallest (zlib and MS-Numpress) or none (uncompressed): the short form of the two options below
  --zlib N, --numpress zlib level of the arrays, 0 (none) to 9 (smallest), default 2; MS-Numpress coding of m/z and intensities
  --peaks WHICH        the peaks written: file (those of the file, default; a profile file keeps its points and the run warns), profile (the
                       points of the profile) or mzlab (centroids of mzLab)
  --workers N          files converted at once (default 1; the bytes written do not depend on it)
  --report FILE.json   one object per input: ingresso, uscita, sha256, spettri, secondi, saltato, conv/1; or ingresso, errore, causa
  --fresh              converts every file again; without it a file already converted from the same bytes, with the same parameters and the same
                       program is not converted again
  --set conv.NAME=V    a parameter of the step convert.mzml (zlib_level, numpress, peaks, dad); the options above are the short forms
  --help               this text

progress goes to stderr (one line per file), stdout lists the mzML files; exit codes: 0 done, 1 a file failed (the others are written), 2 usage, 4 an input is missing or holds nothing to convert";

#[derive(Default)]
struct Args {
    inputs: Vec<PathBuf>,
    out: Option<PathBuf>,
    sets: Vec<String>,
    report: Option<PathBuf>,
    workers: usize,
    fresh: bool,
}

fn value(it: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    it.next().ok_or(format!("{name} needs a value"))
}

fn parse_args(args: Vec<String>) -> Result<Option<Args>, String> {
    let mut a = Args {
        workers: 1,
        ..Args::default()
    };
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
            "--workers" => {
                let v = value(&mut it, &x)?;
                a.workers = v
                    .parse()
                    .ok()
                    .filter(|n| *n >= 1)
                    .ok_or(format!("--workers {v}: expected a whole number from 1"))?;
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

/// One mzML to make: the dataset, the name of the plan input (`NAME` or `NAME@ID`) and the file written.
struct Entry {
    input: PathBuf,
    label: String,
    out: String,
}

/// The ids of the samples of a dataset with more than one, else none (a dataset the reader cannot open is converted as one and fails there).
fn sample_ids(p: &Path) -> Vec<u32> {
    let opened = file_set(p).and_then(|fs| mzlab_lettori::apri(fs, &Limiti::desktop()));
    let ids: Vec<u32> = opened
        .map(|l| l.campioni().iter().map(|s| s.id).collect())
        .unwrap_or_default();
    if ids.len() > 1 {
        ids
    } else {
        Vec::new()
    }
}

/// `NAME.mzML` or `NAME-ID.mzML`, NAME without its extension unless `long`.
fn out_name(name: &str, id: Option<u32>, long: bool) -> String {
    let base = if long {
        name
    } else {
        name.rsplit_once('.').map_or(name, |p| p.0)
    };
    id.map_or(format!("{base}.mzML"), |i| format!("{base}-{i}.mzML"))
}

/// The mzML files of the datasets: one per sample of a dataset with several; two that would get the same name keep the extension of their input.
fn entries(units: &[PathBuf]) -> Vec<Entry> {
    let mut all: Vec<(PathBuf, String, Option<u32>)> = Vec::new();
    for p in units {
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let ids = sample_ids(p);
        if ids.is_empty() {
            all.push((p.clone(), name.clone(), None));
        }
        all.extend(
            ids.into_iter()
                .map(|id| (p.clone(), name.clone(), Some(id))),
        );
    }
    let short: Vec<String> = all
        .iter()
        .map(|(_, n, id)| out_name(n, *id, false))
        .collect();
    let clash = |k: usize| short.iter().filter(|s| **s == short[k]).count() > 1;
    let outs: Vec<String> = (0..all.len())
        .map(|k| out_name(&all[k].1, all[k].2, clash(k)))
        .collect();
    let mut made = Vec::new();
    for (k, (input, name, id)) in all.iter().enumerate() {
        let plain = id.map_or(name.clone(), |i| format!("{name}@{i}"));
        let unique = all.iter().filter(|(_, n, i)| (n, i) == (name, id)).count() == 1;
        // two inputs of one name (two folders) keep their position in it, or the second file would replace the first
        let taken = outs.iter().filter(|o| **o == outs[k]).count() > 1;
        let (lead, mark) = (format!("{k}/"), format!("{k}-"));
        made.push(Entry {
            input: input.clone(),
            label: format!("{}{plain}", if unique { "" } else { &lead }),
            out: format!("{}{}", if taken { &mark } else { "" }, outs[k]),
        });
    }
    made
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
            (Some(e), _) => format!("failed: {e}"),
            (None, true) => "skipped (already converted)".to_string(),
            (None, false) => format!("done in {:.1} s", ms / 1e3),
        };
        eprintln!("[{n}/{total}] {} ... {what}", names[k]);
        ended.lock().unwrap_or_else(|e| e.into_inner())[k] = Some((ms, cached, err));
    })
}

/// `to` made from `from` without a second copy of the bytes when the folder allows it.
fn place(from: &Path, to: &Path) -> std::io::Result<()> {
    let _ = std::fs::remove_file(to);
    std::fs::hard_link(from, to).or_else(|_| std::fs::copy(from, to).map(|_| ()))
}

/// The `--report` item of an entry (its file-name root in the work folder is `root`) after its job `ended`, with the mzML placed in `out`.
fn item(root: &str, e: &Entry, ended: Option<&Ended>, (work, out): (&Path, &Path)) -> Item {
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
            let (spectra, conv) = head_of(&to);
            Item::Done {
                input,
                output: to.display().to_string(),
                sha256,
                spectra,
                seconds,
                skipped,
                conv,
            }
        }
        Err(cause) => Item::Failed { input, cause },
    }
}

/// The last lines on stderr and the code: every file written, or the number that failed.
fn summary(items: &[Item], out: &Path) -> Outcome {
    let skipped = items
        .iter()
        .filter(|i| matches!(i, Item::Done { skipped: true, .. }))
        .count();
    let failed = items
        .iter()
        .filter(|i| matches!(i, Item::Failed { .. }))
        .count();
    let converted = items.len() - skipped - failed;
    let stdout: String = items
        .iter()
        .filter_map(|i| match i {
            Item::Done { output, .. } => Some(format!("{output}\n")),
            Item::Failed { .. } => None,
        })
        .collect();
    let (code, stderr) = if failed == 0 {
        (
            0,
            format!(
                "done: {converted} converted, {skipped} skipped; outputs in {}\n",
                out.display()
            ),
        )
    } else {
        let causes: String = items
            .iter()
            .filter_map(|i| match i {
                Item::Failed { input, cause } => Some(format!("error: {input}: {cause}\n")),
                Item::Done { .. } => None,
            })
            .collect();
        let line = format!(
            "error: {failed} of {} failed ({converted} converted, {skipped} skipped)\n",
            items.len()
        );
        (1, causes + &line + "hint: the other files are written; run again to retry the failed ones, the converted ones are skipped\n")
    };
    Outcome {
        code,
        stdout: stdout.into_bytes(),
        stderr,
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
        .map_err(|e| fail(3, &e.to_string(), "this is a bug of mzlab-cli convert"))?;
    for set in sets {
        apply_set(&mut plan, &reg, set).map_err(|e| fail(2, &e, "mzlab-cli convert --help"))?;
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
                "see mzlab-cli convert --help",
            ))
        }
        Err(e) => Err(fail(
            3,
            &e.to_string(),
            "this is a bug of mzlab-cli convert",
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

/// `mzlab-cli convert ARGS...` (the arguments after `convert`).
pub fn run(args: Vec<String>) -> Outcome {
    let a = match parse_args(args) {
        Ok(Some(a)) => a,
        Ok(None) => {
            return Outcome {
                code: 0,
                stdout: format!("{HELP}\n").into_bytes(),
                stderr: String::new(),
            }
        }
        Err(e) => return fail(2, &e, "mzlab-cli convert --help"),
    };
    let Some(out) = a.out.clone() else {
        return fail(2, "convert needs --out FOLDER", "mzlab-cli convert --help");
    };
    let work = out.join(".convert");
    if let Err(e) = std::fs::create_dir_all(&work) {
        return fail(
            2,
            &format!("{}: {e}", out.display()),
            "--out must be a folder that can be written",
        );
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
    let list = entries(&units);
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
    let ended = Arc::new(Mutex::new(vec![None; list.len()]));
    let names = list.iter().map(|e| e.label.clone()).collect();
    let mut runner = FileRunner::located(&work, found, Vec::new(), None, a.fresh);
    // The jobs are reported through `with_done`, not as progress lines.
    runner.progress = None;
    let runner = runner.with_done(Some(progress(names, ended.clone())));
    let text = plan_json(&plan);
    if let Err(e) = run_checked(&ready, text.as_bytes(), &runner, a.workers) {
        return fail(3, &e.to_string(), "this is a bug of mzlab-cli convert");
    }
    let ended = ended.lock().unwrap_or_else(|e| e.into_inner());
    let roots = file_roots(&plan);
    let items: Vec<Item> = list
        .iter()
        .enumerate()
        .map(|(k, e)| item(&roots[k], e, ended[k].as_ref(), (&work, &out)))
        .collect();
    if let Some(path) = &a.report
        && let Err(e) = std::fs::write(path, report(&items))
    {
        return fail(
            2,
            &format!("{}: {e}", path.display()),
            "--report must be a file that can be written",
        );
    }
    summary(&items, &out)
}
