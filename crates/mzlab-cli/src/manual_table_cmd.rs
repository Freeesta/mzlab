//! `mzlab-cli manual-table FILE... --set windows=PATH [--set sums=true] [--out PATH]`: the table of the integrations drawn by hand in the Data view
//! (`mzlab_core::manint`), one row per window grouped by channel. Each file gives the `/api/mrm` transitions the engine gives the page (an mzML read as
//! it is, a vendor file through the chromatograms of `readers_mzml::guscio`, as the site reads it), so the CSV is the bytes of the page's export. A
//! `.wiff` gives one sample per sample of the file, named as the site names its mzML (`conversione.js::wiffName`).

use std::fs::File;
use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::fileroutes::mrm_transitions;
use mzlab_core::json::Out;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::readers::Limits;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli manual-table: the integrations drawn by hand on transition / SIM traces, measured again from the files, as CSV.

usage: mzlab-cli manual-table FILE... --set windows=PATH [--set sums=true] [--out PATH]

  FILE        an mzML, or a vendor file the app reads (a .wiff with its .wiff.scan gives one sample per sample of the file), in the order of the page
  --set windows=PATH  a JSON list of windows [{\"file\": I, \"tr\": J, \"a\": MIN, \"b\": MIN, \"baseline\": \"zero\"|\"edges\"|\"line\", \"line\": [t0, y0, t1, y1], \"label\": NAME}]:
                      I = the sample (from 0, in the order above), J = its transition in the list of the file, a and b the edges in min,
                      NAME the name given to the window (at most 40 characters, optional)
  --set sums=true     after the windows of each channel a row sum with the sum of their areas
  --out PATH  write there instead of the standard output

Columns: channel (m/z of a SIM ion, q1>q3 of a transition, ion N without Q1), name, sample, start_min, end_min, apex_rt_min, area (counts x s),
height, snr (peak1d around the window), baseline, label (the name of the window). Rows grouped by channel, then by sample. Areas and heights in scientific notation with a decimal
point (7.84077E+04), read as numbers by a spreadsheet in any locale.";

struct Request {
    files: Vec<PathBuf>,
    windows: PathBuf,
    sums: bool,
    out: Option<PathBuf>,
}

fn parse(args: Vec<String>) -> std::result::Result<Option<Request>, String> {
    let (mut files, mut windows, mut sums, mut out) = (Vec::new(), None, false, None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--set" => {
                let v = it.next().ok_or("--set needs NAME=VALUE")?;
                match v.split_once('=') {
                    Some(("windows", w)) if !w.is_empty() => windows = Some(PathBuf::from(w)),
                    Some(("sums", b @ ("true" | "false"))) => sums = b == "true",
                    Some(("sums", b)) => return Err(format!("sums is true or false, not {b}")),
                    _ => return Err(format!("unknown setting {v}")),
                }
            }
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ => files.push(PathBuf::from(a)),
        }
    }
    if files.is_empty() {
        return Err("manual-table needs at least one file".into());
    }
    let windows = windows.ok_or("manual-table needs --set windows=PATH")?;
    Ok(Some(Request {
        files,
        windows,
        sums,
        out,
    }))
}

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// Whether `given` is the reader's name of a lone sample without a DATA name: «<file> #<number>» (`wiff::nomi`), the extension as the file has it.
fn lone_placeholder(stem: &str, given: &str) -> bool {
    given
        .strip_prefix(stem)
        .and_then(|r| {
            r.get(..5)
                .filter(|e| e.eq_ignore_ascii_case(".wiff"))
                .and(r.get(5..))
        })
        .and_then(|r| r.strip_prefix(" #"))
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// The name of the mzML the site makes for a sample (named `given` by the reader) of the `.wiff` `stem`: `conversione.js::wiffName` with `freeMzml`.
fn wiff_name(stem: &str, given: &str, taken: &[String]) -> String {
    let named = !lone_placeholder(stem, given);
    let clean: String = if named {
        given
            .trim()
            .chars()
            .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
            .take(120)
            .collect()
    } else {
        String::new()
    };
    let base = if clean.is_empty() {
        stem.to_string()
    } else {
        clean
    };
    let free = |n: &str| !taken.iter().any(|t| t.eq_ignore_ascii_case(n));
    if free(&format!("{base}.mzML")) {
        return format!("{base}.mzML");
    }
    (1..)
        .map(|i| {
            if i == 1 {
                format!("{base}.wiff.mzML")
            } else {
                format!("{base}_{i}.wiff.mzML")
            }
        })
        .find(|n| free(n))
        .unwrap_or_default()
}

/// (name, `/api/mrm` transitions) of every sample of the file `p`; a sample whose records lie past the end of its file (a copy cut short) adds a line
/// to `warn`, the table keeps what was read before the cut.
fn samples_of(
    p: &Path,
    taken: &mut Vec<String>,
    warn: &mut String,
) -> Result<Vec<(String, String)>> {
    let name = p
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let lower = name.to_lowercase();
    if lower.ends_with(".mzml") {
        let f = File::open(p).map_err(io)?;
        let size = f.metadata().map_err(io)?.len();
        let mut m = Mzml::new(f, size);
        let chroms = m.chromatograms(0)?;
        taken.push(name.clone());
        return Ok(vec![(name, mrm_transitions(&mut m, &chroms)?)]);
    }
    let mut l = mzlab_lettori::apri(crate::flow_set::file_set(p)?, &Limits::desktop())?;
    let stem = name
        .rsplit_once('.')
        .map_or(name.as_str(), |s| s.0)
        .to_string();
    let wiff = lower.ends_with(".wiff");
    let list: Vec<String> = l.samples().iter().map(|c| c.nome.clone()).collect();
    let mut out = Vec::new();
    for (c, given) in list.iter().enumerate() {
        let shell = mzlab_core::readers_mzml::guscio(l.as_mut(), c)?;
        let size = shell.len() as u64;
        let mut m = Mzml::new(shell, size);
        let chroms = m.chromatograms(0)?;
        let n = if wiff {
            wiff_name(&stem, given, taken)
        } else {
            format!("{stem}.mzML")
        };
        let missing = l.missing(c);
        if missing > 0 {
            *warn += &format!("warning: {n}: {missing} records past the end of the file (cut short?): only the points before the cut are integrated\n");
        }
        taken.push(n.clone());
        out.push((n, mrm_transitions(&mut m, &chroms)?));
    }
    Ok(out)
}

/// The `files` of `manint` (`[{"name", "transitions"}]`) for the files given, in order.
pub(crate) fn files_json(files: &[PathBuf], warn: &mut String) -> Result<String> {
    let mut taken = Vec::new();
    let mut o = Out::default();
    o.raw("[");
    let mut first = true;
    for p in files {
        for (name, tr) in samples_of(p, &mut taken, warn)? {
            o.raw(if first { "{\"name\":" } else { ",{\"name\":" })
                .str(&name)
                .raw(",\"transitions\":")
                .raw(&tr)
                .raw("}");
            first = false;
        }
    }
    o.raw("]");
    Ok(o.0)
}

fn table(c: &Request, warn: &mut String) -> Result<String> {
    let files = files_json(&c.files, warn)?;
    let windows = std::fs::read_to_string(&c.windows).map_err(io)?;
    mzlab_core::manint::table_text(&files, &windows, c.sums)
}

pub fn run(args: Vec<String>) -> Outcome {
    let c = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let mut warn = String::new();
    match table(&c, &mut warn) {
        Err(e) => Outcome::fail(&format!("{warn}{}", crate::error_text::human_error(&e))),
        Ok(t) => {
            let mut o = crate::emit(t, &c.out);
            o.stderr.insert_str(0, &warn);
            o
        }
    }
}

#[cfg(test)]
#[path = "manual_table_cmd_tests.rs"]
mod tests;
