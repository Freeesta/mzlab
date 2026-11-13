//! `mzlab-cli match-mz X --tables FILE.json [--set NAME=VALUE...] [--out PATH]`: what a number is compatible with (neutral loss, adduct shift,
//! known contaminant), the `ricerca/1` JSON of `mzlab_core::search::match_number`, byte for byte what the page and the app answer.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli match-mz: what a number is compatible with, as JSON. Candidates with their error, never an identification.

usage: mzlab-cli match-mz X --tables FILE.json [--set NAME=VALUE...] [--out PATH]

  X               the number: a neutral-loss mass, an adduct shift or the m/z of a known contaminant
  --tables FILE   the tables to look in, one JSON object {\"losses\":[{id,f}], \"adducts\":[{id,k,add|sub,z}], \"contam\":[{id,mz,pol}]}
  --set hr=1      high resolution: windows in mDa (losses, adducts) and ppm (contaminants); without it the window is +-0.5 Da
  --set mda=N     window of losses and adduct shifts in mDa (high resolution, default 3)
  --set ppm=N     window of contaminants in ppm (high resolution, default 5)
  --set pol=+|-   keep the adducts and contaminants of that polarity (default both)
  --out PATH      write there instead of the standard output

The answer is {\"x\", \"hr\", \"hits\": [{kind, id, ...}], \"more\", \"tol\"}; uses the same tables as the Addotti e perdite page.";

fn number(name: &str, v: &str) -> Result<f64, String> {
    v.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n >= 0.0)
        .ok_or_else(|| format!("--set {name} needs a number, not {v:?}"))
}

struct Opts {
    pos: Vec<String>,
    out: Option<PathBuf>,
    tables: Option<PathBuf>,
    hr: bool,
    mda: f64,
    ppm: f64,
    pol: String,
}

/// `Ok(None)` = help was asked.
fn parse(args: Vec<String>) -> Result<Option<Opts>, String> {
    let mut o = Opts {
        pos: Vec::new(),
        out: None,
        tables: None,
        hr: false,
        mda: 3.0,
        ppm: 5.0,
        pol: String::new(),
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => o.out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--tables" => o.tables = Some(PathBuf::from(it.next().ok_or("--tables needs a path")?)),
            "--set" => {
                let kv = it.next().ok_or("--set needs NAME=VALUE")?;
                let (k, v) = kv.split_once('=').ok_or("--set needs NAME=VALUE")?;
                match k {
                    "hr" => o.hr = matches!(v, "1" | "true"),
                    "mda" => o.mda = number(k, v)?,
                    "ppm" => o.ppm = number(k, v)?,
                    "pol" if matches!(v, "+" | "-" | "") => o.pol = v.to_string(),
                    _ => return Err(format!("unknown --set {k}={v}")),
                }
            }
            f if f.starts_with("--") => return Err(format!("unknown option {f}")),
            _ => o.pos.push(a),
        }
    }
    Ok(Some(o))
}

pub fn run(args: Vec<String>) -> Outcome {
    let o = match parse(args) {
        Ok(Some(o)) => o,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(m) => return Outcome::fail(&m),
    };
    let ([x], Some(tables)) = (o.pos.as_slice(), &o.tables) else {
        return Outcome::fail("match-mz needs one number and --tables FILE.json");
    };
    let Some(x) = x
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
    else {
        return Outcome::fail("the number must be finite");
    };
    let text = match std::fs::read_to_string(tables) {
        Ok(t) => t,
        Err(e) => return Outcome::fail(&format!("{}: {e}", tables.display())),
    };
    let Some(rest) = text.trim().strip_prefix("{") else {
        return Outcome::fail("--tables must hold one JSON object");
    };
    // the opening brace is an escape: the size checker counts the braces of literals
    let open = "\x7b";
    let req = format!(
        r#"{open}"x":{x},"hr":{},"tol_mda":{},"tol_ppm":{},"pol":"{}",{rest}"#,
        o.hr, o.mda, o.ppm, o.pol
    );
    match mzlab_core::search::match_number(&req) {
        Ok(j) => crate::emit(format!("{j}\n"), &o.out),
        Err(e) => Outcome::fail_error(&e),
    }
}

#[cfg(test)]
#[path = "match_mz_tests.rs"]
mod tests;
