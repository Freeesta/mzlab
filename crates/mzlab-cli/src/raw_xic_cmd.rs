//! `mzlab-cli raw xic FILE... --mz LIST --rt A,B` (`raw-xic/1`, `docs/agenti/raw-access.md`): at each mass and time, is there a peak, in each file? The
//! rows and the members come from `mzlab_core::rawxic` (the `/api/rawxic` route the app and the site answer), so their bytes are the route's; this
//! file only reads the flags, opens the samples and adds the cross-file `summary`, the cap and the `provenance` shared with `raw info`.

use std::path::{Path, PathBuf};

use mzlab_core::error::Result;
use mzlab_core::json::{parse, round, Out, Value};
use mzlab_core::rawxic::Ask;
use mzlab_core::routes::Query;

use crate::extract::{self, jstr, Parsed};
use crate::raw_cmd::{
    choice, error_entry, missing, name_of, prov_file, provenance, warnings_of, whole_run,
};
use crate::Outcome;

pub const XIC_HELP: &str = "mzlab-cli raw xic: at each mass and time, is there a peak, in each file? JSON (raw-xic/1): per file and mass a `verdict` (peak, trace,
nothing, no_data) with its `rule`, the numbers of `quant targets` (area, height, S/N, shape) and the ion counts behind it (Poisson S/N with a one-ion
floor, Currie levels, an upper limit for `nothing`); `provenance` names the files, thresholds and automatic choices.

usage: mzlab-cli raw xic FILE... --mz LIST --rt A,B [--da D] [--ms-level N] [--polarity +|-] [--mz-offset DA|auto:MZ[@RT]] [--set NAME=VALUE,...] [--sample ID] [--limit N] [--dry-run] [--out PATH]

  FILE        an mzML, or any file the app reads; several files give one row per file and mass, in order
  --mz LIST   masses, comma separated; each is the centre of a window of +-D Da (default 0.5)
  --rt A,B    the retention-time window, min; the sides of it (the background) are read from the same trace
  --set       min_sn=3 (S/N on the one-ion floor), min_points=3 (scans in a row with an ion), alpha=0.05 (error of the Currie levels),
              side_min=0.3 (shortest side window, min), gap_min=0.05 (gap to a side, min); every value used is in `provenance.params`
  --limit N   most rows printed (default 50); `n_total` and `truncated` say what was cut, `summary` is always whole
  --dry-run   the resolved query and each file with its cache status (raw-plan/1); reads no peak
exit 0 answered (any verdict, `nothing` too), 1 a file could not be read (its entry has `error`), 2 usage, 4 a file is missing, 5 unknown sample, 6 --out not writable";

/// Rows printed by default.
const LIMIT: usize = 50;
/// The keys of `--set` and of the route.
const SET_KEYS: &[&str] = &["min_sn", "min_points", "alpha", "side_min", "gap_min"];

/// The question: the files, the route question (`mzlab_core::rawxic::Ask`), the cap and the polarity.
pub(crate) struct XicAsk {
    files: Vec<PathBuf>,
    ask: Ask,
    limit: usize,
    pol: Option<i8>,
}

/// A query value with every byte but the unreserved ones percent-encoded (`auto:300@5` keeps its meaning through `Query::parse`).
fn enc(v: &str) -> String {
    v.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `--set a=1,b=2` (repeatable) as pairs; a name outside `SET_KEYS` is refused.
fn set_of(p: &Parsed) -> std::result::Result<Vec<(String, String)>, String> {
    let mut out: Vec<(String, String)> = Vec::new();
    for v in p.all("--set") {
        for item in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (k, val) = item
                .split_once('=')
                .filter(|(k, _)| SET_KEYS.contains(k))
                .ok_or(format!(
                    "--set takes NAME=VALUE with NAME one of {}: {item}",
                    SET_KEYS.join(", ")
                ))?;
            out.retain(|(n, _)| n != k);
            out.push((k.to_string(), val.to_string()));
        }
    }
    Ok(out)
}

