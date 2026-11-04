//! `mzlab-cli peak-auto TRACE.csv`: every peak of a high-resolution trace with its baseline, from `mzlab_core::peakq_auto::auto_integrate`, the
//! function behind the one-click integration of the page (`QUALITA.autoIntegrate`). The `--set` names are the snake_case of the page settings (`PK`).

use mzlab_core::peakq_auto::{auto_integrate, AutoParams};

use crate::peaks_edges_cmd::trace;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli peak-auto: every peak of a high-resolution trace with its baseline, as JSON.

usage: mzlab-cli peak-auto TRACE.csv|- [--from MIN] [--to MIN] [--set NAME=VALUE...]

  TRACE.csv    two columns, RT (min) and intensity; - = the standard input
  --from/--to  the stretch whose apexes count (default: the whole trace)
  --set        the page dialog: hr_smooth (points, odd), hr_sn; engine only: hr_prom (% of the tallest), hr_scans, hr_half (points, 0 = auto), hr_slope (%)
  output       {\"peaks\":[{\"t0\":MIN,\"t1\":MIN,\"b0\":BASELINE,\"b1\":BASELINE,\"area\":COUNTS_MIN}...]}";

fn set(p: &mut AutoParams, name: &str, v: &str) -> Result<(), String> {
    let n: f64 = v.parse().map_err(|_| format!("{name} needs a number"))?;
    if n < 0.0 {
        return Err(format!("{name} must not be negative"));
    }
    match name {
        "hr_smooth" => p.pick.smooth = n as usize,
        "hr_sn" => p.pick.snr = n,
        "hr_prom" => p.pick.rel_prominence = n / 100.0,
        "hr_scans" => p.pick.min_scans = n as usize,
        "hr_half" => p.half_window = n as usize,
        "hr_slope" => p.slope = n / 100.0,
        _ => return Err(format!("unknown parameter {name}")),
    }
    Ok(())
}

fn num(v: Option<String>, flag: &str) -> Result<f64, String> {
    v.and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| format!("{flag} needs a number"))
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let (mut src, mut from, mut to, mut p) = (None, None, None, AutoParams::default());
    let res = (|| -> Result<(), String> {
        while let Some(a) = it.next() {
            match a.as_str() {
                "--from" => from = Some(num(it.next(), "--from")?),
                "--to" => to = Some(num(it.next(), "--to")?),
                "--set" => {
                    let kv = it.next().ok_or("--set needs NAME=VALUE")?;
                    let (k, v) = kv.split_once('=').ok_or("--set needs NAME=VALUE")?;
                    set(&mut p, k, v)?;
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
    let Some(src) = src else {
        return Outcome::fail("peak-auto needs a trace (see --help)");
    };
    match trace(&src) {
        Err(e) => Outcome::fail(&e),
        Ok((x, y)) => {
            let (x0, x1) = (from.unwrap_or(x[0]), to.unwrap_or(x[x.len() - 1]));
            let rows: Vec<String> = auto_integrate(&x, &y, x0, x1, p)
                .iter()
                .map(|k| {
                    format!(
                        "{{\"t0\":{},\"t1\":{},\"b0\":{},\"b1\":{},\"area\":{}}}",
                        x[k.lo], x[k.hi], k.base_lo, k.base_hi, k.area
                    )
                })
                .collect();
            Outcome::ok(format!("{{\"peaks\":[{}]}}\n", rows.join(",")).into_bytes())
        }
    }
}

#[cfg(test)]
#[path = "peaks_auto_cmd_tests.rs"]
mod tests;
