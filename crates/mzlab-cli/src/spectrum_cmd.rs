//! `mzlab-cli spectrum get|list`: one mass spectrum (at a retention time, a window or a scan) as `spectrum/1` or an `mz,intensity` CSV, and the scan list as `scanlist/1`
//! or CSV. Both are the answers of the page's `api/spectrum`, `api/nearest_scan` and `api/scanlist`; the number tokens are the route's own.

use std::path::PathBuf;

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, write_value, Out, Value};
use mzlab_core::routes::Query;

use crate::extract::{self, Parsed, Sample};
use crate::Outcome;

pub const HELP_GET: &str = "mzlab-cli spectrum get: one mass spectrum as JSON (spectrum/1) or CSV (mz,intensity), the answer of the page's api/spectrum.

usage: mzlab-cli spectrum get FILE (--rt MIN | --rt A,B | --scan NO) [--ms-level N] [--id CHANNEL] [--sample ID] [--format json|csv] [--out PATH]

  --rt MIN    the scan nearest to MIN (minutes) at the level; the evidence names it
  --rt A,B    the mean spectrum of the scans between A and B (minutes)
  --scan NO   the scan with that number (`spectrum list` prints it as scan_no); its level and precursor are used; no such scan: exit 5
  --ms-level N the level (default 1, or the level of --scan)
  --id CHANNEL only the scans of an ms:<key> or dd:<level> channel `file channels` lists
  --sample ID the sample of a file with several (.wiff); default the first
  --format F  json (default) or csv with the header mz,intensity
  --out PATH  write there instead of the standard output";

pub const HELP_LIST: &str = "mzlab-cli spectrum list: the scans of a file as JSON (scanlist/1) or CSV, the answer of the page's api/scanlist.

usage: mzlab-cli spectrum list FILE [--ms-level N] [--id CHANNEL] [--sample ID] [--format json|csv] [--out PATH]

  --ms-level N one level; default every level, in time order
  --id CHANNEL only the scans of an ms:<key> or dd:<level> channel
  --sample ID the sample of a file with several (.wiff); default the first
  --format F  json (default) or csv: scan_id,scan_no,ms_level,rt_min,tic,precursor_mz,base_peak_mz,base_peak_intensity
  --out PATH  write there instead of the standard output";

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

