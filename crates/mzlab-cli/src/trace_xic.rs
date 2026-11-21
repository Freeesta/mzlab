//! `mzlab-cli trace xic`: the extracted ion chromatogram of one m/z (`trace/1`, `rt_min,intensity`) or of a list of m/z (`trace-set/1`, a wide CSV).
//! Every trace is the page's `api/xic` answer for that m/z, so the points are the bytes the site draws; `--smooth` is the engine's 1-2-1 / 1-4-6-4-1
//! filter (`peak1d::smooth`) and `--dry-run` names what would be extracted without opening the file's data.

use mzlab_core::error::{Error, Result};

use crate::extract::{self, Parsed};
use crate::trace_cmd::{self, Answer};
use crate::Outcome;

pub const HELP_XIC: &str = "mzlab-cli trace xic: the extracted ion chromatogram of one m/z or of a list, as JSON (trace/1, trace-set/1) or CSV, the answers of the page's api/xic.

usage: mzlab-cli trace xic FILE (--mz M[,M...] | --sweep LO:HI:STEP) [--ppm P | --da D | --tol P[ppm]|D[da]] [--smooth 3|5] [--rt A,B] [--ms-level N] [--polarity +|-]
                           [--sample ID] [--format json|csv] [--out PATH] [--dry-run]

  --mz LIST   the ions' m/z, one or several separated by commas; one gives trace/1, several give trace-set/1 (a trace/1 per ion, in the order given)
  --sweep S   LO:HI:STEP, the centres LO+i*STEP (at most 20000): the bytes of --mz with that list, in one call; --tol defaults to STEP/2 da
  --ppm P     half-window in ppm of each m/z; --da D half-window in Da; --tol P ppm or D da (a suffix ppm or da is required); one of the three (required with --mz)
  --smooth K  smooth every trace with the engine's filter before the window is cut: 3 (1-2-1) or 5 (1-4-6-4-1); the evidence names it
  --rt A,B    keep the points with A <= rt <= B (minutes)
  --ms-level N the level of the scans summed (default 1)
  --polarity  the + or - part of a mixed-polarity file (a file with no such part gives an empty trace, counted in the evidence)
  --sample ID the sample of a file with several (.wiff); default the first
  --format F  json (default) or csv: rt_min,intensity for one m/z, rt_min then one xic:<mz> column per ion for several (they share the scans)
  --out PATH  write there instead of the standard output
  --dry-run   print the extraction that would run (xicplan/1: ions, window in Da, level, smoothing, sweep) and read nothing; a missing file is still exit 4";

/// Most ions one call extracts (each one reads the section once).
const MAX_IONS: usize = 500;

struct Job {
    file: std::path::PathBuf,
    sample: Option<u32>,
    win: Option<(f64, f64)>,
    csv: bool,
    out: Option<std::path::PathBuf>,
    mz: Vec<f64>,
    /// Half-window in ppm (`Some`) or Da (`None`, with `da`).
    ppm: Option<f64>,
    da: f64,
    level: u8,
    pol: Option<i8>,
    smooth: Option<usize>,
    dry: bool,
    /// `--sweep` as `(lo, hi, step)` and whether the half-window is the default STEP/2.
    sweep: Option<((f64, f64, f64), bool)>,
}

impl Job {
    /// The stderr line when `--sweep` took the half-window itself.
    fn default_note(&self) -> String {
        match self.sweep {
            Some((_, true)) => format!(
                "note: --tol defaults to STEP/2 = {} Da\n",
                extract::jnum(self.da)
            ),
            _ => String::new(),
        }
    }

    /// Half-window in Da of `mz`.
    fn tol(&self, mz: f64) -> f64 {
        self.ppm.map_or(self.da, |p| mz * p * 1e-6)
    }
}

