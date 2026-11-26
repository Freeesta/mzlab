//! `spectrum compare A B`: the cosine of two MS2 spectra binned at nominal mass (`spectrum-compare/1`). The CLI selects the scans of each side and asks the
//! engine's `/api/spectrum` with `rebin=nominal`; the cosine, the matched ions and the warnings are `mzlab_lib::speccmp`, the function the site and the app call.

use std::path::PathBuf;

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use super::{asked, query, text_of, At};
use crate::extract::{self, Parsed, Sample};
use crate::Outcome;

pub const HELP: &str = "mzlab-cli spectrum compare: the cosine of two MS2 spectra at nominal mass, with the matched ions (spectrum-compare/1).

usage: mzlab-cli spectrum compare A [B] (--a-rt MIN|A,B | --a-scan NO) (--b-rt MIN|A,B | --b-scan NO) [--a-precursor MZ] [--b-precursor MZ] [--a-sample ID] [--b-sample ID] [--ms-level N] [--set tol=DA,ce_tol=EV] [--out PATH]

  A [B]        the files of the two spectra (B defaults to A: two selections of one file); unit-resolution data, each spectrum cut into nominal bins
  --a-rt       the scan nearest to MIN (minutes), or the mean of the scans between A and B; --a-scan: the scan with that number
  --a-precursor MZ  only the scans of that precursor (within 0.6 Da); a window or a time that holds several precursors needs it (exit 5)
  --ms-level N the level of both (default 2)
  --set        tol (Da between two bin centres, default 0 = the same bin) and ce_tol (eV, default 0.5: the collision energies that differ by more give
               a warning); the parameters and the defaults used are in the document
  --out PATH   write there instead of the standard output

The cosine is the engine's (square-root intensities). The document lists the matched ions with both intensities, the ions of each side that the other
lacks, the share of each spectrum's intensity the match explains, the collision energy and precursor of each and the warnings.";

/// One side as the flags give it.
struct Pick {
    file: PathBuf,
    sample: Option<u32>,
    at: At,
    precursor: Option<f64>,
}

fn pick(p: &Parsed, side: &str, files: &[String]) -> std::result::Result<Pick, String> {
    let file = PathBuf::from(
        files
            .get(usize::from(side == "b"))
            .or(files.first())
            .ok_or("spectrum compare needs a file")?,
    );
    let flag = |k: &str| format!("--{side}-{k}");
    let at = match (p.get(&flag("rt")), p.get(&flag("scan"))) {
        (Some(_), Some(_)) => {
            return Err(format!("give {} or {}, not both", flag("rt"), flag("scan")))
        }
        (None, None) => return Err(format!("give {} or {}", flag("rt"), flag("scan"))),
        (None, Some(n)) => At::Scan(
            n.parse()
                .map_err(|_| format!("{} needs a scan number", flag("scan")))?,
        ),
        (Some(r), None) => {
            let num = |x: &str| {
                x.parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .ok_or(format!("{} needs minutes: A or A,B", flag("rt")))
            };
            match r.split_once(',') {
                Some((a, b)) => {
                    let (a, b) = (num(a)?, num(b)?);
                    At::Window(a.min(b), a.max(b))
                }
                None => At::Nearest(num(r)?),
            }
        }
    };
    let precursor = p
        .get(&flag("precursor"))
        .map(|v| {
            v.parse::<f64>()
                .ok()
                .filter(|x| x.is_finite() && *x > 0.0)
                .ok_or(format!("{} needs an m/z", flag("precursor")))
        })
        .transpose()?;
    let sample = p.sample_of(&flag("sample"), &file)?;
    Ok(Pick {
        file,
        sample,
        at,
        precursor,
    })
}