fn xic_ask_of(p: &Parsed) -> std::result::Result<XicAsk, String> {
    let files: Vec<PathBuf> = p.pos.iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Err("raw xic needs a file".into());
    }
    let mz = p
        .get("--mz")
        .ok_or("raw xic needs --mz LIST (masses, comma separated)")?;
    let rt = p.get("--rt").ok_or("raw xic needs --rt A,B (minutes)")?;
    let set = set_of(p)?;
    let mut query = format!("mz={}&rt={}", enc(mz), enc(rt));
    for (flag, key) in [
        ("--da", "da"),
        ("--ms-level", "ms_level"),
        ("--mz-offset", "mz_offset"),
    ] {
        if let Some(v) = p.get(flag) {
            query += &format!("&{key}={}", enc(v));
        }
    }
    for (k, v) in &set {
        query += &format!("&{k}={}", enc(v));
    }
    let limit = match p.get("--limit") {
        None => LIMIT,
        Some(v) => v
            .parse::<usize>()
            .ok()
            .filter(|n| *n >= 1)
            .ok_or("--limit needs a whole number of rows, 1 or more")?,
    };
    let pol = match p.get("--polarity") {
        None => None,
        Some("+") => Some(1),
        Some("-") => Some(-1),
        Some(_) => return Err("--polarity takes + or -".into()),
    };
    // the route reads the same text, so a value the site would refuse is refused here before any file is opened
    let ask = Ask::of(&Query::parse(&query)).map_err(|e| crate::error_text::human_error(&e))?;
    Ok(XicAsk {
        files,
        ask,
        limit,
        pol,
    })
}

/// What one file answered: its entry, its rows, the `provenance.files` member, warnings and automatic choices.
struct Answer {
    entry: String,
    rows: Vec<String>,
    prov: String,
    warnings: Vec<String>,
    choices: Vec<String>,
}

fn one(f: &Path, sample: Option<u32>, a: &XicAsk) -> Result<(Answer, String)> {
    extract::with_tmp("raw", |tmp| {
        let mut s = extract::open_sample(f, sample, tmp)?;
        s.routes.pol = a.pol;
        let out = s.routes.raw_xic(&a.ask)?;
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
        let mut o = Out::default();
        o.raw("{\"file\":")
            .str(&s.name)
            .raw(",\"container\":")
            .str(&s.container);
        o.raw(",\"sha256\":")
            .str(&sha)
            .raw(",\"size_bytes\":")
            .raw(&size.map_or("null".into(), |n| n.to_string()));
        o.raw(",\"samples\":[{\"id\":")
            .raw(&sid)
            .raw(",\"name\":")
            .str(&sname)
            .raw(",")
            .raw(&out.members)
            .raw("}]}");
        let rows = out
            .rows
            .iter()
            .map(|r| format!("{{\"file\":{},\"sample\":{sid},{}", jstr(&s.name), &r[1..]))
            .collect();
        let mut warnings = Vec::new();
        warnings_of(&s.name, s.sample, &out.members, &mut warnings);
        let mut choices = Vec::new();
        if sample.is_none() && s.sample.is_some() {
            choices.push(choice(
                "sample",
                &sid,
                "the first sample of the file (--sample picks another)",
            ));
        }
        let cps = parse(format!("{{{}}}", out.members).as_bytes())
            .ok()
            .and_then(|v| {
                v.get("ion_quantum")
                    .and_then(|q| q.get("cps"))
                    .and_then(|c| c.f64())
            });
        choices.push(choice(
            &format!("ion_quantum {}", s.name),
            &cps.map_or("null".into(), |c| c.to_string()),
            &format!("the smallest positive intensity of {} probed scans of MS level {} (null: not ion counts, the pipeline S/N decides)", mzlab_core::rawxic::PROBE, a.ask.level),
        ));
        let prov = prov_file(&s.name, &sha, size, s.sample);
        Ok((
            Answer {
                entry: o.0,
                rows,
                prov,
                warnings,
                choices,
            },
            s.note,
        ))
    })
}

