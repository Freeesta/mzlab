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

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out, mut tables) = (Vec::new(), None, None);
    let (mut hr, mut mda, mut ppm, mut pol) = (false, None, None, String::new());
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            "--tables" => match it.next() {
                Some(p) => tables = Some(PathBuf::from(p)),
                None => return Outcome::fail("--tables needs a path"),
            },
            "--set" => {
                let Some((k, v)) = it.next().and_then(|s| {
                    s.split_once('=')
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                }) else {
                    return Outcome::fail("--set needs NAME=VALUE");
                };
                match k.as_str() {
                    "hr" => hr = matches!(v.as_str(), "1" | "true"),
                    "mda" | "ppm" => match number(&k, &v) {
                        Ok(n) if k == "mda" => mda = Some(n),
                        Ok(n) => ppm = Some(n),
                        Err(m) => return Outcome::fail(&m),
                    },
                    "pol" if matches!(v.as_str(), "+" | "-" | "") => pol = v,
                    _ => return Outcome::fail(&format!("unknown --set {k}={v}")),
                }
            }
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    let ([x], Some(tables)) = (pos.as_slice(), tables) else {
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
    let text = match std::fs::read_to_string(&tables) {
        Ok(t) => t,
        Err(e) => return Outcome::fail(&format!("{}: {e}", tables.display())),
    };
    let Some(rest) = text.trim().strip_prefix('{') else {
        return Outcome::fail("--tables must hold one JSON object");
    };
    let req = format!(
        r#"{{"x":{x},"hr":{hr},"tol_mda":{},"tol_ppm":{},"pol":"{pol}",{rest}"#,
        mda.unwrap_or(3.0),
        ppm.unwrap_or(5.0)
    );
    match mzlab_core::search::match_number(&req) {
        Ok(j) => crate::emit(format!("{j}\n"), &out),
        Err(e) => Outcome::fail_error(&e),
    }
}

#[cfg(test)]
mod tests {
    use crate::execute;

    /// Synthetic tables: no real list, only masses of simple formulas.
    const TABLES: &str = r#"{"losses":[{"id":"H2O","f":"H2O"},{"id":"NH3","f":"NH3"}],
        "adducts":[{"id":"[M+H]+","k":1,"add":"H","z":1},{"id":"[M+Na]+","k":1,"add":"Na","z":1},{"id":"[M-H]-","k":1,"sub":"H","z":-1}],
        "contam":[{"id":"peg1","mz":63.04406,"pol":1},{"id":"x","mz":18.2,"pol":-1}]}"#;

    fn run(a: &[&str]) -> crate::Outcome {
        execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    fn tables_file() -> ((), String) {
        let p = std::env::temp_dir().join(format!("mzlab_match_mz_{}.json", std::process::id()));
        std::fs::write(&p, TABLES).unwrap();
        ((), p.to_str().unwrap().to_string())
    }

    #[test]
    fn the_output_is_the_core_answer_byte_for_byte() {
        let (_d, t) = tables_file();
        let o = run(&[
            "match-mz", "18.0106", "--tables", &t, "--set", "hr=1", "--set", "mda=3",
        ]);
        assert_eq!(o.code, 0);
        let req = format!(
            r#"{{"x":18.0106,"hr":true,"tol_mda":3,"tol_ppm":5,"pol":"",{}"#,
            TABLES.trim().strip_prefix('{').unwrap()
        );
        let want = mzlab_core::search::match_number(&req).unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
    }

    #[test]
    fn a_contaminant_is_a_hit_with_its_error_and_never_a_verdict() {
        let (_d, t) = tables_file();
        let o = run(&[
            "match-mz", "63.0441", "--tables", &t, "--set", "hr=1", "--set", "ppm=10", "--set",
            "pol=+",
        ]);
        let s = String::from_utf8(o.stdout).unwrap();
        assert!(s.contains("peg1") && s.contains("contaminant"), "{s}");
    }

    #[test]
    fn usage_errors_exit_2() {
        let (_d, t) = tables_file();
        for a in [
            vec!["match-mz"],
            vec!["match-mz", "18"],
            vec!["match-mz", "18", "--tables", "/nonexistent.json"],
            vec!["match-mz", "18", "--tables", &t, "--set", "ppm=abc"],
            vec!["match-mz", "18", "--tables", &t, "--set", "nope=1"],
            vec!["match-mz", "18", "--tables", &t, "--nope"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["match-mz", "--help"]).code, 0);
    }
}
