//! `mzlab-cli raw xic FILE... --mz LIST --rt A,B` (`raw-xic/1`, `docs/agenti/raw-access.md`): at each mass and time, is there a peak, in each file? The
//! rows and the members come from `mzlab_core::rawxic` (the `/api/rawxic` route the app and the site answer), so their bytes are the route's; this
//! file only reads the flags, opens the samples and adds the cross-file `summary`, the cap and the `provenance` shared with `raw info`.

use std::path::Path;

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, round, Out, Value};

use crate::extract::{self, jstr, Parsed};
use crate::raw_cmd::{
    choice, error_entry, missing, name_of, prov_file, provenance, warnings_of, whole_run,
};
use crate::raw_plan::plan_of;
use crate::raw_xic_ask::{xic_ask_of, XicAsk};
use crate::Outcome;

pub const XIC_HELP: &str = "mzlab-cli raw xic: at each mass and time, is there a peak, in each file? JSON (raw-xic/1): per file and mass a `verdict` (peak, trace,
nothing, no_data) with its `rule`, the numbers of `quant targets` (area, height, S/N, shape) and the ion counts behind it (Poisson S/N with a one-ion
floor, Currie levels, an upper limit for `nothing`); `provenance` names the files, thresholds and automatic choices.

usage: mzlab-cli raw xic FILE... --mz LIST --rt A,B [--da D] [--ms-level N] [--polarity +|-] [--mz-offset DA|auto:MZ[@RT]] [--set NAME=VALUE,...] [--ions [--scan-type ems|epi|q1|sim|other]] [--sample ID] [--limit N] [--dry-run] [--out PATH]

  FILE        an mzML, or any file the app reads; several files give one row per file and mass, in order
  --mz LIST   masses, comma separated; each is the centre of a window of +-D Da (default 0.5)
  --rt A,B    the retention-time window, min; the sides of it (the background) are read from the same trace
  --set       min_sn=3 (S/N on the one-ion floor), min_points=3 (scans in a row with an ion), alpha=0.05 (error of the Currie levels),
              side_min=0.3 (shortest side window, min), gap_min=0.05 (gap to a side, min); every value used is in `provenance.params`
  --ions      opt-in: both scales, cps and ions. Each row gains `ions_*`, `cps_net`, `sn_ions`, `verdict_ions`, each sample an `ion_scale` (ions per cps and where it
              comes from: the method fields given with --set, else 1 / ion quantum) and its `ion_scale_choice`. --scan-type names the scan type (default: read from the scans)
  --set       (with --ions) fill_ms (ems, epi), dwell_ms (sim), scan_s + step_da + mass_lo + mass_hi (q1): the method values of the scan type; without them the data estimate is used
  --limit N   most rows printed (default 50); `n_total` and `truncated` say what was cut, `summary` is always whole
  --dry-run   the resolved query and each file with its cache status (raw-plan/1); reads no peak
exit 0 answered (any verdict, `nothing` too), 1 a file could not be read or its rows are inconsistent (its entry has `error`), 2 usage, 4 a file is missing, 5 unknown sample, 6 --out not writable";

/// What one file answered: its entry, its rows, the `provenance.files` member, warnings and automatic choices.
struct Answer {
    entry: String,
    rows: Vec<String>,
    prov: String,
    warnings: Vec<String>,
    choices: Vec<String>,
}

/// The `files` entry of one file: what it is and, under `samples`, the members of the route.
pub(crate) fn entry_json(
    name: &str,
    container: &str,
    sha: &str,
    size: Option<u64>,
    sid: &str,
    sname: &str,
    members: &str,
) -> String {
    let mut o = Out::default();
    o.raw("{\"file\":")
        .str(name)
        .raw(",\"container\":")
        .str(container);
    o.raw(",\"sha256\":")
        .str(sha)
        .raw(",\"size_bytes\":")
        .raw(&size.map_or("null".into(), |n| n.to_string()));
    o.raw(",\"samples\":[{\"id\":")
        .raw(sid)
        .raw(",\"name\":")
        .str(sname)
        .raw(",")
        .raw(members)
        .raw("}]}");
    o.0
}

