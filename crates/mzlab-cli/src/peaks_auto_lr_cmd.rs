//! `mzlab-cli peak auto-lr TRACE.csv`: every peak of a jagged low-resolution SIM/MRM trace, from `mzlab_core::peakq_lrauto`, the function behind
//! «Integrate all peaks» of the page (`QUALITA.lrAuto`). The JSON is `lr_auto_json`, the bytes the page gets; `--set` names are those of the dialog.

use mzlab_core::area_unit::{lr_auto_json_in, AreaUnit};
use mzlab_core::peakq_lrauto::{lr_auto_integrate, LrAutoParams, Split};

use crate::peaks_edges_cmd::trace;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli peak auto-lr: every peak of a low-resolution trace (SIM, MRM), found on a smoothed copy and integrated on the raw points, as JSON.

usage: mzlab-cli peak auto-lr TRACE.csv|- [--from MIN] [--to MIN] [--split drop|valley] [--area-unit s|min] [--set NAME=VALUE...]

  TRACE.csv    two columns, RT (min) and intensity; - = the standard input
  --from/--to  the stretch whose apexes count (default: the whole trace)
  --set        the names of the \"params\" of the output: half (Savitzky-Golay half window of the detection copy, points; 0 = 40 % of the FWHM of the
               tallest peak), valley (fraction 0-1 of the smaller peak the valley must go down to keep two peaks, as printed), sn (smallest height over
               the noise), min_pts, despike (1/0: median of 3 against spikes); numbers only. The old names lr_half, lr_valley (in %), lr_sn,
               lr_min_pts, lr_despike still work and log a note on stderr
  --split MODE drop (default) = one baseline under touching peaks, valley = each baseline ends at the valley
  --area-unit  s = counts x s (default, as peak area), min = counts x min
  output       {\"params\":{half, half_auto, valley, sn, min_pts, despike, split, noise, baseline (always \"engine\")}, \"peaks\":[{lo, hi, t0, t1, rt, b0, b1, area, below,
               area_zero, height, sn, end0, end1}...]}: indices, min, counts; areas in counts x s (area_unit) of the raw trace against the straight line
               b0-b1 (signed; below = the part under it) and from zero; end = background | valley | cap (5 FWHM)";

/// Sets one parameter by the name the output prints (`valley` is a fraction there); the old dialog names (`lr_*`, valley in %) are aliases and
/// return a note for stderr.
fn set(p: &mut LrAutoParams, name: &str, v: &str) -> Result<Option<String>, String> {
    let n: f64 = v.parse().map_err(|_| format!("{name} needs a number"))?;
    if n.is_nan() || n < 0.0 {
        return Err(format!("{name} must not be negative"));
    }
    let (key, old) = match name.strip_prefix("lr_") {
        Some(k) => (k, true),
        None => (name, false),
    };
    match key {
        "half" => p.half = n as usize,
        "valley" if old => p.valley = LrAutoParams::from_slice(&[f64::NAN, n]).valley,
        "valley" if n > 1.0 => {
            return Err("valley is a fraction between 0 and 1 (lr_valley takes %)".into())
        }
        "valley" => p.valley = n,
        "sn" => p.snr = n,
        "min_pts" => p.min_pts = n as usize,
        "despike" => p.despike = n != 0.0,
        _ => return Err(format!("unknown parameter {name}")),
    }
    Ok(old.then(|| {
        let unit = if key == "valley" {
            " (still in %; the new name takes a fraction)"
        } else {
            ""
        };
        format!("note: --set {name} is the old name of {key}{unit}")
    }))
}

fn num(v: Option<String>, flag: &str) -> Result<f64, String> {
    v.and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| format!("{flag} needs a number"))
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let (mut src, mut from, mut to, mut p) = (None, None, None, LrAutoParams::default());
    let mut unit = AreaUnit::default();
    let mut notes = String::new();
    let res = (|| -> Result<(), String> {
        while let Some(a) = it.next() {
            match a.as_str() {
                "--from" => from = Some(num(it.next(), "--from")?),
                "--to" => to = Some(num(it.next(), "--to")?),
                "--area-unit" => {
                    unit = AreaUnit::parse(&it.next().ok_or("--area-unit needs s or min")?)?
                }
                "--split" => {
                    p.split = match it.next().ok_or("--split needs drop or valley")?.as_str() {
                        "drop" => Split::Drop,
                        "valley" => Split::Valley,
                        _ => return Err("--split is drop or valley".into()),
                    }
                }
                "--set" => {
                    let kv = it.next().ok_or("--set needs NAME=VALUE")?;
                    let (k, v) = kv.split_once('=').ok_or("--set needs NAME=VALUE")?;
                    if let Some(n) = set(&mut p, k, v)? {
                        notes.push_str(&n);
                        notes.push('\n');
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
        return Outcome::fail("peak auto-lr needs a trace (see --help)");
    };
    match trace(&src) {
        Err(e) => Outcome::fail(&e),
        Ok((x, y)) => {
            let (x0, x1) = (from.unwrap_or(x[0]), to.unwrap_or(x[x.len() - 1]));
            let r = lr_auto_integrate(&x, &y, x0, x1, p);
            let mut o = Outcome::ok(format!("{}\n", lr_auto_json_in(&x, &r, unit)).into_bytes());
            o.stderr = notes;
            o
        }
    }
}

#[cfg(test)]
#[path = "peaks_auto_lr_cmd_tests.rs"]
mod tests;
