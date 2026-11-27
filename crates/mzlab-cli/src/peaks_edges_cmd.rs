//! `mzlab-cli peak edges TRACE.csv --at T`: the limits of the low-resolution peak under a click, from `mzlab_core::peakq_lr::lr_edges`, the same
//! function the page asks through `QUALITA.edges`. The trace is two columns (RT in min, intensity), comma, tab or space separated, header optional.

use mzlab_core::peakq_click::{lr_click, ClickGuard};
use mzlab_core::peakq_lr::{lr_edges, LrParams, NoiseMode};

use crate::Outcome;

pub const HELP: &str = "mzlab-cli peak edges: the limits of the peak at a time on a low-resolution trace, as JSON.

usage: mzlab-cli peak edges TRACE.csv|- --at MIN [--ref MIN] [--mrm] [--noise mad|rms|manual] [--set NAME=VALUE...]

  TRACE.csv    two columns, RT (min) and intensity; - = the standard input
  --at MIN     the time of the click
  --ref MIN    base width of the peaks already integrated (used with --set constrain=PERCENT)
  --mrm        the trace is an MRM transition
  --noise MODE the noise estimate: mad (default), rms or manual
  --set        numbers only; the page dialog: win, edge_frac, sn_min, multi, tail; engine only: area_noise, min_half, constrain, nr0, nr1
  --set click=1  the page click: the apex nearest --at, evidence and guard {ok,edges,apex,points,area,height,noise,sn,refused,forced,params}; exit 1 when refused;
                 guard min_points=5 min_sn=3 reach=0.25 (min); force=1 keeps a refused window (reasons stay in refused)
  output       {\"edges\":[t0,t1]}, or {\"edges\":null} when the S/N is under sn_min";

/// The click mode of `--set click=1` and its guard.
#[derive(Default)]
struct Click {
    on: bool,
    guard: ClickGuard,
}

pub(crate) fn trace(src: &str) -> Result<(Vec<f64>, Vec<f64>), String> {
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

fn set(p: &mut LrParams, g: &mut Click, name: &str, v: &str) -> Result<(), String> {
    let n: f64 = v.parse().map_err(|_| format!("{name} needs a number"))?;
    match name {
        "click" => g.on = n != 0.0,
        "force" => g.guard.force = n != 0.0,
        "min_points" => g.guard.min_points = n.max(0.0) as usize,
        "min_sn" => g.guard.min_sn = n,
        "reach" => g.guard.reach = n,
        "win" => p.win = n,
        "area_noise" => p.area_noise = n,
        "edge_frac" => p.edge_frac = n,
        "sn_min" => p.sn_min = n,
        "min_half" => p.min_half = n,
        "multi" => p.multi = n,
        "tail" => p.tail = n,
        "constrain" => p.constrain = n,
        "nr0" => p.nr0 = n,
        "nr1" => p.nr1 = n,
        _ => return Err(format!("unknown parameter {name}")),
    }
    Ok(())
}

/// The click of the page: JSON on stdout, the automatic choice and the guard numbers on stderr; a refused window exits 1 and says how to keep it.
fn click_outcome(
    x: &[f64],
    y: &[f64],
    at: f64,
    mrm: bool,
    p: &LrParams,
    g: &ClickGuard,
) -> Outcome {
    let c = lr_click(x, y, at, mrm, p, g);
    let mut o = Outcome::ok(format!("{}\n", c.to_json()).into_bytes());
    let forced = if c.forced {
        format!("; forced past {:?}", c.refused)
    } else {
        String::new()
    };
    o.stderr = format!(
        "note: click at {at} min: apex {}, window {:?}, {} points, S/N {:.1}; guard min_points={} min_sn={} reach={}{forced}\n",
        c.apex.map_or("none".to_string(), |a| a.to_string()),
        c.edges,
        c.points,
        c.sn,
        g.min_points,
        g.min_sn,
        g.reach
    );
    if !c.ok() {
        o.code = crate::exit::DATA;
        o.stderr += &format!("refused: {:?}; --set force=1 keeps the window\n", c.refused);
    }
    o
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let (mut src, mut at, mut reference, mut mrm, mut p) =
        (None, None, 0.0, false, LrParams::default());
    let mut click = Click::default();
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
                "--noise" => {
                    p.noise = match it
                        .next()
                        .ok_or("--noise needs mad, rms or manual")?
                        .as_str()
                    {
                        "mad" => NoiseMode::Mad,
                        "rms" => NoiseMode::Rms,
                        "manual" => NoiseMode::Manual,
                        _ => return Err("noise is mad, rms or manual".into()),
                    }
                }
                "--set" => {
                    let kv = it.next().ok_or("--set needs NAME=VALUE")?;
                    let (k, v) = kv.split_once('=').ok_or("--set needs NAME=VALUE")?;
                    set(&mut p, &mut click, k, v)?;
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
        return Outcome::fail("peak edges needs a trace and --at (see --help)");
    };
    match trace(&src) {
        Err(e) => Outcome::fail(&e),
        Ok((x, y)) if click.on => click_outcome(&x, &y, at, mrm, &p, &click.guard),
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
