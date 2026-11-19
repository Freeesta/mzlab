//! `mzlab-cli peak auto TRACE.csv`: every peak of a high-resolution trace with its baseline, from `mzlab_core::peakq_auto::auto_integrate`, the
//! function behind the one-click integration of the page (`QUALITA.autoIntegrate`). The `--set` names are the snake_case of the page settings (`PK`).

use mzlab_core::area_unit::AreaUnit;
use mzlab_core::baseline::integrate;
use mzlab_core::peakq_auto::{auto_integrate, AutoParams};

use crate::peaks_edges_cmd::trace;
use crate::plain::peak_area_cmd::{baseline_of, is_base};
use crate::Outcome;

pub const HELP: &str = "mzlab-cli peak auto: every peak of a high-resolution trace with its baseline, as JSON.

usage: mzlab-cli peak auto TRACE.csv|- [--from MIN] [--to MIN] [--baseline zero|edges|line] [--area-unit s|min] [--set NAME=VALUE...]

  TRACE.csv    two columns, RT (min) and intensity; - = the standard input
  --from/--to  the stretch whose apexes count (default: the whole trace)
  --baseline   the baseline of the area of every peak: zero (default: any background under the peak is part of the area), edges (the line b0-b1
               found at the peak ends) or line (with --set base_y0, base_t0, base_y1, base_t1, see peak area --help)
  --area-unit  s = counts x s (default, as peak area), min = counts x min (the old unit)
  --set        numbers only; the page dialog: hr_smooth (points, odd), hr_sn; engine only: hr_prom (% of the tallest), hr_scans, hr_half (points, 0 = auto), hr_slope (%),
               and with --baseline line: base_y0, base_t0, base_y1, base_t1
  output       {\"peaks\":[{\"t0\":MIN,\"t1\":MIN,\"b0\":COUNTS,\"b1\":COUNTS,\"area\":COUNTS_S,\"baseline\":\"edges\"}...]} with edges; with zero or line
               each peak has \"t0\", \"t1\", \"area\" (signed, against that line), \"baseline\" and \"below\" (COUNTS_S, the part under the line) and NO b0/b1:
               the ends of the detection baseline are not what the area is measured from, so they are left out";

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
    let mut unit = AreaUnit::default();
    let res = (|| -> Result<(), String> {
        while let Some(a) = it.next() {
            match a.as_str() {
                "--from" => from = Some(num(it.next(), "--from")?),
                "--to" => to = Some(num(it.next(), "--to")?),
                "--area-unit" => {
                    unit = AreaUnit::parse(&it.next().ok_or("--area-unit needs s or min")?)?
                }
                "--baseline" => bases.push((
                    "baseline".to_string(),
                    it.next().ok_or("--baseline needs a mode")?,
                )),
                "--set" => {
                    let kv = it.next().ok_or("--set needs NAME=VALUE")?;
                    let (k, v) = kv.split_once('=').ok_or("--set needs NAME=VALUE")?;
                    if is_base(k) && k != "baseline" {
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
        return Outcome::fail("peak auto needs a trace (see --help)");
    };
    let base = match baseline_of(&bases) {
        Ok(b) => b,
        Err(e) => return Outcome::fail(&e),
    };
    match trace(&src) {
        Err(e) => Outcome::fail(&e),
        Ok((x, y)) => {
            let (x0, x1) = (from.unwrap_or(x[0]), to.unwrap_or(x[x.len() - 1]));
            let f = unit.factor();
            let rows: Vec<String> = auto_integrate(&x, &y, x0, x1, p)
                .iter()
                .map(|k| {
                    match base.at(0.0) {
                        None => format!(
                            "{{\"t0\":{},\"t1\":{},\"b0\":{},\"b1\":{},\"area\":{},\"baseline\":\"edges\"}}",
                            x[k.lo], x[k.hi], k.base_lo, k.base_hi, k.area * f
                        ),
                        Some(_) => {
                            let r = integrate(&x, &y, k.lo, k.hi, |t| base.at(t).unwrap_or(0.0));
                            format!(
                                "{{\"t0\":{},\"t1\":{},\"area\":{},\"baseline\":\"{}\",\"below\":{}}}",
                                x[k.lo],
                                x[k.hi],
                                r.area * f,
                                base.name(),
                                r.below * f
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
