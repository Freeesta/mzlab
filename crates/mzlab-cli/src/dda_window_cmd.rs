//! `mzlab-cli dda-window FILE --from MIN --to MIN [--set hr=0|1] [--out PATH]`: the MS2 started from the survey scans of a time window, grouped by
//! precursor, the JSON of `api/dda_window` (`mzlab_core::ddafinestra`, the «N MS²» list of the DDA trio of the page). The route reads the file as
//! its source, as the page and the app: a precursor intensity the reader lacks comes from the parent survey scan.

use std::path::PathBuf;

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::Limits;
use mzlab_core::mzidx::DEFAULT_BUDGET;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::sorgente::ReaderSource;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli dda-window: the MS2 started from the survey scans of a time window, one group per precursor, as JSON (the «N MS²» list of the page).

usage: mzlab-cli dda-window FILE --from MIN --to MIN [--set hr=1|0] [--out PATH]

  FILE        an mzML, or any single file the app reads (.raw, mzXML, ...)
  --from MIN  start of the window (minutes); --to MIN its end (equal = one scan)
  --set hr=0  read the file as low resolution (precursors grouped within 0.5 Da), as the page with high resolution off; default hr=1
  --out PATH  write there instead of the standard output";

struct Ask {
    file: PathBuf,
    out: Option<PathBuf>,
    from: f64,
    to: f64,
    hr: bool,
}

fn parse(args: Vec<String>) -> std::result::Result<Option<Ask>, String> {
    let (mut file, mut out, mut from, mut to, mut hr) = (None, None, None, None, true);
    let num = |v: Option<String>, o: &str| {
        v.and_then(|x| x.parse::<f64>().ok())
            .filter(|x| x.is_finite())
            .ok_or(format!("{o} needs minutes"))
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--from" => from = Some(num(it.next(), "--from")?),
            "--to" => to = Some(num(it.next(), "--to")?),
            "--set" => match it.next().as_deref() {
                Some("hr=1" | "hr=true") => hr = true,
                Some("hr=0" | "hr=false") => hr = false,
                _ => return Err("--set takes hr=1 or hr=0".into()),
            },
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ if file.is_none() => file = Some(PathBuf::from(a)),
            _ => return Err("dda-window takes one file".into()),
        }
    }
    Ok(Some(Ask {
        file: file.ok_or("dda-window needs a file")?,
        out,
        from: from.ok_or("dda-window needs --from")?,
        to: to.ok_or("dda-window needs --to")?,
        hr,
    }))
}

/// The query of the page for a window: the same text the site and the app send.
pub fn query(from: f64, to: f64, hr: bool) -> Query {
    Query::parse(&format!(
        "r0={from}&r1={to}{}",
        if hr { "" } else { "&hr=0" }
    ))
}

fn read(c: &Ask, tmp: &std::path::Path) -> Result<String> {
    let idx = tmp.join("0.mzidx");
    crate::build_index(&c.file, &idx, DEFAULT_BUDGET)?;
    let mut r = Routes::new(crate::open(&idx)?);
    let q = query(c.from, c.to, c.hr);
    let a = if crate::flow_pace::reads_as_mzml(&c.file) {
        r.mzml_route(
            "/api/dda_window",
            &q,
            &mut std::fs::File::open(&c.file)
                .map_err(|e| Error::new("err.flow.io").with("detail", e))?,
        )?
    } else {
        let mut l = mzlab_lettori::apri(crate::flow_set::file_set(&c.file)?, &Limits::desktop())?;
        r.mzml_route(
            "/api/dda_window",
            &q,
            &mut ReaderSource {
                l: l.as_mut(),
                c: 0,
            },
        )?
    };
    match a {
        Some(a) => Ok(a),
        None => r
            .handle("/api/dda_window", &q)?
            .ok_or_else(|| Error::new("err.route.unavailable")),
    }
}

pub fn run(args: Vec<String>) -> Outcome {
    let c = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-ddaw{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(|e| Error::new("err.file.index").with("detail", e))
        .and_then(|_| read(&c, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    match r {
        Err(e) => Outcome::fail(&e.to_string()),
        Ok(t) => crate::emit(t + "\n", &c.out),
    }
}

#[cfg(test)]
#[path = "dda_window_tests.rs"]
mod tests;
