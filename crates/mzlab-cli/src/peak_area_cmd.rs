//! `mzlab-cli peak-area TRACE.csv --from MIN --to MIN [--set baseline=zero|line]`: the area of the Data view between two boundaries measured from
//! zero (the default) or from a line the user sets, from `mzlab_core::baseline::peak_area_json`, the text the page gets from `QUALITA.peakArea` (the same bytes).

use mzlab_core::baseline::{peak_area_json, Baseline};

use crate::peaks_edges_cmd::trace;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli peak-area: the area of a peak between two boundaries, from zero or from a line you set, as JSON.

usage: mzlab-cli peak-area TRACE.csv|- --from MIN --to MIN [--set baseline=zero|line] [--set NAME=VALUE...]

  TRACE.csv    two columns, RT (min) and intensity; - = the standard input
  --from/--to  the boundaries (each taken to the nearest point of the trace)
  --set        baseline: zero (y = 0, the default: any background under the peak is part of the area) or line; a line: base_y0 (counts), base_t0, base_y1, base_t1 (min, counts;
               base_y1 absent = base_y0, a level). edges, the line between the boundaries, is the page's own (picchi.js)
  output       {\"baseline\":MODE,\"area\":COUNTS_S,\"below\":COUNTS_S,\"height\":COUNTS,\"rt\":MIN}: area signed, below = the part under the line";

/// The `--set` names of a baseline (`Baseline::from_params`) from a list of `NAME=VALUE`.
pub(crate) fn baseline_of(sets: &[(String, String)]) -> Result<Baseline, String> {
    for (k, v) in sets {
        if k.starts_with("base_") && v.parse::<f64>().is_err() {
            return Err(format!("{k} needs a number"));
        }
    }
    let get = |k: &str| {
        sets.iter()
            .rev()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
    };
    Baseline::from_params(get("baseline"), |k| get(k).and_then(|v| v.parse().ok()))
}

/// Is `name` one of the `--set` names of a baseline.
pub(crate) fn is_base(name: &str) -> bool {
    matches!(
        name,
        "baseline" | "base_t0" | "base_y0" | "base_t1" | "base_y1"
    )
}

fn num(v: Option<String>, flag: &str) -> Result<f64, String> {
    v.and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| format!("{flag} needs a number"))
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let (mut src, mut from, mut to, mut sets) = (None, None, None, Vec::new());
    let res = (|| -> Result<(), String> {
        while let Some(a) = it.next() {
            match a.as_str() {
                "--from" => from = Some(num(it.next(), "--from")?),
                "--to" => to = Some(num(it.next(), "--to")?),
                "--set" => {
                    let kv = it.next().ok_or("--set needs NAME=VALUE")?;
                    let (k, v) = kv.split_once('=').ok_or("--set needs NAME=VALUE")?;
                    if !is_base(k) {
                        return Err(format!("unknown parameter {k}"));
                    }
                    sets.push((k.to_string(), v.to_string()));
                }
                "--help" | "-h" => return Err(String::new()),
                o if o.starts_with("--") => return Err(format!("unknown option {o}")),
                o => src = Some(o.to_string()),
            }
        }
        Ok(())
    })();
    match res {
        Err(e) if e.is_empty() => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
        Ok(()) => {}
    }
    let (Some(src), Some(from), Some(to)) = (src, from, to) else {
        return Outcome::fail("peak-area needs a trace, --from and --to (see --help)");
    };
    let base = match baseline_of(&sets) {
        Ok(Baseline::Edges) => {
            return Outcome::fail(
                "peak-area measures from zero or a line: --set baseline=zero|line",
            )
        }
        Ok(b) => b,
        Err(e) => return Outcome::fail(&e),
    };
    match trace(&src) {
        Err(e) => Outcome::fail(&e),
        Ok((x, y)) => match peak_area_json(&x, &y, from, to, base) {
            Some(t) => Outcome::ok(format!("{t}\n").into_bytes()),
            None => Outcome::fail("peak-area: empty trace"),
        },
    }
}

#[cfg(test)]
#[path = "peak_area_cmd_tests.rs"]
mod tests;
