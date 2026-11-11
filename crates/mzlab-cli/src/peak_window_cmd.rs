//! `mzlab-cli peak-window TRACE.csv --end MIN --other MIN (--at MIN | --step N) [--min MIN] [--max MIN]`: where a dragged end of an integration
//! window goes, from `mzlab_core::intwin::move_end_json`, the text the page gets from `QUALITA.windowEnd` (the same bytes).

use mzlab_core::intwin::move_end_json;

use crate::peaks_edges_cmd::trace;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli peak-window: where one end of an integration window goes when it is dragged along the trace, as JSON.

usage: mzlab-cli peak-window TRACE.csv|- --end MIN --other MIN (--at MIN | --step N) [--min MIN] [--max MIN]

  TRACE.csv    two columns, RT (min) and intensity; - = the standard input
  --end        the end being moved; --other the other end of the window
  --at         where the pointer is (the nearest point of the trace); --step N: N points from --end (negative = earlier), the keyboard
  --min/--max  the borders of the neighbouring integrations: the end may touch them, never cross them
  output       {\"t\":MIN,\"i\":INDEX}: a point of the trace, at least one point before (or after) --other";

fn num(v: Option<String>, flag: &str) -> Result<f64, String> {
    v.and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{flag} needs a number"))
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let (mut src, mut end, mut other, mut at, mut step, mut lo, mut hi) =
        (None, None, None, None, None, None, None);
    let res = (|| -> Result<(), String> {
        while let Some(a) = it.next() {
            match a.as_str() {
                "--end" => end = Some(num(it.next(), "--end")?),
                "--other" => other = Some(num(it.next(), "--other")?),
                "--at" => at = Some(num(it.next(), "--at")?),
                "--step" => {
                    step = Some(
                        it.next()
                            .and_then(|s| s.parse::<i64>().ok())
                            .ok_or("--step needs a whole number")?,
                    )
                }
                "--min" => lo = Some(num(it.next(), "--min")?),
                "--max" => hi = Some(num(it.next(), "--max")?),
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
    let (Some(src), Some(end), Some(other)) = (src, end, other) else {
        return Outcome::fail("peak-window needs a trace, --end and --other (see --help)");
    };
    if at.is_some() == step.is_some() {
        return Outcome::fail("peak-window needs one of --at or --step");
    }
    match trace(&src) {
        Err(e) => Outcome::fail(&e),
        Ok((x, _)) => match move_end_json(&x, end, other, at, step.unwrap_or(0), lo, hi) {
            Some(t) => Outcome::ok(format!("{t}\n").into_bytes()),
            None => Outcome::fail("peak-window: no point of the trace is free for this end"),
        },
    }
}

#[cfg(test)]
#[path = "peak_window_cmd_tests.rs"]
mod tests;