/// `--set tol=0.5,ce_tol=1` as the JSON object the engine takes; the engine refuses a name it does not know.
fn set_json(p: &Parsed) -> std::result::Result<String, String> {
    let mut items: Vec<(String, f64)> = Vec::new();
    for v in p.all("--set") {
        for item in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (k, x) = item
                .split_once('=')
                .ok_or(format!("--set needs NAME=VALUE: {item}"))?;
            let x = x
                .parse::<f64>()
                .ok()
                .filter(|x| x.is_finite())
                .ok_or(format!("--set {k} needs a number"))?;
            items.retain(|(n, _)| n != k);
            items.push((k.to_string(), x));
        }
    }
    let mut o = Out::default();
    o.raw("{");
    for (i, (k, x)) in items.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").num(*x);
    }
    Ok(o.raw("}").0.clone())
}

/// The scan nearest to `rt` among those of precursor `p`, by its number.
fn nearest_of(s: &mut Sample, rt: f64, p: f64, level: u8) -> Result<At> {
    let q = query(&[
        ("rt", rt.to_string()),
        ("level", level.to_string()),
        ("precursor", p.to_string()),
    ]);
    let none = || {
        Error::new("err.scan.unknown").with(
            "id",
            format!("nearest to {rt} min at MS level {level}, precursor {p}"),
        )
    };
    let t = s.routes.handle("/api/nearest_scan", &q)?.ok_or_else(none)?;
    let sid = parse(t.as_bytes())?
        .get("sid")
        .and_then(Value::f64)
        .ok_or_else(none)? as usize;
    Ok(At::Scan(s.routes.idx.scans[sid].number(sid)))
}

/// The distinct precursors and collision energies of the level-`level` scans in `lo..=hi` (of precursor `want`, when given).
fn scans_of(s: &Sample, lo: f64, hi: f64, level: u8, want: Option<f64>) -> (Vec<f64>, Vec<f64>) {
    let mut precursors: Vec<f64> = Vec::new();
    let mut ces: Vec<f64> = Vec::new();
    for c in s.routes.idx.scans.iter().filter(|c| {
        c.level == level
            && c.rt >= lo
            && c.rt <= hi
            && want.is_none_or(|p| (c.prec(-100.0) - p).abs() <= 0.6)
    }) {
        let pr = c.prec(-100.0);
        if pr > 0.0 && !precursors.iter().any(|x| (x - pr).abs() <= 0.6) {
            precursors.push(pr);
        }
        if let Some(e) = c
            .collision_energy
            .filter(|e| !ces.iter().any(|x| (x - e).abs() < 1e-9))
        {
            ces.push(e);
        }
    }
    (precursors, ces)
}

/// What one side selected: the scans' precursor and collision energies, and the route's bins.
fn side(s: &mut Sample, pk: &Pick, level: u8) -> Result<String> {
    let mut at = match pk.at {
        At::Scan(n) => At::Scan(n),
        At::Window(a, b) => At::Window(a, b),
        At::Nearest(rt) => At::Nearest(rt),
    };
    // a time with a precursor: the nearest scan of that precursor, taken by its number so the route answers that scan alone
    if let (At::Nearest(rt), Some(p)) = (&at, pk.precursor) {
        at = nearest_of(s, *rt, p, level)?;
    }
    let a = asked(s, &at, Some(level), None)?;
    let mut pairs = a.pairs;
    // a user precursor narrows a window; for a scan number the scan's own precursor stays and a different one is refused
    let given = pk.precursor;
    match (pairs.iter().position(|(k, _)| *k == "precursor"), given) {
        (None, Some(p)) => pairs.push(("precursor", p.to_string())),
        (Some(i), Some(p)) if (pairs[i].1.parse::<f64>().unwrap_or(0.0) - p).abs() > 0.6 => {
            return Err(Error::new("err.spectrum.compare.precursor")
                .with("given", p.to_string())
                .with("found", pairs[i].1.clone()));
        }
        _ => {}
    }
    pairs.push(("rebin", "nominal".into()));
    let text = s
        .routes
        .handle("/api/spectrum", &query(&pairs))?
        .ok_or_else(|| Error::new("err.scan.none"))?;
    let (lo, hi) = a.window;
    let want: Option<f64> = pairs
        .iter()
        .find(|(k, _)| *k == "precursor")
        .and_then(|(_, v)| v.parse().ok());
    let (precursors, ces) = scans_of(s, lo, hi, level, want);
    if want.is_none() && precursors.len() > 1 {
        let list = precursors
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(Error::new("err.spectrum.compare.precursors").with("precursors", list));
    }
    let mut o = Out::default();
    o.raw("{\"name\":")
        .str(&s.name)
        .raw(",\"query\":")
        .str(&text_of(&pairs))
        .raw(",\"precursor\":");
    o.opt(want.or(precursors.first().copied()))
        .raw(",\"ce_values\":")
        .floats(&ces);
    o.raw(",\"spectrum\":").raw(&text).raw("}");
    Ok(o.0)
}

