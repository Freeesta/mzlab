//! `mzlab-cli peaks-edges TRACE.csv --at T`: the limits of the low-resolution peak under a click, from `mzlab_core::peakq_lr::lr_edges`, the same
//! function the page asks through `QUALITA.edges`. The trace is two columns (RT in min, intensity), comma, tab or space separated, header optional.

use mzlab_core::peakq_lr::{lr_edges, LrParams, NoiseMode};

use crate::Outcome;

pub const HELP: &str = "mzlab-cli peaks-edges: the limits of the peak at a time on a low-resolution trace, as JSON.

usage: mzlab-cli peaks-edges TRACE.csv|- --at MIN [--ref MIN] [--mrm] [--set NAME=VALUE...]

  TRACE.csv    two columns, RT (min) and intensity; - = the standard input
  --at MIN     the time of the click
  --ref MIN    base width of the peaks already integrated (used with --set constrain=PERCENT)
  --mrm        the trace is an MRM transition
  --set        win, areaNoise, edgeFrac, snMin, noise (mad|rms|manual), minHalf, multi, tail, constrain, nr0, nr1 (the dialog of the page)
  output       {\"edges\":[t0,t1]}, or {\"edges\":null} when the S/N is under snMin";

fn trace(src: &str) -> Result<(Vec<f64>, Vec<f64>), String> {
    let text = if src == "-" {
        let mut t = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut t).map_err(|e| e.to_string())?;
        t
    } else {
        std::fs::read_to_string(src).map_err(|e| format!("{src}: {e}"))?
    };
    let (mut x, mut y) = (Vec::new(), Vec::new());
    for line in text.lines() {
        let mut f = line.split([',', '\t', ' ', ';']).filter(|s| !s.is_empty());
        if let (Some(Ok(a)), Some(Ok(b))) = (
            f.next().map(str::parse::<f64>),
            f.next().map(str::parse::<f64>),
        ) {
            x.push(a);
            y.push(b);
        }
    }
    if x.is_empty() {
        return Err(format!("{src}: no numeric rows"));
    }
    Ok((x, y))
}

fn set(p: &mut LrParams, name: &str, v: &str) -> Result<(), String> {
    if name == "noise" {
        p.noise = match v {
            "mad" => NoiseMode::Mad,
            "rms" => NoiseMode::Rms,
            "manual" => NoiseMode::Manual,
            _ => return Err("noise is mad, rms or manual".into()),
        };
        return Ok(());
    }
    let n: f64 = v.parse().map_err(|_| format!("{name} needs a number"))?;
    match name {
        "win" => p.win = n,
        "areaNoise" => p.area_noise = n,
        "edgeFrac" => p.edge_frac = n,
        "snMin" => p.sn_min = n,
        "minHalf" => p.min_half = n,
        "multi" => p.multi = n,
        "tail" => p.tail = n,
        "constrain" => p.constrain = n,
        "nr0" => p.nr0 = n,
        "nr1" => p.nr1 = n,
        _ => return Err(format!("unknown parameter {name}")),
    }
    Ok(())
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let (mut src, mut at, mut reference, mut mrm, mut p) =
        (None, None, 0.0, false, LrParams::default());
    let res = (|| -> Result<(), String> {
        while let Some(a) = it.next() {
            match a.as_str() {
                "--at" => {
                    at = Some(
                        it.next()
                            .and_then(|v| v.parse::<f64>().ok())
                            .ok_or("--at needs a number")?,
                    )
                }
                "--ref" => {
                    reference = it
                        .next()
                        .and_then(|v| v.parse::<f64>().ok())
                        .ok_or("--ref needs a number")?
                }
                "--mrm" => mrm = true,
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
    let (Some(src), Some(at)) = (src, at) else {
        return Outcome::fail("peaks-edges needs a trace and --at (see --help)");
    };
    match trace(&src) {
        Err(e) => Outcome::fail(&e),
        Ok((x, y)) => {
            let body = match lr_edges(&x, &y, at, reference, mrm, &p) {
                Some((a, b)) => format!("{{\"edges\":[{a},{b}]}}\n"),
                None => "{\"edges\":null}\n".to_string(),
            };
            Outcome::ok(body.into_bytes())
        }
    }
}

#[cfg(test)]
#[path = "peaks_edges_cmd_tests.rs"]
mod tests;