/// `--ppm`, `--da` or `--tol`: exactly one, positive; `(ppm, da)`.
fn tolerance(p: &Parsed) -> std::result::Result<(Option<f64>, f64), String> {
    let tol = match p.get("--tol") {
        None => None,
        Some(t) => {
            let low = t.to_lowercase();
            let (num, ppm) = match (low.strip_suffix("ppm"), low.strip_suffix("da")) {
                (Some(n), _) => (n, true),
                (_, Some(n)) => (n, false),
                _ => return Err("--tol needs a unit: 10ppm or 0.2da".into()),
            };
            let v = num
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or("--tol needs a number and a unit: 10ppm or 0.2da")?;
            Some((v, ppm))
        }
    };
    match (p.num("--ppm")?, p.num("--da")?, tol) {
        (None, None, None) => Err("trace xic needs --ppm, --da or --tol".into()),
        (Some(ppm), None, None) | (None, None, Some((ppm, true))) if ppm > 0.0 => {
            Ok((Some(ppm), 0.0))
        }
        (None, Some(da), None) | (None, None, Some((da, false))) if da > 0.0 => Ok((None, da)),
        (a, b, c) if u8::from(a.is_some()) + u8::from(b.is_some()) + u8::from(c.is_some()) > 1 => {
            Err("give one of --ppm, --da, --tol".into())
        }
        _ => Err("the tolerance needs a positive number".into()),
    }
}

/// `--sweep` as `((lo, hi, step), centres)`; `--mz` and `--sweep` are one or the other.
#[allow(clippy::type_complexity)]
fn sweep(p: &Parsed) -> std::result::Result<Option<((f64, f64, f64), Vec<f64>)>, String> {
    let Some(spec) = p.get("--sweep") else {
        return Ok(None);
    };
    let centres = mzlab_core::routes_sweep::parse_sweep(spec).map_err(|e| {
        e.params
            .iter()
            .fold(e.key.to_string(), |a, (k, v)| format!("{a} {k}={v}"))
    })?;
    let n: Vec<f64> = spec
        .split(':')
        .filter_map(|t| t.trim().parse().ok())
        .collect();
    Ok(Some(((n[0], n[1], n[2]), centres)))
}

/// `--mz`: one or more positive numbers, no repeats.
fn ions(p: &Parsed) -> std::result::Result<Vec<f64>, String> {
    let list = p.get("--mz").ok_or("trace xic needs --mz")?;
    let mut mz: Vec<f64> = Vec::new();
    for w in list.split(',') {
        let v = w
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v > 0.0)
            .ok_or(format!(
                "--mz needs positive numbers separated by commas, not \"{w}\""
            ))?;
        if mz.contains(&v) {
            return Err(format!("--mz lists {v} twice"));
        }
        mz.push(v);
    }
    if mz.len() > MAX_IONS {
        return Err(format!("--mz takes at most {MAX_IONS} ions"));
    }
    Ok(mz)
}

fn job(args: Vec<String>) -> std::result::Result<std::result::Result<Job, &'static str>, String> {
    let Some(p) = extract::parse(
        args,
        &[
            "--mz",
            "--sweep",
            "--ppm",
            "--da",
            "--tol",
            "--smooth",
            "--rt",
            "--ms-level",
            "--polarity",
            "--sample",
            "--format",
            "--out",
        ],
        &["--dry-run"],
    )?
    else {
        return Ok(Err(HELP_XIC));
    };
    let file = trace_cmd::one_file(&p, "trace xic")?;
    if p.get("--mz").is_some() == p.get("--sweep").is_some() {
        return Err("err.xic.sweep.mz".into());
    }
    let sw = sweep(&p)?;
    let (mz, ppm, da, sweep) = match sw {
        None => {
            let (ppm, da) = tolerance(&p)?;
            (ions(&p)?, ppm, da, None)
        }
        Some((spec, centres)) => {
            let given = ["--ppm", "--da", "--tol"]
                .iter()
                .any(|f| p.get(f).is_some());
            let (ppm, da) = if given {
                tolerance(&p)?
            } else {
                (None, spec.2 / 2.0)
            };
            (centres, ppm, da, Some((spec, !given)))
        }
    };
    let pol = match p.get("--polarity") {
        None => None,
        Some("+") => Some(1),
        Some("-") => Some(-1),
        Some(_) => return Err("--polarity takes + or -".into()),
    };
    let smooth = match p.get("--smooth") {
        None => None,
        Some("3") => Some(3),
        Some("5") => Some(5),
        Some(_) => return Err("--smooth takes 3 or 5".into()),
    };
    Ok(Ok(Job {
        sample: p.sample(&file)?,
        win: p.window()?,
        csv: p.csv()?,
        out: p.out(),
        file,
        mz,
        ppm,
        da,
        level: p.level()?,
        pol,
        smooth,
        dry: p.flag("--dry-run"),
        sweep,
    }))
}