/// One `summary` row per mass: files answered, rows per verdict, the highest `sn_floor` with its file and the median area of the `peak` rows.
fn summary(masses: &[f64], per_file: &[(String, Vec<Value>)]) -> String {
    let mut out = Vec::new();
    for (k, mz) in masses.iter().enumerate() {
        let rows: Vec<(&str, &Value)> = per_file
            .iter()
            .filter_map(|(n, r)| r.get(k).map(|v| (n.as_str(), v)))
            .collect();
        let count = |w: &str| {
            rows.iter()
                .filter(|(_, r)| r.get("verdict").and_then(|v| v.str()) == Some(w))
                .count()
        };
        let best = rows
            .iter()
            .filter_map(|(n, r)| r.get("sn_floor").and_then(|v| v.f64()).map(|s| (s, *n)))
            .fold(None, |b: Option<(f64, &str)>, c| {
                if b.is_none_or(|b| c.0 > b.0) {
                    Some(c)
                } else {
                    b
                }
            });
        let mut areas: Vec<f64> = rows
            .iter()
            .filter(|(_, r)| r.get("verdict").and_then(|v| v.str()) == Some("peak"))
            .filter_map(|(_, r)| r.get("area").and_then(|v| v.f64()))
            .collect();
        areas.sort_by(f64::total_cmp);
        let median = (!areas.is_empty()).then(|| {
            let n = areas.len();
            round(
                if n % 2 == 1 {
                    areas[n / 2]
                } else {
                    (areas[n / 2 - 1] + areas[n / 2]) / 2.0
                },
                1,
            )
        });
        let mut o = Out::default();
        o.raw("{\"mz\":")
            .num(*mz)
            .raw(",\"n_files\":")
            .int(rows.len() as i64);
        for w in ["peak", "trace", "nothing", "no_data"] {
            o.raw(",\"").raw(w).raw("\":").int(count(w) as i64);
        }
        o.raw(",\"max_sn_floor\":").opt(best.map(|b| b.0));
        o.raw(",\"max_sn_floor_file\":")
            .raw(&best.map_or("null".into(), |b| jstr(b.1)));
        o.raw(",\"median_area\":").opt(median).raw("}");
        out.push(o.0);
    }
    out.join(",")
}

/// `provenance.params`: every value of the question with its default resolved.
fn params_json(p: &Parsed, a: &XicAsk) -> String {
    let k = &a.ask;
    let mut o = Out::default();
    o.raw("{\"mz\":")
        .floats(&k.mz)
        .raw(",\"da\":")
        .num(k.da)
        .raw(",\"rt\":")
        .floats(&[k.rt.0, k.rt.1]);
    o.raw(",\"ms_level\":").int(i64::from(k.level));
    o.raw(",\"polarity\":").raw(
        &a.pol
            .map_or("null".into(), |p| jstr(if p > 0 { "+" } else { "-" })),
    );
    o.raw(",\"mz_offset\":")
        .raw(&k.offset.map_or("null".into(), |s| jstr(&s.to_string())));
    o.raw(",\"sample\":")
        .raw(&p.get("--sample").map_or("null".into(), jstr));
    let t = &k.t;
    o.raw(",\"set\":{\"min_sn\":")
        .num(t.min_sn)
        .raw(",\"min_points\":")
        .int(t.min_points as i64);
    o.raw(",\"alpha\":")
        .num(t.alpha)
        .raw(",\"side_min\":")
        .num(t.side_min)
        .raw(",\"gap_min\":")
        .num(t.gap_min);
    o.raw("},\"limit\":").int(a.limit as i64).raw("}");
    o.0
}

