//! `mzlab-cli peak noise TRACE.csv`: the noise of an ion-counting trace with a floor of one ion and, with `--rt A,B`, the S/N and check of
//! that peak (`mzlab_core::ionstat`, rule R1 of docs/studies/TP_SCREENING_RULES.md). Stdout is `noise-floor/1`, the bytes of mzlab-wasm
//! `noise_floor`; the one log line of the choice goes to stderr as `note:`.

use mzlab_core::ionstat::{floor_json, floor_log, measure, FloorAsk, FloorParams, OneIon};

use crate::peaks_edges_cmd::trace;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli peak noise: the noise of an ion-counting trace with a floor of one ion, and the S/N of a peak on it, as JSON (noise-floor/1).

usage: mzlab-cli peak noise TRACE.csv|- [--rt A,B] [--set NAME=VALUE...]

  TRACE.csv    two columns, RT (min) and intensity (cps); - = the standard input
  --rt A,B     the peak window (min): left out of the background; its apex above the background median gives sn, sn_mad and the check
  --set        from, to (min: the stretch whose points are background, default the whole trace), one_ion (cps of one ion; auto = the quantum
               of the trace, the smallest positive value or step between values, when >= 90 % of the values are its multiples; 0 = off),
               sd_weight (0.5), min_sn (5)
  rule         noise = max(1.4826 MAD, sd_weight x SD, one_ion x sqrt(max(lambda, 1))), lambda = mean background counts per point; check pass
               (S/N >= min_sn), fail, or warn (no ion term, or noise 0)
  output       {\"schema\":\"noise-floor/1\",\"params\",\"background\":{n, median, mean},\"quantum\":{q, support, n_positive, accepted}|null,
               \"terms\":{mad, sd, sd_term, one_ion, one_ion_source, lambda, ion_term},\"noise\",\"estimator\":mad|sd|one_ion|poisson,
               \"peak\":{from, to, apex_rt, height, sn, sn_mad, ions}|null,\"check\":{id, status, value, threshold, rule}|null}
  exit         0 ok (whatever the check says), 1 no background point in the stretch, 2 usage, 4 missing trace";

fn set(p: &mut FloorParams, ask: &mut FloorAsk, name: &str, v: &str) -> Result<(), String> {
    if name == "from" || name == "to" {
        let t = num(Some(v.into()), name)?;
        *(if name == "from" { &mut ask.from } else { &mut ask.to }) = Some(t);
        return Ok(());
    }
    if name == "one_ion" && v == "auto" {
        p.one_ion = OneIon::Auto;
        return Ok(());
    }
    let n: f64 = v.parse().map_err(|_| format!("{name} needs a number"))?;
    if !n.is_finite() || n < 0.0 {
        return Err(format!("{name} must be a number not below 0"));
    }
    match name {
        "one_ion" if n == 0.0 => p.one_ion = OneIon::Off,
        "one_ion" => p.one_ion = OneIon::Cps(n),
        "sd_weight" => p.sd_weight = n,
        "min_sn" => p.min_sn = n,
        _ => return Err(format!("unknown parameter {name}")),
    }
    Ok(())
}

fn num(v: Option<String>, flag: &str) -> Result<f64, String> {
    v.and_then(|s| s.parse::<f64>().ok())
        .filter(|x| x.is_finite())
        .ok_or_else(|| format!("{flag} needs a number"))
}

fn window(v: Option<String>) -> Result<(f64, f64), String> {
    let s = v.ok_or("--rt needs A,B")?;
    let (a, b) = s.split_once(',').ok_or("--rt needs A,B")?;
    let (a, b) = (num(Some(a.into()), "--rt")?, num(Some(b.into()), "--rt")?);
    if a > b {
        return Err("--rt needs A <= B".into());
    }
    Ok((a, b))
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let (mut src, mut ask, mut p) = (None, FloorAsk::default(), FloorParams::default());
    let res = (|| -> Result<(), String> {
        while let Some(a) = it.next() {
            match a.as_str() {
                "--rt" => ask.peak = Some(window(it.next())?),
                "--set" => {
                    let kv = it.next().ok_or("--set needs NAME=VALUE")?;
                    let (k, v) = kv.split_once('=').ok_or("--set needs NAME=VALUE")?;
                    set(&mut p, &mut ask, k, v)?;
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
        return Outcome::fail("peak noise needs a trace (see --help)");
    };
    let (x, y) = match trace(&src) {
        Err(e) => return Outcome::fail(&e),
        Ok(t) => t,
    };
    match measure(&x, &y, ask, p) {
        Err(key) => Outcome::fail(&key),
        Ok(r) => {
            let mut o = Outcome::ok(format!("{}\n", floor_json(&r)).into_bytes());
            o.stderr = format!("note: {}\n", floor_log(&r));
            o
        }
    }
}

#[cfg(test)]
#[path = "peak_noise_cmd_tests.rs"]
mod tests;