fn query(pairs: &[(&str, String)]) -> Query {
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
enum At {
    Scan(i64),
    Nearest(f64),
    Window(f64, f64),
}

/// The index of the scan `at` names, `None` for a window.
fn scan_index(s: &mut Sample, at: &At, level: u8, chan: Option<&str>) -> Result<Option<usize>> {
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
                q.push(("canale", c.into()));
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

fn get_one(
    s: &mut Sample,
    at: &At,
    level: Option<u8>,
    chan: Option<&str>,
    csv: bool,
) -> Result<String> {
    let mut level = level.unwrap_or(1);
    let mut resolved = "null".to_string();
    let mut pairs: Vec<(&str, String)> = Vec::new();
    let sid = scan_index(s, at, level, chan)?;
    let (lo, hi) = match (sid, at) {
        (Some(i), _) => {
            let sc = &s.routes.idx.scans[i];
            // a scan given by number states its own level; the level of --rt is the one asked for
            if matches!(at, At::Scan(_)) {
                level = sc.level;
            }
            if let (true, Some(p)) = (sc.level > 1, sc.precursor) {
                pairs.push(("precursor", p.to_string()));
            }
            resolved = format!(
                "{{\"scan_id\":{i},\"scan_no\":{},\"rt_min\":{},\"ms_level\":{}}}",
                sc.number(i),
                extract::jnum(sc.rt),
                sc.level
            );
            (sc.rt, sc.rt)
        }
        (None, At::Window(a, b)) => (*a, *b),
        _ => unreachable!(),
    };
    pairs.splice(
        0..0,
        [
            ("rt0", lo.to_string()),
            ("rt1", hi.to_string()),
            ("level", level.to_string()),
        ],
    );
    if let Some(c) = chan {
        pairs.push(("canale", c.into()));
    }
    let text = s
        .routes
        .handle("/api/spectrum", &query(&pairs))?
        .ok_or_else(|| Error::new("err.scan.none"))?;
    let bad = || Error::new("error.badIndex");
    let x = extract::tokens(&text, "mz").ok_or_else(bad)?;
    let y = extract::tokens(&text, "y").ok_or_else(bad)?;
    let pts = extract::pairs(&x, &y, None);
    if csv {
        return Ok(extract::points_csv("mz", "intensity", &pts));
    }
    // the rest of the route's answer (mode, scans averaged, first and last scan, merge) is the evidence
    let ans = scalar_members(&text)?;
    let head = format!(
        "\"file\":{},\"sample\":{},\"x_unit\":\"m/z\",\"y_unit\":\"counts\",\"ms_level\":{level}",
        extract::jstr(&s.name),
        s.sample.map_or("null".to_string(), |i| i.to_string())
    );
    let ev = format!(
        "\"evidence\":{{\"route\":\"/api/spectrum\",\"query\":{},\"scan\":{resolved},\"answer\":{{{}}}}}",
        extract::jstr(&text_of(&pairs)),
        ans
    );
    Ok(extract::points_doc("spectrum/1", &head, &pts, &ev))
}

pub fn get(args: Vec<String>) -> Outcome {
    let go = || -> std::result::Result<_, String> {
        let Some(p) = extract::parse(
            args,
            &[
                "--rt",
                "--scan",
                "--ms-level",
                "--id",
                "--sample",
                "--format",
                "--out",
            ],
            &[],
        )?
        else {
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
        Ok(Ok((
            p.sample(&file)?,
            p.csv()?,
            p.out(),
            file,
            at,
            level,
            p.get("--id").map(String::from),
        )))
    };
    let (sample, csv, out, file, at, level, chan) = match go() {
        Ok(Ok(a)) => a,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    finish(
        extract::with_tmp("spectrum", |tmp| {
            let mut s = extract::open_sample(&file, sample, tmp)?;
            Ok((
                get_one(&mut s, &at, level, chan.as_deref(), csv)?,
                s.note.clone(),
            ))
        }),
        &out,
    )
}

fn one_file(p: &Parsed, what: &str) -> std::result::Result<PathBuf, String> {
    match p.pos.as_slice() {
        [f] => Ok(PathBuf::from(f)),
        [] => Err(format!("{what} needs a file")),
        _ => Err(format!("{what} takes one file")),
    }
}

const COLS: [(&str, &str); 7] = [
    ("sid", "scan_id"),
    ("no", "scan_no"),
    ("rt", "rt_min"),
    ("tic", "tic"),
    ("prec", "precursor_mz"),
    ("bpmz", "base_peak_mz"),
    ("bpint", "base_peak_intensity"),
];

/// The rows of `api/scanlist` for the levels asked, in time order: `(rt, [level, sid, no, rt, tic, prec, bpmz, bpint] tokens)`.
fn list_rows(s: &mut Sample, level: Option<u8>, chan: Option<&str>) -> Result<Vec<Vec<String>>> {
    let mut levels: Vec<u8> = match level {
        Some(l) => vec![l],
        None => s.routes.idx.sections.iter().map(|x| x.level).collect(),
    };
    levels.sort_unstable();
    levels.dedup();
    let mut rows: Vec<Vec<String>> = Vec::new();
    for l in levels {
        let mut q = vec![("level", l.to_string())];
        if let Some(c) = chan {
            q.push(("canale", c.into()));
        }
        let text = s
            .routes
            .handle("/api/scanlist", &query(&q))?
            .ok_or_else(|| Error::new("err.scan.none"))?;
        let cols: Vec<Vec<&str>> = COLS
            .iter()
            .map(|(k, _)| extract::tokens(&text, k).unwrap_or_default())
            .collect();
        for i in 0..cols[0].len() {
            let mut r = vec![l.to_string()];
            r.extend(
                cols.iter()
                    .map(|c| c.get(i).copied().unwrap_or("null").to_string()),
            );
            rows.push(r);
        }
    }
    // rt is column 3 (after the level, scan id and scan number)
    rows.sort_by(|a, b| {
        a[3].parse::<f64>()
            .unwrap_or(0.0)
            .total_cmp(&b[3].parse::<f64>().unwrap_or(0.0))
    });
    Ok(rows)
}

fn list_doc(s: &mut Sample, level: Option<u8>, chan: Option<&str>, csv: bool) -> Result<String> {
    let rows = list_rows(s, level, chan)?;
    let cell = |t: &str| {
        if t == "null" {
            String::new()
        } else {
            t.to_string()
        }
    };
    if csv {
        let mut o =
            "scan_id,scan_no,ms_level,rt_min,tic,precursor_mz,base_peak_mz,base_peak_intensity\n"
                .to_string();
        for r in &rows {
            let v = [&r[1], &r[2], &r[0], &r[3], &r[4], &r[5], &r[6], &r[7]];
            o += &v.iter().map(|t| cell(t)).collect::<Vec<_>>().join(",");
            o += "\n";
        }
        return Ok(o);
    }
    let mut o = Out::default();
    o.raw("{\"schema\":\"scanlist/1\",\"file\":").str(&s.name);
    o.raw(",\"sample\":")
        .raw(&s.sample.map_or("null".to_string(), |i| i.to_string()));
    o.raw(",\"x_unit\":\"min\",\"n_scans\":")
        .int(rows.len() as i64)
        .raw(",\"scans\":[");
    for (i, r) in rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        o.raw(&format!(
            "{{\"scan_id\":{},\"scan_no\":{},\"ms_level\":{},\"rt_min\":{},\"tic\":{},\"precursor_mz\":{},\"base_peak_mz\":{},\"base_peak_intensity\":{}}}",
            r[1], r[2], r[0], r[3], r[4], r[5], r[6], r[7]
        ));
    }
    o.raw("],\"evidence\":{\"route\":\"/api/scanlist\",\"levels\":");
    let levels: Vec<String> = {
        let mut l: Vec<String> = rows.iter().map(|r| r[0].clone()).collect();
        l.dedup();
        l
    };
    o.raw(&format!("[{}]", levels.join(",")));
    o.raw("}}");
    Ok(o.0)
}

pub fn list(args: Vec<String>) -> Outcome {
    let go = || -> std::result::Result<_, String> {
        let Some(p) = extract::parse(
            args,
            &["--ms-level", "--id", "--sample", "--format", "--out"],
            &[],
        )?
        else {
            return Ok(Err(HELP_LIST));
        };
        let file = one_file(&p, "spectrum list")?;
        let level = p.get("--ms-level").map(|_| p.level()).transpose()?;
        Ok(Ok((
            p.sample(&file)?,
            p.csv()?,
            p.out(),
            file,
            level,
            p.get("--id").map(String::from),
        )))
    };
    let (sample, csv, out, file, level, chan) = match go() {
        Ok(Ok(a)) => a,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    finish(
        extract::with_tmp("scanlist", |tmp| {
            let mut s = extract::open_sample(&file, sample, tmp)?;
            Ok((
                list_doc(&mut s, level, chan.as_deref(), csv)?,
                s.note.clone(),
            ))
        }),
        &out,
    )
}

#[cfg(test)]
#[path = "spectrum_tests.rs"]
mod tests;
