//! `mzlab-cli raw scan FILE... --rt A,B` (`raw-scan/1`, `docs/agenti/raw-access.md`): what is in the mean spectrum of a time window that is not in the
//! background beside it, per bin. The members, the summary and the rows come from `mzlab_core::rawscan` (the `/api/rawscan` route the app and the
//! site answer), so their bytes are the route's; this file reads the flags, opens the samples, caps or writes the rows and adds the `provenance`.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::rawscan::Ask;
use mzlab_core::routes::Query;

use crate::extract::{self, jstr, Parsed};
use crate::raw_cmd::{choice, error_entry, missing, prov_file, provenance, warnings_of, whole_run};
use crate::raw_plan::plan_of;
use crate::raw_xic_ask::enc;
use crate::raw_xic_cmd::{choices_of, entry_json};
use crate::Outcome;

pub const SCAN_HELP: &str = "mzlab-cli raw scan: what is in the mean spectrum of a time window that is not in the background beside it, per bin. JSON (raw-scan/1):
per file a `summary` (scans, side scans, TIC, bins with a peak) and rows by net intensity: the window's mean, the mean of its two sides, their difference,
the ratio, the net ion counts, their z and a verdict (peak: z >= min_z and net >= min_net ions per scan; trace: z over the one-sided limit; else nothing).

usage: mzlab-cli raw scan FILE... --rt A,B [--side MIN] [--gap MIN] [--bin nominal|0.1] [--mz-range LO,HI] [--top N] [--channel ID] [--ms-level N]
                          [--polarity +|-] [--mz-offset DA|auto:MZ[@RT]] [--set NAME=VALUE,...] [--sample ID] [--limit N] [--full DIR] [--dry-run] [--out PATH]

  --rt A,B    the window, min; its sides (the background) are read from the same scans
  --side MIN  shortest side window, min (default 0.3; each side is as wide as the window, at least this)    --gap MIN  gap between the window and a side (default 0.05)
  --bin       nominal (whole masses, default) or 0.1 (tenths of a dalton), each centred on its value   --mz-range LO,HI  only bins in this range (after the offset)
  --top N     rows kept per file, highest net first (default 20; 0 keeps every bin)   --limit N  most rows printed in all (default 50); `n_total`, `truncated` say what was cut
  --channel ID  a scan channel of `file channels` (ms:<key>); its MS level is used   --full DIR  write every row of every file to DIR/raw-scan.csv, print only its path, row count and SHA-256
  --set       min_z=3 (z of the net counts for a peak), min_net=1 (net ions per scan for a peak), alpha=0.05 (error of a trace), side_min, gap_min; every value used is in `provenance.params`
  --dry-run   the resolved query and each file with its cache status (raw-plan/1); reads no peak
exit 0 answered, 1 a file could not be read (its entry has `error`), 2 usage, 4 a file is missing, 5 unknown sample or channel, 6 --out or --full not writable";

/// Rows printed by default.
const LIMIT: usize = 50;
/// The keys of `--set` and of the route.
const SET_KEYS: &[&str] = &["min_z", "min_net", "alpha", "side_min", "gap_min"];
/// The columns of `--full`: the row of the route after `file` and `sample`.
const COLUMNS: &[&str] = &[
    "bin",
    "mz_center",
    "mean",
    "background",
    "net",
    "ratio",
    "background_zero",
    "counts_net",
    "z_counts",
    "verdict",
];

/// The question: the files, the route question, what the CLI itself reads and the cap.
pub(crate) struct ScanAsk {
    files: Vec<PathBuf>,
    ask: Ask,
    /// The route's query with `top` as asked; `--full` asks for every bin.
    query: String,
    limit: usize,
    pol: Option<i8>,
    full: Option<PathBuf>,
}

/// `--set a=1,b=2` (repeatable) as pairs; a name outside `SET_KEYS` is refused.
fn set_of(p: &Parsed) -> std::result::Result<Vec<(String, String)>, Error> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (flag, key) in [("--side", "side_min"), ("--gap", "gap_min")] {
        if let Some(v) = p.get(flag) {
            out.push((key.to_string(), v.to_string()));
        }
    }
    for v in p.all("--set") {
        for item in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (k, val) = item
                .split_once('=')
                .filter(|(k, _)| SET_KEYS.contains(k))
                .ok_or_else(|| {
                    Error::new("err.cli.rawscan.set")
                        .with("item", item)
                        .with("names", SET_KEYS.join(", "))
                })?;
            out.retain(|(n, _)| n != k);
            out.push((k.to_string(), val.to_string()));
        }
    }
    Ok(out)
}

