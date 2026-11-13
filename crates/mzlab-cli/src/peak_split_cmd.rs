//! `mzlab-cli peak-split FILE... --set tr=J --set a=MIN --set b=MIN [--set NAME=VALUE...]`: one integration drawn by hand on a transition / SIM
//! trace and its valleys (`mzlab_core::manint`). Without `split` it prints where the window splits at `depth` (the bytes of the page's
//! `api/manvalley`); with `split` it prints the table of the window cut there, a row per part (the bytes of `api/manint` for that window).

use std::path::PathBuf;

use mzlab_core::json::Out;
use mzlab_core::manint::{table_text, valleys_text};
use mzlab_core::valley::DEPTH;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli peak-split: one integration drawn by hand on a transition / SIM trace, split at its valleys.

usage: mzlab-cli peak-split FILE... --set tr=J --set a=MIN --set b=MIN [--set NAME=VALUE...] [--out PATH]

  FILE        an mzML, or a vendor file the app reads, in the order of the page (as manual-table)
  --set file=I        the sample (from 0, default 0) and --set tr=J its transition in the list of the file
  --set a=MIN --set b=MIN   the edges of the window
  --set baseline=zero|edges|line   default zero; line needs --set line=t0,y0,t1,y1
  --set depth=X       0..1, default 0.1: a valley splits when the smaller of its two peaks rises above it by X times the highest point of the
                      window (trace minus baseline); a shoulder, without a minimum, is never split
  --set split=auto|MIN[,MIN...]   cut the window there (auto = at the valleys found at depth) and print its table
  --set label=NAME    the name of the window (the last column of the table)
  --out PATH  write there instead of the standard output

Without split: {\"splits\":[MIN,...]}, the times of the valleys. With split: the CSV of manual-table for this window, a row per part between drop
lines on the window's baseline (overlap = drop, pv = Hp/Hv: the height of the part over the higher valley next to it; Ph. Eur. 2.2.46 takes the
value of the smaller peak). The areas of the parts add up to the area of the whole window.";

#[derive(Default)]
struct Request {
    files: Vec<PathBuf>,
    window: Out,
    depth: Option<f64>,
    split: Option<String>,
    out: Option<PathBuf>,
}

fn number(name: &str, v: &str) -> Result<f64, String> {
    v.trim()
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite())
        .ok_or_else(|| format!("{name} needs a number, not {v}"))
}

fn parse(args: Vec<String>) -> Result<Option<Request>, String> {
    let mut c = Request::default();
    let (mut file, mut tr, mut a, mut b, mut base, mut line, mut label) =
        (0.0, None, None, None, None, None, None);
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => c.out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--set" => {
                let v = it.next().ok_or("--set needs NAME=VALUE")?;
                match v.split_once('=') {
                    Some(("file", x)) => file = number("file", x)?,
                    Some(("tr", x)) => tr = Some(number("tr", x)?),
                    Some(("a", x)) => a = Some(number("a", x)?),
                    Some(("b", x)) => b = Some(number("b", x)?),
                    Some(("baseline", x @ ("zero" | "edges" | "line"))) => {
                        base = Some(x.to_string())
                    }
                    Some(("baseline", x)) => {
                        return Err(format!("baseline is zero, edges or line, not {x}"))
                    }
                    Some(("line", x)) => {
                        let v = x
                            .split(',')
                            .map(|n| number("line", n))
                            .collect::<Result<Vec<_>, _>>()?;
                        if v.len() != 4 {
                            return Err("line needs four numbers t0,y0,t1,y1".into());
                        }
                        line = Some(v);
                    }
                    Some(("depth", x)) => match number("depth", x)? {
                        d if (0.0..=1.0).contains(&d) => c.depth = Some(d),
                        _ => return Err(format!("depth is between 0 and 1, not {x}")),
                    },
                    Some(("split", x)) if !x.is_empty() => c.split = Some(x.to_string()),
                    Some(("label", x)) => label = Some(x.to_string()),
                    _ => return Err(format!("unknown setting {v}")),
                }
            }
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ => c.files.push(PathBuf::from(arg)),
        }
    }
    if c.files.is_empty() {
        return Err("peak-split needs at least one file".into());
    }
    let (Some(tr), Some(a), Some(b)) = (tr, a, b) else {
        return Err("peak-split needs --set tr=J --set a=MIN --set b=MIN".into());
    };
    let w = &mut c.window;
    w.raw("{\"file\":")
        .num(file)
        .raw(",\"tr\":")
        .num(tr)
        .raw(",\"a\":")
        .num(a)
        .raw(",\"b\":")
        .num(b);
    if let Some(n) = base {
        w.raw(",\"baseline\":").str(&n);
    }
    if let Some(v) = line {
        w.raw(",\"line\":[")
            .raw(
                &v.iter()
                    .map(|x| format!("{x}"))
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .raw("]");
    }
    if let Some(n) = label {
        w.raw(",\"label\":").str(&n);
    }
    Ok(Some(c))
}

/// The times of `split`: the valleys found at the depth asked for `auto`, else the numbers given.
fn cuts(c: &Request, files: &str, window: &str) -> Result<String, String> {
    let given = c.split.as_deref().unwrap_or_default();
    if given == "auto" {
        let v = valleys_text(files, window, c.depth.unwrap_or(DEPTH))
            .map_err(|e| crate::error_text::human_error(&e))?;
        return Ok(v
            .trim_start_matches("{\"splits\":[")
            .trim_end_matches("]}")
            .to_string());
    }
    let v = given
        .split(',')
        .map(|n| number("split", n))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v.iter()
        .map(|x| format!("{x}"))
        .collect::<Vec<_>>()
        .join(","))
}

fn answer(c: &Request, warn: &mut String) -> Result<String, String> {
    let human = |e: mzlab_core::error::Error| crate::error_text::human_error(&e);
    let files = crate::manual_table_cmd::files_json(&c.files, warn).map_err(human)?;
    let mut window = c.window.0.clone();
    window.push('}');
    if c.split.is_none() {
        return valleys_text(&files, &window, c.depth.unwrap_or(DEPTH)).map_err(human);
    }
    let at = cuts(c, &files, &window)?;
    window.pop();
    table_text(&files, &format!("[{window},\"splits\":[{at}]}}]"), false).map_err(human)
}

pub fn run(args: Vec<String>) -> Outcome {
    let c = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let mut warn = String::new();
    match answer(&c, &mut warn) {
        Err(e) => Outcome::fail(&format!("{warn}{e}")),
        Ok(t) => {
            let mut o = crate::emit(t, &c.out);
            o.stderr.insert_str(0, &warn);
            o
        }
    }
}

#[cfg(test)]
#[path = "peak_split_cmd_tests.rs"]
mod tests;
