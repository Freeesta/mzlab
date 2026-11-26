//! `mzlab-cli spectrum get|list`: one mass spectrum (at a retention time, a window or a scan) as `spectrum/1` or an `mz,intensity` CSV, and the scan list as `scanlist/1`
//! or CSV. Both are the answers of the page's `api/spectrum`, `api/nearest_scan` and `api/scanlist`; the number tokens are the route's own.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, write_value, Out, Value};
use mzlab_core::routes::Query;

use crate::extract::{self, Parsed, Sample};
use crate::Outcome;

#[path = "spectrum_bins.rs"]
mod bins;
#[path = "spectrum_help.rs"]
mod help;
use help::GET_VALUES;
pub use help::HELP_GET;
#[path = "spectrum_compare.rs"]
mod compare_cmd;
#[path = "spectrum_level.rs"]
mod level;
#[path = "spectrum_list.rs"]
mod list_cmd;
#[path = "spectrum_minus.rs"]
mod minus;
#[path = "spectrum_peak.rs"]
mod peak;
#[path = "spectrum_sum.rs"]
mod sum;
pub use compare_cmd::compare;
pub use list_cmd::list;
fn finish(r: Result<(String, String)>, out: &Option<PathBuf>) -> Outcome {
    match r {
        Err(e) => crate::sample_select::fail(&e),
        Ok((t, note)) => {
            let mut o = crate::emit(t, out);
            if o.code == 0 {
                o.stderr = note;
            }
            o
        }
    }
}

pub(crate) fn query(pairs: &[(&str, String)]) -> Query {
    Query(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    )
}