fn scan_ask_of(p: &Parsed) -> std::result::Result<ScanAsk, Error> {
    let files: Vec<PathBuf> = p.pos.iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Err(Error::new("err.cli.rawscan.needsFile"));
    }
    let rt = p
        .get("--rt")
        .ok_or_else(|| Error::new("err.cli.rawscan.needsRt"))?;
    let mut query = format!("rt={}", enc(rt));
    for (flag, key) in [
        ("--bin", "bin"),
        ("--mz-range", "mz_range"),
        ("--top", "top"),
        ("--channel", "channel"),
        ("--ms-level", "ms_level"),
        ("--mz-offset", "mz_offset"),
    ] {
        if let Some(v) = p.get(flag) {
            query += &format!("&{key}={}", enc(v));
        }
    }
    for (k, v) in set_of(p)? {
        query += &format!("&{k}={}", enc(&v));
    }
    let limit = match p.get("--limit") {
        None => LIMIT,
        Some(v) => v
            .parse::<usize>()
            .ok()
            .filter(|n| *n >= 1)
            .ok_or_else(|| Error::new("err.cli.rawscan.limit").with("value", v))?,
    };
    let pol = match p.get("--polarity") {
        None => None,
        Some("+") => Some(1),
        Some("-") => Some(-1),
        Some(v) => return Err(Error::new("err.cli.rawscan.polarity").with("value", v)),
    };
    // the route reads the same text, so a value the site would refuse is refused here before any file is opened
    let ask = Ask::of(&Query::parse(&query))?;
    Ok(ScanAsk {
        files,
        ask,
        query,
        limit,
        pol,
        full: p.get("--full").map(PathBuf::from),
    })
}

/// What one file answered: its entry, its summary and rows (the file and sample in front), the `provenance.files` member, warnings and choices.
struct Answer {
    entry: String,
    summary: String,
    rows: Vec<String>,
    prov: String,
    warnings: Vec<String>,
    choices: Vec<String>,
}

/// `warn.raw.oneSide`: the window has scans on one side only, or none; `side` names the side that has them.
fn one_side(file: &str, sample: Option<u32>, summary: &str) -> Option<String> {
    let v = parse(summary.as_bytes()).ok()?;
    let n = |k: &str| v.get(k).and_then(|x| x.f64()).unwrap_or(0.0);
    let (l, r) = (n("n_left_scans"), n("n_right_scans"));
    if n("n_scans") == 0.0 || (l > 0.0 && r > 0.0) {
        return None;
    }
    let side = match (l > 0.0, r > 0.0) {
        (true, _) => "left",
        (_, true) => "right",
        _ => "none",
    };
    let mut o = Out::default();
    o.raw("{\"key\":")
        .str("warn.raw.oneSide")
        .raw(",\"params\":{\"file\":")
        .str(file)
        .raw(",\"sample\":")
        .raw(&sample.map_or("null".into(), |s| s.to_string()));
    o.raw(",\"side\":").str(side).raw("}}");
    Some(o.0)
}

/// `{"file":..,"sample":..,` in front of the members of `obj` (a JSON object).
fn tagged(name: &str, sid: &str, obj: &str) -> String {
    format!("{{\"file\":{},\"sample\":{sid},{}", jstr(name), &obj[1..])
}

