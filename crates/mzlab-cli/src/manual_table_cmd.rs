//! `mzlab-cli peak table FILE... --set windows=PATH [--set sums=true] [--out PATH]`: the table of the integrations drawn by hand in the Data view
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

pub const HELP: &str = "mzlab-cli peak table: the integrations drawn by hand on transition / SIM traces, measured again from the files, as CSV.

usage: mzlab-cli peak table (FILE... | --set traces=PATH) --set windows=PATH [--set sums=true] [--set times=PATH] [--set columns=sn,fig,quality] [--json] [--out PATH]

  FILE        an mzML, or a vendor file the app reads (a .wiff with its .wiff.scan gives one sample per sample of the file), in the order of the page
  --set windows=PATH  a JSON list of windows [{\"file\": I, \"tr\": J, \"a\": MIN, \"b\": MIN, \"baseline\": \"zero\"|\"edges\"|\"line\", \"line\": [t0, y0, t1, y1], \"label\": NAME,
                      \"follow\": true, \"names\": {\"I\": NAME}}]:
                      I = the sample (from 0, in the order above), J = its transition in the list of the file, a and b the edges in min,
                      NAME the name given to the window (at most 40 characters, optional). follow: apply the window to the same channel of every
                      other sample that has no window of its own on it (the apex is searched within the window's width on each side; an apex
                      outside the window moves it, no peak leaves it: column origin = manual|applied|shifted|no_peak); names: a name for one
                      sample alone, by its place I
  --set sums=true     after the windows of each channel a row sum with the sum of their areas
  --set times=PATH    a JSON object {\"<sample>\": MINUTES or null} that replaces the time point a sample's name carries (null = none); the others keep theirs
  --set shifts=true   instead of the table, where the FIRST window lands in the other samples, as JSON
                      [{\"file\", \"tr\", \"a\", \"b\", \"shift\", \"origin\"}] (the bytes of the page's api/manfollow)
  --set auto=true     instead of the table, every peak of every channel of every sample found by the engine and grouped by time, as JSON
                      {\"windows\", \"log\", \"groups\", \"unmatched\"} (the bytes of the page's api/manauto); no windows file is needed
  --set params=PATH   with auto=true: a JSON file {\"rt_tol\", \"half\", \"valley\", \"sn\", \"min_pts\", \"despike\", \"split\"}; a missing key keeps its default
  --set traces=PATH   instead of FILEs: a JSON list [{\"name\", \"conc\"?, \"conc_unit\"?, \"transitions\": [{\"rt\": [MIN], \"y\": [..], \"panel\"?, \"label\"?, \"unit\"?, \"q1\"?, \"q3\"?, \"name\"?}]}] of
                      traces you supply (a chromatogram, an XIC, a DAD trace); \"corr\" on a window is a note kept in its row. With a panel on a trace the
                      table has five more columns after origin: panel, conc, conc_unit, correction, below (counts x s), area_unit
  --set columns=LIST  more columns after the others, any of: sn (sn_zone: the height over the noise of the stretch a window names with \"noise\": [a, b], empty without),
                      fig (w_half_min, plates, tailing, on the window's own baseline), quality (gaussian, asymmetry, noise_score); the bytes of the page's table
  --json      the table as JSON {\"columns\": [NAME], \"rows\": [{NAME: TEXT}]} (every cell the text of the CSV, so a number keeps its digits); only for the table
  --out PATH  write there instead of the standard output

Columns: channel (m/z of a SIM ion, q1>q3 of a transition, ion N without Q1), sample, time_min (the time point of the sample in minutes, read from its name
(t0, 3h, 1 h 30, 90 s; none for a blank or when ambiguous), empty when unknown), label (the name of the window), area (counts x s), start_min,
end_min, apex_rt_min, height, snr (peak1d around the window), baseline, overlap, pv. Rows grouped by channel, then by sample. Areas and heights in scientific notation with a decimal
point (7.84077E+04), read as numbers by a spreadsheet in any locale.";

struct Request {
    files: Vec<PathBuf>,
    windows: PathBuf,
    sums: bool,
    times: Option<PathBuf>,
    shifts: bool,
    auto: Option<PathBuf>,
    traces: Option<PathBuf>,
    columns: Vec<String>,
    json: bool,
    out: Option<PathBuf>,
}

fn parse(args: Vec<String>) -> std::result::Result<Option<Request>, String> {
    let (mut files, mut windows, mut sums, mut shifts, mut out) =
        (Vec::new(), None, false, false, None);
    let (mut auto, mut params, mut times, mut traces) = (false, None, None, None);
    let (mut columns, mut json): (Vec<String>, bool) = (Vec::new(), false);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--json" => json = true,
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--set" => {
                let v = it.next().ok_or("--set needs NAME=VALUE")?;
                match v.split_once('=') {
                    Some(("windows", w)) if !w.is_empty() => windows = Some(PathBuf::from(w)),
                    Some(("sums", b @ ("true" | "false"))) => sums = b == "true",
                    Some(("sums", b)) => return Err(format!("sums is true or false, not {b}")),
                    Some(("shifts", b @ ("true" | "false"))) => shifts = b == "true",
                    Some(("shifts", b)) => return Err(format!("shifts is true or false, not {b}")),
                    Some(("auto", b @ ("true" | "false"))) => auto = b == "true",
                    Some(("auto", b)) => return Err(format!("auto is true or false, not {b}")),
                    Some(("params", p)) if !p.is_empty() => params = Some(PathBuf::from(p)),
                    Some(("times", p)) if !p.is_empty() => times = Some(PathBuf::from(p)),
                    Some(("traces", p)) if !p.is_empty() => traces = Some(PathBuf::from(p)),
                    Some(("columns", l)) if !l.is_empty() => {
                        columns = l.split(',').map(str::to_string).collect()
                    }
                    _ => return Err(format!("unknown setting {v}")),
                }
            }
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ => files.push(PathBuf::from(a)),
        }
    }
    if files.is_empty() == traces.is_none() {
        return Err("peak table needs at least one file, or --set traces=PATH (not both)".into());
    }
    if params.is_some() && !auto {
        return Err("--set params=PATH goes with --set auto=true".into());
    }
    if json && (auto || shifts) {
        return Err("--json is for the table, which auto and shifts do not write".into());
    }
    let windows = match (windows, auto) {
        (Some(w), _) => w,
        (None, true) => PathBuf::new(),
        (None, false) => return Err("peak table needs --set windows=PATH".into()),
    };
    Ok(Some(Request {
        files,
        windows,
        sums,
        times,
        shifts,
        auto: auto.then(|| params.unwrap_or_default()),
        traces,
        columns,
        json,
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

/// The `files` of `manint` (`[{"name", "transitions"}]`) for the files given, in order; the time point is the engine's, from each name.
pub(crate) fn files_json(files: &[PathBuf], warn: &mut String) -> Result<String> {
    files_json_timed(files, warn, None)
}

/// `files_json` with the time points of `times` (`{"<sample>": minutes | null}`, the sample as the table writes it) in place of the ones of the names.
fn files_json_timed(
    files: &[PathBuf],
    warn: &mut String,
    times: Option<&mzlab_core::json::Value>,
) -> Result<String> {
    let mut taken = Vec::new();
    let mut o = Out::default();
    o.raw("[");
    let mut first = true;
    for p in files {
        for (name, tr) in samples_of(p, &mut taken, warn)? {
            o.raw(if first { "{\"name\":" } else { ",{\"name\":" })
                .str(&name);
            if let Some(t) = times.and_then(|t| t.get(mzlab_core::manint::sample_of(&name))) {
                match t.f64() {
                    Some(m) => o.raw(",\"time\":").num(m),
                    None => o.raw(",\"time\":null"),
                };
            }
            o.raw(",\"transitions\":").raw(&tr).raw("}");
            first = false;
        }
    }
    o.raw("]");
    Ok(o.0)
}

fn table(c: &Request, warn: &mut String) -> Result<String> {
    let times = match &c.times {
        Some(p) => Some(mzlab_core::json::parse(
            std::fs::read_to_string(p).map_err(io)?.as_bytes(),
        )?),
        None => None,
    };
    let files = match &c.traces {
        // traces the caller supplies (a chromatogram, an XIC, a DAD trace), as the page sends them to the engine
        Some(p) => std::fs::read_to_string(p).map_err(io)?,
        None => files_json_timed(&c.files, warn, times.as_ref())?,
    };
    if let Some(p) = &c.auto {
        let params = if p.as_os_str().is_empty() {
            String::new()
        } else {
            std::fs::read_to_string(p).map_err(io)?
        };
        return mzlab_core::manint::auto_text(&files, &params);
    }
    let windows = std::fs::read_to_string(&c.windows).map_err(io)?;
    if c.shifts {
        let list = mzlab_core::json::parse(windows.as_bytes())?;
        let first = list
            .arr()
            .first()
            .ok_or_else(|| Error::new("err.manint.channel"))?;
        let mut w = Out::default();
        mzlab_core::json::write_value(&mut w, first);
        return mzlab_core::manint::follow_text(&files, &w.0);
    }
    // the optional columns go with the windows, as the page sends them
    let windows = if c.columns.is_empty() {
        windows
    } else {
        let mut o = Out::default();
        o.raw("{\"windows\":").raw(&windows).raw(",\"columns\":[");
        for (i, n) in c.columns.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.str(n);
        }
        o.raw("]}");
        o.0
    };
    let csv = mzlab_core::manint::table_text(&files, &windows, c.sums)?;
    Ok(if c.json { csv_to_json(&csv) } else { csv })
}

/// The cells of one CSV line (a quoted cell may hold commas and doubled quotes).
fn cells_of(line: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    let mut ch = line.chars().peekable();
    while let Some(c) = ch.next() {
        match c {
            '"' if quoted && ch.peek() == Some(&'"') => {
                cur.push('"');
                ch.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// The table as `{"columns": [..], "rows": [{column: text}]}`: the keys are the header's names, in order; each cell keeps the text of the CSV.
fn csv_to_json(csv: &str) -> String {
    let mut lines = csv.lines();
    let head = cells_of(lines.next().unwrap_or(""));
    let mut o = Out::default();
    o.raw("{\"columns\":[");
    for (i, h) in head.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(h);
    }
    o.raw("],\"rows\":[");
    for (r, line) in lines.enumerate() {
        if r > 0 {
            o.raw(",");
        }
        o.raw("{");
        for (i, c) in cells_of(line).iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.str(head.get(i).map_or("", String::as_str))
                .raw(":")
                .str(c);
        }
        o.raw("}");
    }
    o.raw("]}\n");
    o.0
}

pub fn run(args: Vec<String>) -> Outcome {
    let c = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let mut warn = String::new();
    match table(&c, &mut warn) {
        Err(e) => {
            let mut o = Outcome::fail_error(&e);
            o.stderr += &warn;
            o
        }
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

#[cfg(test)]
#[path = "manual_table_traces_tests.rs"]
mod traces_tests;