fn text_of(pairs: &[(&str, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// What the request asks for, after the file was read.
#[derive(Clone, Copy)]
pub(crate) enum At {
    Scan(i64),
    Nearest(f64),
    Window(f64, f64),
}

/// The index of the scan `at` names, `None` for a window.
pub(crate) fn scan_index(
    s: &mut Sample,
    at: &At,
    level: u8,
    chan: Option<&str>,
) -> Result<Option<usize>> {
    Ok(match at {
        At::Scan(no) => {
            let scans = &s.routes.idx.scans;
            let i = (0..scans.len())
                .find(|&i| scans[i].number(i) == *no)
                .ok_or_else(|| Error::new("err.scan.unknown").with("id", no))?;
            Some(i)
        }
        At::Nearest(rt) => {
            let mut q = vec![("rt", rt.to_string()), ("level", level.to_string())];
            if let Some(c) = chan {
                q.push(("channel", c.into()));
            }
            let none = || {
                Error::new("err.scan.unknown")
                    .with("id", format!("nearest to {rt} min at MS level {level}"))
            };
            let text = s
                .routes
                .handle("/api/nearest_scan", &query(&q))?
                .ok_or_else(none)?;
            let v = parse(text.as_bytes())?;
            Some(v.get("sid").and_then(Value::f64).ok_or_else(none)? as usize)
        }
        At::Window(..) => None,
    })
}

/// The members of the route answer that are not arrays, as JSON members.
fn scalar_members(text: &str) -> Result<String> {
    let mut ans = Out::default();
    let v = parse(text.as_bytes())?;
    for (k, x) in v
        .entries()
        .iter()
        .filter(|(_, x)| !matches!(x, Value::Arr(_)))
    {
        ans.raw(if ans.0.is_empty() { "" } else { "," })
            .str(k)
            .raw(":");
        write_value(&mut ans, x);
    }
    Ok(ans.0)
}

/// The `scan` member of the evidence: the scan the request resolved to.
fn scan_json(i: usize, sc: &mzlab_core::idxread::Scan) -> String {
    format!(
        "{{\"scan_id\":{i},\"scan_no\":{},\"rt_min\":{},\"ms_level\":{}}}",
        sc.number(i),
        extract::jnum(sc.rt),
        sc.level
    )
}

/// What a request resolved to before the route is asked: the level, the scan (`scan` member of the evidence), the route query and the window.
pub(crate) struct Asked {
    pub level: u8,
    pub auto: Option<String>,
    pub resolved: String,
    pub pairs: Vec<(&'static str, String)>,
    pub window: (f64, f64),
}

pub(crate) fn asked(
    s: &mut Sample,
    at: &At,
    level: Option<u8>,
    chan: Option<&str>,
) -> Result<Asked> {
    let (mut level, auto) = level::start(s, at, level)?;
    let mut resolved = "null".to_string();
    let mut pairs: Vec<(&str, String)> = Vec::new();
    let sid = scan_index(s, at, level, chan)?;
    let window = match (sid, at) {
        (Some(i), _) => {
            let sc = &s.routes.idx.scans[i];
            // a scan given by number states its own level; the level of --rt is the one asked for
            if matches!(at, At::Scan(_)) {
                level = sc.level;
            }
            if let (true, Some(p)) = (sc.level > 1, sc.precursor) {
                pairs.push(("precursor", p.to_string()));
            }
            resolved = scan_json(i, sc);
            (sc.rt, sc.rt)
        }
        (None, At::Window(a, b)) => (*a, *b),
        _ => unreachable!(),
    };
    pairs.splice(
        0..0,
        [
            ("rt0", window.0.to_string()),
            ("rt1", window.1.to_string()),
            ("level", level.to_string()),
        ],
    );
    if let Some(c) = chan {
        pairs.push(("channel", c.into()));
    }
    Ok(Asked {
        level,
        auto,
        resolved,
        pairs,
        window,
    })
}

fn get_one(
    s: &mut Sample,
    at: &At,
    level: Option<u8>,
    chan: Option<&str>,
    csv: bool,
    minus: &mut Option<minus::Minus>,
    (sum, peak): (&Option<sum::SumOpt>, &Option<peak::PeakOpt>),
) -> Result<(String, String)> {
    let Asked {
        level,
        auto,
        resolved,
        mut pairs,
        window: (lo, hi),
    } = asked(s, at, level, chan)?;
    // the apex search of `--peak auto` makes its own window and background from the range given
    if let Some(o) = peak {
        return peak::run(s, level, (lo, hi), o, (csv, auto.as_deref()));
    }
    // the evidence of a sum reads the window as the spectrum does, before `mode` joins the query
    let sum_ev = (sum.as_ref())
        .map(|o| sum::evidence(s, &pairs, &o.fill_ms))
        .transpose()?;
    if sum.is_some() {
        pairs.push(("mode", "sum".into()));
    }
    let (text, minus_ev) = match minus {
        None => (
            s.routes
                .handle("/api/spectrum", &query(&pairs))?
                .ok_or_else(|| Error::new("err.scan.none"))?,
            String::new(),
        ),
        Some(m) => {
            let single = !matches!(at, At::Window(..));
            let (t, e) = minus::answer(s, m, &pairs, (lo, hi), single, level)?;
            (t, format!(",{e}"))
        }
    };
    let bad = || Error::new("error.badIndex");
    let x = extract::tokens(&text, "mz").ok_or_else(bad)?;
    let y = extract::tokens(&text, "y").ok_or_else(bad)?;
    let pts = extract::pairs(&x, &y, None);
    let (extra, notes) = sum::tail(sum_ev.as_deref(), at, (csv, chan.is_some()), &text)?;
    if csv {
        return Ok((extract::points_csv("mz", "intensity", &pts), notes));
    }
    let tail = format!("{minus_ev}{extra}");
    // the scan member also carries `level_auto` when the file's only level was chosen
    let scan = format!(
        "{resolved}{}",
        auto.map_or(String::new(), |a| format!(",\"level_auto\":{a}"))
    );
    Ok((doc(s, level, &pts, &scan, &pairs, &text, &tail)?, notes))
}

/// The `spectrum/1` document: the points, and as evidence the rest of the route's answer (mode, scans averaged, first and last scan, merge).
fn doc(
    s: &Sample,
    level: u8,
    pts: &[(&str, &str)],
    resolved: &str,
    pairs: &[(&str, String)],
    text: &str,
    tail: &str,
) -> Result<String> {
    let head = format!(
        "\"file\":{},\"sample\":{},\"x_unit\":\"m/z\",\"y_unit\":\"counts\",\"ms_level\":{level}",
        extract::jstr(&s.name),
        s.sample.map_or("null".to_string(), |i| i.to_string())
    );
    let ev = format!(
        "\"evidence\":{{\"route\":\"/api/spectrum\",\"query\":{},\"scan\":{resolved},\"answer\":{{{}}}{tail}}}",
        extract::jstr(&text_of(pairs)),
        scalar_members(text)?
    );
    Ok(extract::points_doc("spectrum/1", &head, pts, &ev))
}

pub fn get(args: Vec<String>) -> Outcome {
    let go = || -> std::result::Result<_, String> {
        let Some(p) = extract::parse(args, GET_VALUES, &["--sum"])? else {
            return Ok(Err(HELP_GET));
        };
        let file = one_file(&p, "spectrum get")?;
        let at = match (p.get("--rt"), p.get("--scan")) {
            (Some(_), Some(_)) => return Err("give --rt or --scan, not both".into()),
            (None, None) => return Err("spectrum get needs --rt or --scan".into()),
            (Some(r), None) => match p.window()? {
                Some((a, b)) if r.contains(',') => At::Window(a, b),
                Some((a, _)) => At::Nearest(a),
                None => unreachable!(),
            },
            (None, Some(n)) => At::Scan(n.parse().map_err(|_| "--scan needs a scan number")?),
        };
        let level = p.get("--ms-level").map(|_| p.level()).transpose()?;
        let blank = blank_of(&p, &file)?;
        if blank.is_some() && p.get("--id").is_some() {
            return Err("--minus is not for --id (the blank is read at the same level)".into());
        }
        let flags = (blank.is_some(), p.get("--id").is_some());
        let (peak, sum) = peak::modes(&p, &at, flags)?;
        let bin = bins::options(&p, blank.is_some(), sum.is_some() || peak.is_some())?;
        Ok(Ok((
            p.sample(&file)?,
            p.csv()?,
            p.out(),
            file,
            at,
            level,
            p.get("--id").map(String::from),
            blank,
            (sum, peak),
            bin,
        )))
    };
    let (sample, csv, out, file, at, level, chan, blank, modes, bin) = match go() {
        Ok(Ok(a)) => a,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    finish(
        extract::with_tmp("spectrum", |tmp| {
            let mut s = extract::open_sample(&file, sample, tmp)?;
            if let Some(b) = &bin {
                let (doc, notes) = bins::get(&mut s, &at, level, chan.as_deref(), csv, b)?;
                return Ok((doc, s.note.clone() + &notes));
            }
            let mut m = match &blank {
                Some((f, id, w)) => Some(minus::open(f, *id, *w, tmp)?),
                None => None,
            };
            let (doc, notes) = get_one(
                &mut s,
                &at,
                level,
                chan.as_deref(),
                csv,
                &mut m,
                (&modes.0, &modes.1),
            )?;
            Ok((doc, s.note.clone() + &notes))
        }),
        &out,
    )
}

/// The blank of `--minus FILE [--minus-sample ID] [--minus-rt A,B]`: its file (`FILE` itself with only `--minus-sample`), sample and window.
#[allow(clippy::type_complexity)]
fn blank_of(
    p: &Parsed,
    file: &Path,
) -> std::result::Result<Option<(PathBuf, Option<u32>, Option<(f64, f64)>)>, String> {
    let named = p.get("--minus").map(PathBuf::from);
    if named.is_none() && p.get("--minus-sample").is_none() {
        return match p.get("--minus-rt") {
            Some(_) => Err("--minus-rt needs --minus or --minus-sample".into()),
            None => Ok(None),
        };
    }
    let blank = named.unwrap_or_else(|| file.to_path_buf());
    let window = p
        .get("--minus-rt")
        .map(|v| {
            let num = |x: &str| {
                x.parse::<f64>()
                    .ok()
                    .filter(|x| x.is_finite())
                    .ok_or("--minus-rt needs minutes: A or A,B".to_string())
            };
            let (a, b) = v.split_once(',').unwrap_or((v, v));
            let (a, b) = (num(a)?, num(b)?);
            Ok::<_, String>((a.min(b), a.max(b)))
        })
        .transpose()?;
    let id = p.sample_of("--minus-sample", &blank)?;
    Ok(Some((blank, id, window)))
}

fn one_file(p: &Parsed, what: &str) -> std::result::Result<PathBuf, String> {
    match p.pos.as_slice() {
        [f] => Ok(PathBuf::from(f)),
        [] => Err(format!("{what} needs a file")),
        _ => Err(format!("{what} takes one file")),
    }
}

#[cfg(test)]
#[path = "spectrum_bins_tests.rs"]
mod bins_tests;
#[cfg(test)]
#[path = "spectrum_compare_tests.rs"]
mod compare_tests;
#[cfg(test)]
#[path = "spectrum_peak_tests.rs"]
mod peak_tests;
#[cfg(test)]
#[path = "spectrum_sum_tests.rs"]
mod sum_tests;
#[cfg(test)]
#[path = "spectrum_tests.rs"]
mod tests;