/// `raw-plan/1`: the resolved query and each file with its cache status; no peak is read.
fn plan(p: &Parsed, a: &XicAsk) -> Outcome {
    let dir = crate::idx_cache::dir();
    let mut rows = Vec::new();
    for f in &a.files {
        let label = match p.sample(f) {
            Ok(s) => s.map_or(String::new(), |id| format!("{}@{id}", name_of(f))),
            Err(m) => return Outcome::fail(&m),
        };
        let cache = match &dir {
            None => "none".to_string(),
            Some(d) => match crate::idx_cache::peek(d, f, &label) {
                Ok((st, _, _)) => st.to_string(),
                Err(e) => return crate::sample_select::fail(&e),
            },
        };
        let size = std::fs::metadata(f).ok().map(|m| m.len());
        rows.push(format!(
            "{{\"file\":{},\"size_bytes\":{},\"cache\":\"{cache}\"}}",
            jstr(&name_of(f)),
            size.map_or("null".into(), |n| n.to_string())
        ));
    }
    let text = format!(
        "{{\"schema\":\"raw-plan/1\",\"command\":\"raw xic\",\"query\":{},\"files\":[{}]}}\n",
        params_json(p, a),
        rows.join(",")
    );
    crate::emit(text, &p.out())
}

pub fn xic_run(args: Vec<String>) -> Outcome {
    let command: Vec<String> = ["mzlab-cli", "raw", "xic"]
        .iter()
        .map(|s| s.to_string())
        .chain(args.iter().cloned())
        .collect();
    let valued = [
        "--mz",
        "--rt",
        "--da",
        "--ms-level",
        "--polarity",
        "--mz-offset",
        "--set",
        "--sample",
        "--limit",
        "--out",
    ];
    let p = match extract::parse(args, &valued, &["--dry-run"]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{XIC_HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let a = match xic_ask_of(&p) {
        Ok(a) => a,
        Err(e) => return Outcome::fail(&e),
    };
    if let Some(e) = missing(&a.files) {
        return crate::sample_select::fail(&e);
    }
    if p.flag("--dry-run") {
        return plan(&p, &a);
    }
    let (mut entries, mut per_file, mut provs, mut warnings, mut choices) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut rows, mut notes, mut bad) = (Vec::new(), String::new(), false);
    for f in &a.files {
        let sample = match p.sample(f) {
            Ok(s) => s,
            Err(m) => return Outcome::fail(&m),
        };
        match one(f, sample, &a) {
            Ok((ans, note)) => {
                let parsed = ans
                    .rows
                    .iter()
                    .filter_map(|r| parse(r.as_bytes()).ok())
                    .collect();
                per_file.push((name_of(f), parsed));
                entries.push(ans.entry);
                rows.extend(ans.rows);
                provs.push(ans.prov);
                warnings.extend(ans.warnings);
                choices.extend(ans.choices);
                notes += &note;
            }
            Err(e) if whole_run(&e) => return crate::sample_select::fail(&e),
            Err(e) => {
                bad = true;
                entries.push(error_entry(f, &e, "raw xic"));
            }
        }
    }
    choices.push(choice(
        "background",
        "null",
        "the scans of the same trace from W before the window to the gap before it and from the gap after it to W after (W = the window, at least side_min); the peak region or the window is judged",
    ));
    let (n_total, truncated) = (rows.len(), rows.len() > a.limit);
    rows.truncate(a.limit);
    let text = format!(
        "{{\"schema\":\"raw-xic/1\",\"query\":{},\"files\":[{}],\"summary\":[{}],\"rows\":[{}],\"n_total\":{n_total},\"limit\":{},\"truncated\":{truncated},\"warnings\":[{}],{}}}\n",
        params_json(&p, &a),
        entries.join(","),
        summary(&a.ask.mz, &per_file),
        rows.join(","),
        a.limit,
        warnings.join(","),
        provenance(&command, &provs, &params_json(&p, &a), &choices)
    );
    let mut o = crate::emit(text, &p.out());
    if o.code == 0 {
        o.stderr = notes;
        if bad {
            o.code = crate::exit::DATA;
        }
    }
    o
}

#[cfg(test)]
#[path = "raw_xic_cmd_tests.rs"]
pub(crate) mod tests;