pub fn compare(args: Vec<String>) -> Outcome {
    let go = || -> std::result::Result<_, String> {
        let valued = [
            "--a-rt",
            "--a-scan",
            "--a-precursor",
            "--a-sample",
            "--b-rt",
            "--b-scan",
            "--b-precursor",
            "--b-sample",
            "--ms-level",
            "--set",
            "--out",
        ];
        let Some(p) = extract::parse(args, &valued, &[])? else {
            return Ok(Err(HELP));
        };
        if p.pos.is_empty() || p.pos.len() > 2 {
            return Err("spectrum compare takes one or two files".into());
        }
        let level = match p.get("--ms-level") {
            None => 2,
            Some(_) => p.level()?,
        };
        Ok(Ok((
            pick(&p, "a", &p.pos)?,
            pick(&p, "b", &p.pos)?,
            level,
            set_json(&p)?,
            p.out(),
        )))
    };
    let (a, b, level, set, out) = match go() {
        Ok(Ok(x)) => x,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let run = |tmp: &std::path::Path| -> Result<(String, String)> {
        let mut sides = Vec::new();
        for pk in [&a, &b] {
            let mut s = extract::open_sample(&pk.file, pk.sample, tmp)?;
            sides.push(side(&mut s, pk, level)?);
        }
        let text = mzlab_lib::speccmp::compare_text(
            &set,
            &format!("{{\"a\":{},\"b\":{}}}", sides[0], sides[1]),
        )?;
        let note = notes(&text);
        Ok((text, note))
    };
    match extract::with_tmp("spectrum-compare", run) {
        Err(e) => crate::sample_select::fail(&e),
        Ok((t, note)) => {
            let mut o = crate::emit(t, &out);
            if o.code == 0 {
                o.stderr = note;
            }
            o
        }
    }
}

/// A number, a string or a list of numbers (joined by slashes) as the words of a message.
fn text_of_value(x: &Value) -> String {
    match x {
        Value::Num(n) => n.to_string(),
        Value::Str(t) => t.clone(),
        Value::Arr(a) => a.iter().map(text_of_value).collect::<Vec<_>>().join("/"),
        _ => String::new(),
    }
}

/// The stderr lines: the cosine with its evidence, and each warning as the catalogue says it.
fn notes(doc: &str) -> String {
    let Ok(v) = parse(doc.as_bytes()) else {
        return String::new();
    };
    let num = |x: &Value, k: &str| x.get(k).and_then(Value::f64).unwrap_or(0.0);
    let mut s = format!(
        "note: cosine {} over {} matched bins (tolerance {} Da)\n",
        num(&v, "cosine"),
        num(&v, "n_matched"),
        v.get("params").map_or(0.0, |p| num(p, "tol"))
    );
    for w in v.get("warnings").map_or(&[][..], Value::arr) {
        let key = w.get("key").and_then(Value::str).unwrap_or("");
        let par = w
            .entries()
            .iter()
            .filter(|(k, _)| k != "key")
            .map(|(k, x)| (k.clone(), text_of_value(x)))
            .collect();
        s += &format!("warning: {}\n", crate::error_text::notice_text(key, par));
    }
    s
}