/// Why a trace has no signal (`None` when it has some): the section holds no scan of that level and polarity, the ion is absent, or the window cut it all.
fn emptiness(a: &Answer, win: Option<(f64, f64)>) -> Result<Option<&'static str>> {
    let bad = || Error::new("error.badIndex");
    let x = extract::tokens(&a.text, "rt").ok_or_else(bad)?;
    let y = extract::tokens(&a.text, "y").ok_or_else(bad)?;
    if x.is_empty() {
        return Ok(Some("no_scans_in_section"));
    }
    let seen = |t: &&str| !t.parse::<f64>().is_ok_and(|v| v == 0.0);
    if !y.iter().any(seen) {
        return Ok(Some("no_signal"));
    }
    let kept = extract::pairs(&x, &y, win);
    Ok(kept.iter().all(|p| !seen(&p.1)).then_some("outside_window"))
}

/// The `xicplan/1` document: what the run would extract.
fn plan(j: &Job) -> String {
    let num = extract::jnum;
    let list =
        j.mz.iter()
            .map(|&m| {
                format!(
                    "{{\"id\":{},\"mz\":{},\"tolerance_da\":{},\"tolerance_ppm\":{}}}",
                    extract::jstr(&format!("xic:{}", num(m))),
                    num(m),
                    num(j.tol(m)),
                    j.ppm.map_or("null".to_string(), num)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
    let name = j
        .file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let win = j.win.map_or("null".to_string(), |(l, h)| {
        format!("[{},{}]", num(l), num(h))
    });
    let sweep = j.sweep.map_or(String::new(), |((lo, hi, step), dflt)| {
        format!(
            ",\"sweep\":{{\"lo\":{},\"hi\":{},\"step\":{},\"n\":{},\"tol_default\":{dflt}}},\"matrix_bytes_per_scan\":{}",
            num(lo),
            num(hi),
            num(step),
            j.mz.len(),
            j.mz.len() * 8
        )
    });
    format!(
        "{{\"schema\":\"xicplan/1\",\"dry_run\":true,\"file\":{},\"sample\":{},\"ms_level\":{},\"polarity\":{},\"smooth\":{},\"rt_window_min\":{win},\"format\":\"{}\",\"output\":{},\"n_traces\":{}{sweep},\"traces\":[{list}]}}",
        extract::jstr(&name),
        j.sample.map_or("null".to_string(), |s| s.to_string()),
        j.level,
        j.pol.map_or("null".to_string(), |p| format!("\"{}\"", if p > 0 { "+" } else { "-" })),
        j.smooth.map_or("null".to_string(), |k| k.to_string()),
        if j.csv { "csv" } else { "json" },
        j.out
            .as_ref()
            .map_or("null".to_string(), |p| extract::jstr(&p.to_string_lossy())),
        j.mz.len()
    )
}

/// The wide CSV of several traces that share their scans: `rt_min,xic:<mz>,...`.
fn wide_csv(docs: &[(String, Vec<(String, String)>)]) -> Result<String> {
    let bad = || Error::new("error.badIndex");
    let first = &docs.first().ok_or_else(bad)?.1;
    let mut s = String::from("rt_min");
    for (id, pts) in docs {
        if pts.len() != first.len() || pts.iter().zip(first).any(|(a, b)| a.0 != b.0) {
            return Err(bad());
        }
        s += &format!(",{id}");
    }
    s.push('\n');
    for i in 0..first.len() {
        s += &first[i].0;
        for (_, pts) in docs {
            s += &format!(",{}", pts[i].1);
        }
        s.push('\n');
    }
    Ok(s)
}

/// The documents of a run: one extraction of every ion (one call for a sweep), each with its evidence, then the one document, the set or the wide CSV.
fn run(j: &Job, tmp: &std::path::Path) -> Result<(String, String)> {
    let mut s = extract::open_sample(&j.file, j.sample, tmp)?;
    s.note += &j.default_note();
    let mut answers = Vec::new();
    let tols: Vec<f64> = j.mz.iter().map(|&m| j.tol(m)).collect();
    // `--sweep` is one call for the whole matrix (`Routes::xic_sweep`); a list is one extraction per ion, as the page asks them (the sweep is not
    // faster per call in-process, so a list does not pay its memory check)
    let mut found = if j.sweep.is_some() {
        trace_cmd::xic_answers(&mut s, &j.mz, &tols, j.level, j.pol, j.smooth)?
    } else {
        let mut v = Vec::with_capacity(j.mz.len());
        for (&mz, &tol) in j.mz.iter().zip(&tols) {
            v.push(trace_cmd::xic_answer(
                &mut s, mz, tol, j.level, j.pol, j.smooth,
            )?);
        }
        v
    }
    .into_iter();
    for &mz in &j.mz {
        let mut a = found.next().ok_or(Error::new("error.badIndex"))?;
        if let Some(p) = j.ppm {
            a.evidence += &format!(",\"tolerance_ppm\":{}", extract::jnum(p));
        }
        if let Some(why) = emptiness(&a, j.win)? {
            a.evidence += &format!(",\"empty\":true,\"empty_reason\":\"{why}\"");
            s.note += &format!(
                "note: xic:{} is empty ({}); exit 0, the trace is all zero or absent\n",
                extract::jnum(mz),
                why.replace('_', " ")
            );
        }
        answers.push((mz, a));
    }
    let id = |mz: f64| format!("xic:{}", extract::jnum(mz));
    let text = match (answers.len(), j.csv) {
        (1, _) => {
            let (mz, a) = answers.remove(0);
            trace_cmd::render(a, &s, j.win, j.csv, &id(mz))?
        }
        (_, false) => {
            let mut docs = Vec::new();
            for (mz, a) in answers {
                let d = trace_cmd::render(a, &s, j.win, false, &id(mz))?;
                docs.push(d);
            }
            format!(
                "{{\"schema\":\"trace-set/1\",{},\"n_traces\":{},\"traces\":[{}]}}",
                trace_cmd::tail(&s.name, &s),
                docs.len(),
                docs.join(",")
            )
        }
        (_, true) => {
            let mut cols = Vec::new();
            for (mz, a) in &answers {
                let x = extract::tokens(&a.text, "rt").ok_or(Error::new("error.badIndex"))?;
                let y = extract::tokens(&a.text, "y").ok_or(Error::new("error.badIndex"))?;
                let pts = extract::pairs(&x, &y, j.win)
                    .into_iter()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect();
                cols.push((id(*mz), pts));
            }
            wide_csv(&cols)?
        }
    };
    Ok((text, s.note.clone()))
}

pub fn xic(args: Vec<String>) -> Outcome {
    let j = match job(args) {
        Ok(Ok(j)) => j,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    if j.dry {
        // the plan is what the run would do: the file must exist (exit 4), nothing is read from it
        if let Err(e) = std::fs::metadata(&j.file) {
            return Outcome::fail_error(
                &Error::new("err.flow.io").with("detail", format!("{}: {e}", j.file.display())),
            );
        }
        return trace_cmd::finish(Ok((plan(&j), j.default_note())), &j.out);
    }
    let mut o = trace_cmd::finish(extract::with_tmp("xic", |tmp| run(&j, tmp)), &j.out);
    // the refusal of the matrix is a data limit (1) and has a way out; a memory refusal of the index build keeps the code of its key
    let swept = o.error.as_ref().is_some_and(|f| {
        f.key == "error.memoryBudget"
            && f.params.iter().any(|(k, v)| k == "fase" && v == "xicSweep")
    });
    if swept {
        o.code = crate::exit::DATA;
        return o.with_hint(
            "sweep a narrower LO:HI or a larger STEP, or split the sweep in several calls",
        );
    }
    o
}

#[cfg(test)]
#[path = "trace_xic_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "trace_xic_sweep_tests.rs"]
mod sweep_tests;