fn one(f: &Path, sample: Option<u32>, a: &ScanAsk, full: bool) -> Result<(Answer, String)> {
    extract::with_tmp("raw", |tmp| {
        let mut s = extract::open_sample(f, sample, tmp)?;
        s.routes.pol = a.pol;
        // the route's rows: `--full` takes every bin, else the `top` of the question
        let ask = if full {
            Ask::of(&Query::parse(&format!("{}&top=0", a.query)))?
        } else {
            a.ask.clone()
        };
        let out = s.routes.raw_scan(&ask)?;
        let sha = s
            .routes
            .idx
            .source
            .get("sha256")
            .and_then(|v| v.str())
            .unwrap_or("")
            .to_string();
        let size = std::fs::metadata(f)
            .ok()
            .filter(|m| m.is_file())
            .map(|m| m.len());
        let sid = s.sample.map_or("null".into(), |i| i.to_string());
        let sname = s
            .listing
            .iter()
            .find(|l| Some(l.0) == s.sample)
            .map_or(s.name.clone(), |l| l.1.clone());
        let entry = entry_json(
            &s.name,
            &s.container,
            &sha,
            size,
            &sid,
            &sname,
            &out.members,
        );
        let mut warnings = Vec::new();
        warnings_of(&s.name, s.sample, &out.members, &mut warnings);
        warnings.extend(one_side(&s.name, s.sample, &out.summary));
        let level = parse(format!("{{{}}}", out.members).as_bytes())
            .ok()
            .and_then(|v| v.get("ms_level").and_then(|l| l.f64()))
            .map_or(a.ask.level, |l| l as u8);
        let choices = choices_of(
            &s.name,
            &sid,
            sample.is_none() && s.sample.is_some(),
            &out.members,
            level,
            false,
        );
        let prov = prov_file(&s.name, &sha, size, s.sample);
        Ok((
            Answer {
                entry,
                summary: tagged(&s.name, &sid, &out.summary),
                rows: out.rows.iter().map(|r| tagged(&s.name, &sid, r)).collect(),
                prov,
                warnings,
                choices,
            },
            s.note,
        ))
    })
}

/// `provenance.params`: every value of the question with its default resolved.
fn params_json(p: &Parsed, a: &ScanAsk) -> String {
    let k = &a.ask;
    let t = &k.t;
    let mut o = Out::default();
    o.raw("{\"rt\":").floats(&[k.rt.0, k.rt.1]);
    o.raw(",\"bin\":")
        .str(if k.bin == mzlab_core::rawscan::Bin::Tenth {
            "0.1"
        } else {
            "nominal"
        });
    o.raw(",\"mz_range\":")
        .raw(&k.range.map_or("null".into(), |(lo, hi)| {
            let mut r = Out::default();
            r.floats(&[lo, hi]);
            r.0
        }));
    o.raw(",\"top\":").int(k.top as i64);
    o.raw(",\"channel\":")
        .raw(&k.channel.as_deref().map_or("null".into(), jstr));
    o.raw(",\"ms_level\":").int(i64::from(k.level));
    o.raw(",\"polarity\":").raw(
        &a.pol
            .map_or("null".into(), |p| jstr(if p > 0 { "+" } else { "-" })),
    );
    o.raw(",\"mz_offset\":")
        .raw(&k.offset.map_or("null".into(), |s| jstr(&s.to_string())));
    o.raw(",\"sample\":")
        .raw(&p.get("--sample").map_or("null".into(), jstr));
    o.raw(",\"set\":{\"min_z\":")
        .num(t.min_z)
        .raw(",\"min_net\":")
        .num(t.min_net)
        .raw(",\"alpha\":")
        .num(t.alpha);
    o.raw(",\"side_min\":")
        .num(t.side_min)
        .raw(",\"gap_min\":")
        .num(t.gap_min);
    o.raw("},\"limit\":").int(a.limit as i64).raw("}");
    o.0
}

/// What the files answered, in file order.
#[derive(Default)]
struct Gathered {
    entries: Vec<String>,
    summaries: Vec<String>,
    provs: Vec<String>,
    warnings: Vec<String>,
    choices: Vec<String>,
    rows: Vec<String>,
    notes: String,
    bad: bool,
}

/// Each file through `one`; a file that cannot be answered keeps its place with an `error` entry, a whole-run failure ends the command.
fn gather(p: &Parsed, a: &ScanAsk) -> std::result::Result<Gathered, Outcome> {
    let mut g = Gathered::default();
    for f in &a.files {
        let sample = p.sample(f).map_err(|m| Outcome::fail(&m))?;
        match one(f, sample, a, a.full.is_some()) {
            Ok((ans, note)) => {
                g.entries.push(ans.entry);
                g.summaries.push(ans.summary);
                g.rows.extend(ans.rows);
                g.provs.push(ans.prov);
                g.warnings.extend(ans.warnings);
                g.choices.extend(ans.choices);
                g.notes += &note;
            }
            Err(e) if whole_run(&e) => return Err(crate::sample_select::fail(&e)),
            Err(e) => {
                g.bad = true;
                g.entries.push(error_entry(f, &e, "raw scan"));
            }
        }
    }
    Ok(g)
}

