//! `mzlab-cli peak-auto TRACE.csv`: every peak of a high-resolution trace with its baseline, from `mzlab_core::peakq_auto::auto_integrate`, the
//! function behind the one-click integration of the page (`QUALITA.autoIntegrate`). The `--set` names are the snake_case of the page settings (`PK`).

use mzlab_core::baseline::integrate;
use mzlab_core::peakq_auto::{auto_integrate, AutoParams};

use crate::peaks_edges_cmd::trace;
use crate::plain::peak_area_cmd::{baseline_of, is_base};
use crate::Outcome;

pub const HELP: &str = "mzlab-cli peak-auto: every peak of a high-resolution trace with its baseline, as JSON.

usage: mzlab-cli peak-auto TRACE.csv|- [--from MIN] [--to MIN] [--set NAME=VALUE...]

  TRACE.csv    two columns, RT (min) and intensity; - = the standard input
  --from/--to  the stretch whose apexes count (default: the whole trace)
  --set        the page dialog: hr_smooth (points, odd), hr_sn; engine only: hr_prom (% of the tallest), hr_scans, hr_half (points, 0 = auto), hr_slope (%);
               the area of every peak: baseline = edges (default, b0-b1), zero, or line with base_y0, base_t0, base_y1, base_t1 (see peak-area --help)
  output       {\"peaks\":[{\"t0\":MIN,\"t1\":MIN,\"b0\":BASELINE,\"b1\":BASELINE,\"area\":COUNTS_MIN}...]}; with zero or line each peak
               also has \"baseline\" and \"below\" (COUNTS_MIN, the part under the line) and its area is the signed one against that line";

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
    let mut bases = Vec::new();
    let res = (|| -> Result<(), String> {
        while let Some(a) = it.next() {
            match a.as_str() {
                "--from" => from = Some(num(it.next(), "--from")?),
                "--to" => to = Some(num(it.next(), "--to")?),
                "--set" => {
                    let kv = it.next().ok_or("--set needs NAME=VALUE")?;
                    let (k, v) = kv.split_once('=').ok_or("--set needs NAME=VALUE")?;
                    if is_base(k) {
                        bases.push((k.to_string(), v.to_string()));
                    } else {
                        set(&mut p, k, v)?;
                    }
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
    let base = match baseline_of(&bases) {
        Ok(b) => b,
        Err(e) => return Outcome::fail(&e),
    };
    match trace(&src) {
        Err(e) => Outcome::fail(&e),
        Ok((x, y)) => {
            let (x0, x1) = (from.unwrap_or(x[0]), to.unwrap_or(x[x.len() - 1]));
            let rows: Vec<String> = auto_integrate(&x, &y, x0, x1, p)
                .iter()
                .map(|k| {
                    let head = format!(
                        "{{\"t0\":{},\"t1\":{},\"b0\":{},\"b1\":{}",
                        x[k.lo], x[k.hi], k.base_lo, k.base_hi
                    );
                    match base.at(0.0) {
                        None => format!("{head},\"area\":{}}}", k.area),
                        Some(_) => {
                            let r = integrate(&x, &y, k.lo, k.hi, |t| base.at(t).unwrap_or(0.0));
                            format!(
                                "{head},\"area\":{},\"baseline\":\"{}\",\"below\":{}}}",
                                r.area,
                                base.name(),
                                r.below
                            )
                        }
                    }
                })
                .collect();
            Outcome::ok(format!("{{\"peaks\":[{}]}}\n", rows.join(",")).into_bytes())
        }
    }
}

#[cfg(test)]
#[path = "peaks_auto_cmd_tests.rs"]
mod tests;