/// The automatic choices of one file: the sample taken when none was named and the ion quantum read from the probe.
pub(crate) fn choices_of(
    name: &str,
    sid: &str,
    first: bool,
    members: &str,
    level: u8,
    ions: bool,
) -> Vec<String> {
    let mut choices = Vec::new();
    if first {
        choices.push(choice(
            "sample",
            sid,
            "the first sample of the file (--sample picks another)",
        ));
    }
    let cps = parse(format!("{{{members}}}").as_bytes())
        .ok()
        .and_then(|v| {
            v.get("ion_quantum")
                .and_then(|q| q.get("cps"))
                .and_then(|c| c.f64())
        });
    choices.push(choice(
        &format!("ion_quantum {name}"),
        &cps.map_or("null".into(), |c| c.to_string()),
        &format!("the smallest positive intensity of {} probed scans of MS level {level} (null: not ion counts, the pipeline S/N decides)", mzlab_core::rawxic::PROBE),
    ));
    if ions {
        choices.push(crate::raw_xic_ions::scale_choice(name, members));
    }
    choices
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
        let entry = entry_json(
            &s.name,
            &s.container,
            &sha,
            size,
            &sid,
            &sname,
            &out.members,
        );
        let rows = out
            .rows
            .iter()
            .map(|r| format!("{{\"file\":{},\"sample\":{sid},{}", jstr(&s.name), &r[1..]))
            .collect();
        let mut warnings = Vec::new();
        warnings_of(&s.name, s.sample, &out.members, &mut warnings);
        let choices = choices_of(
            &s.name,
            &sid,
            sample.is_none() && s.sample.is_some(),
            &out.members,
            a.ask.level,
            a.ask.ions.is_some(),
        );
        let prov = prov_file(&s.name, &sha, size, s.sample);
        Ok((
            Answer {
                entry,
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
    o.raw("},\"limit\":").int(a.limit as i64);
    crate::raw_xic_ions::params_member(&mut o, k);
    o.raw("}");
    o.0
}

/// What the files answered, in file order.
#[derive(Default)]
struct Gathered {
    entries: Vec<String>,
    per_file: Vec<(String, Vec<Value>)>,
    provs: Vec<String>,
    warnings: Vec<String>,
    choices: Vec<String>,
    rows: Vec<String>,
    notes: String,
    bad: bool,
}

/// The rows of one file as values; a row that cannot be read or a count other than the masses' fails the file loud (`err.raw.xic.rows`, exit 1).
fn check_rows(rows: &[String], masses: usize, file: &str) -> Result<Vec<Value>> {
    let bad = |w: String| {
        Error::new("err.raw.xic.rows")
            .with("file", file)
            .with("detail", w)
    };
    let parsed: Vec<Value> = rows
        .iter()
        .map(|r| parse(r.as_bytes()).map_err(|e| bad(format!("{e:?}"))))
        .collect::<Result<_>>()?;
    if parsed.len() != masses {
        return Err(bad(format!("{} rows for {masses} masses", parsed.len())));
    }
    Ok(parsed)
}

/// Each file through `one`; a file that cannot be answered keeps its place with an `error` entry, a whole-run failure ends the command.
fn gather(p: &Parsed, a: &XicAsk) -> std::result::Result<Gathered, Outcome> {
    let mut g = Gathered::default();
    for f in &a.files {
        let sample = p.sample(f).map_err(|m| Outcome::fail(&m))?;
        let answered = one(f, sample, a).and_then(|(ans, note)| {
            let parsed = check_rows(&ans.rows, a.ask.mz.len(), &name_of(f))?;
            Ok((ans, note, parsed))
        });
        match answered {
            Ok((ans, note, parsed)) => {
                g.per_file.push((name_of(f), parsed));
                g.entries.push(ans.entry);
                g.rows.extend(ans.rows);
                g.provs.push(ans.prov);
                g.warnings.extend(ans.warnings);
                g.choices.extend(ans.choices);
                g.notes += &note;
            }
            Err(e) if whole_run(&e) => return Err(crate::sample_select::fail(&e)),
            Err(e) => {
                g.bad = true;
                g.entries.push(error_entry(f, &e, "raw xic"));
            }
        }
    }
    Ok(g)
}

/// The `raw-xic/1` document of what was gathered, capped at `--limit` rows.
fn render(p: &Parsed, a: &XicAsk, command: &[String], mut g: Gathered) -> Outcome {
    g.choices.push(choice(
        "background",
        "null",
        "the scans of the same trace from W before the window to the gap before it and from the gap after it to W after (W = the window, at least side_min); the peak region or the window is judged",
    ));
    let (n_total, truncated) = (g.rows.len(), g.rows.len() > a.limit);
    g.rows.truncate(a.limit);
    let text = format!(
        "{{\"schema\":\"raw-xic/1\",\"query\":{},\"files\":[{}],\"summary\":[{}],\"rows\":[{}],\"n_total\":{n_total},\"limit\":{},\"truncated\":{truncated},\"warnings\":[{}],{}}}\n",
        params_json(p, a),
        g.entries.join(","),
        summary(&a.ask.mz, &g.per_file),
        g.rows.join(","),
        a.limit,
        g.warnings.join(","),
        provenance(command, &g.provs, &params_json(p, a), &g.choices)
    );
    let mut o = crate::emit(text, &p.out());
    if o.code == 0 {
        o.stderr = g.notes + &crate::mz_offset::nothing_hint(a.ask.offset.is_some(), &g.rows);
        if g.bad {
            o.code = crate::exit::DATA;
        }
    }
    o
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
        "--scan-type",
        "--sample",
        "--limit",
        "--out",
    ];
    let p = match extract::parse(args, &valued, &["--dry-run", "--ions"]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{XIC_HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let a = match xic_ask_of(&p) {
        Ok(a) => a,
        Err(e) => return Outcome::fail_error(&e),
    };
    if let Some(e) = missing(&a.files) {
        return crate::sample_select::fail(&e);
    }
    if p.flag("--dry-run") {
        return plan_of(&p, &a.files, "raw xic", &params_json(&p, &a));
    }
    match gather(&p, &a) {
        Ok(g) => render(&p, &a, &command, g),
        Err(o) => o,
    }
}

#[cfg(test)]
#[path = "raw_xic_cmd_tests.rs"]
pub(crate) mod tests;