/// A CSV cell: quoted when it holds a comma, a quote or a line break; a JSON null is empty.
fn cell(v: Option<&Value>) -> String {
    let t = match v {
        None | Some(Value::Null) => return String::new(),
        Some(v) => match v.str() {
            Some(s) => s.to_string(),
            None => {
                let mut o = Out::default();
                mzlab_core::json::write_value(&mut o, v);
                o.0
            }
        },
    };
    if t.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", t.replace('"', "\"\""))
    } else {
        t
    }
}

/// `--full DIR`: every row as `DIR/raw-scan.csv` (the header `file,sample,` and the columns of a row); the answer carries its path, rows and SHA-256.
fn write_full(dir: &Path, rows: &[String]) -> Result<String> {
    let io = |e: std::io::Error| {
        Error::new("err.cli.out").with("path", format!("{}: {e}", dir.display()))
    };
    std::fs::create_dir_all(dir).map_err(io)?;
    let mut csv = format!("file,sample,{}\n", COLUMNS.join(","));
    for r in rows {
        let v = parse(r.as_bytes())?;
        let line: Vec<String> = ["file", "sample"]
            .iter()
            .chain(COLUMNS)
            .map(|k| cell(v.get(k)))
            .collect();
        csv += &(line.join(",") + "\n");
    }
    let path = dir.join("raw-scan.csv");
    std::fs::write(&path, &csv).map_err(io)?;
    let mut o = Out::default();
    o.raw("{\"path\":")
        .str(&path.to_string_lossy())
        .raw(",\"rows\":")
        .int(rows.len() as i64);
    o.raw(",\"sha256\":")
        .str(&mzlab_core::budget::sha256_hex(csv.as_bytes()))
        .raw("}");
    Ok(o.0)
}

/// The `raw-scan/1` document of what was gathered, capped at `--limit` rows (or, with `--full`, without rows).
fn render(p: &Parsed, a: &ScanAsk, command: &[String], mut g: Gathered) -> Outcome {
    g.choices.push(choice(
        "background",
        "null",
        "the scans of the same level from W before the window to the gap before it and from the gap after it to W after (W = the window, at least side_min), pooled: the mean over all side scans",
    ));
    let n_total = g.rows.len();
    let full = match &a.full {
        Some(d) => match write_full(d, &g.rows) {
            Ok(f) => {
                g.rows.clear();
                Some(f)
            }
            Err(e) => return Outcome::fail_error(&e),
        },
        None => None,
    };
    let truncated = g.rows.len() > a.limit;
    g.rows.truncate(a.limit);
    let text = format!(
        "{{\"schema\":\"raw-scan/1\",\"query\":{},\"files\":[{}],\"summary\":[{}],\"rows\":[{}],\"n_total\":{n_total},\"limit\":{},\"truncated\":{truncated},\"full\":[{}],\"warnings\":[{}],{}}}\n",
        params_json(p, a),
        g.entries.join(","),
        g.summaries.join(","),
        g.rows.join(","),
        a.limit,
        full.unwrap_or_default(),
        g.warnings.join(","),
        provenance(command, &g.provs, &params_json(p, a), &g.choices)
    );
    let mut o = crate::emit(text, &p.out());
    if o.code == 0 {
        o.stderr = g.notes;
        if g.bad {
            o.code = crate::exit::DATA;
        }
    }
    o
}

pub fn scan_run(args: Vec<String>) -> Outcome {
    let command: Vec<String> = ["mzlab-cli", "raw", "scan"]
        .iter()
        .map(|s| s.to_string())
        .chain(args.iter().cloned())
        .collect();
    let valued = [
        "--rt",
        "--side",
        "--gap",
        "--bin",
        "--mz-range",
        "--top",
        "--channel",
        "--ms-level",
        "--polarity",
        "--mz-offset",
        "--set",
        "--sample",
        "--limit",
        "--full",
        "--out",
    ];
    let p = match extract::parse(args, &valued, &["--dry-run"]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{SCAN_HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let a = match scan_ask_of(&p) {
        Ok(a) => a,
        Err(e) => return Outcome::fail_error(&e),
    };
    if let Some(e) = missing(&a.files) {
        return crate::sample_select::fail(&e);
    }
    if p.flag("--dry-run") {
        return plan_of(&p, &a.files, "raw scan", &params_json(&p, &a));
    }
    match gather(&p, &a) {
        Ok(g) => render(&p, &a, &command, g),
        Err(o) => o,
    }
}

#[cfg(test)]
#[path = "raw_scan_cmd_tests.rs"]
pub(crate) mod tests;
