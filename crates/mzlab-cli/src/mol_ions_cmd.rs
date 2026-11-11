//! `mzlab-cli mol-ions REQUEST.json|- [--set decimals=N] [--set ppm=X] [--out PATH]`: the `ions/1` of the structures of a drawing (the ESI+ and
//! ESI− ions expected from the adducts of the m/z calculator), byte for byte what the «Expected ions» card of the Draw page and of the app shows.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-ions: the ions expected in ESI+ and ESI- for drawn structures, as JSON (ions/1).

usage: mzlab-cli mol-ions REQUEST.json|- [--set NAME=VALUE...] [--out PATH]

  REQUEST.json    {\"structures\": [[{\"el\", \"iso\", \"h\", \"q\", \"r\"}, ...], ...], \"decimals\": 4, \"ppm\": 5}: element, mass number
                  (0 or absent = natural), implicit hydrogens, charge and radical code of every atom; - reads the standard input
  --set decimals  decimals of the m/z text, 0-6 (default 4)
  --set ppm       half width of the XIC window lo-hi, in ppm (default 5)
  --out PATH      write there instead of the standard output

The answer is {\"format\": \"ions/1\", \"decimals\", \"ppm\", \"structures\": [{formula, text, charge, mass, kind, rows} or {error, label}]};
each row is {name, id, pol, k, z, common, formula, text, mz, mzText, lo, hi, odd}. A neutral structure gets the adducts of the m/z
calculator (doubly charged ones only above 150 Da with two N for 2+ or four O for 2-), a charged one its own ion. Every row is a
candidate m/z to look for, never an identification.";

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out, mut dec, mut ppm) = (Vec::new(), None, None, None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            "--set" => {
                let Some(kv) = it.next() else {
                    return Outcome::fail("--set needs NAME=VALUE");
                };
                let (k, v) = kv.split_once('=').unwrap_or((kv.as_str(), ""));
                let Ok(n) = v.trim().parse::<f64>() else {
                    return Outcome::fail(&format!("--set {k}: {v} is not a number"));
                };
                match k {
                    "decimals" => dec = Some(n),
                    "ppm" => ppm = Some(n),
                    _ => {
                        return Outcome::fail(&format!(
                            "--set {k}: unknown parameter (decimals, ppm)"
                        ))
                    }
                }
            }
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    let [src] = pos.as_slice() else {
        return Outcome::fail("mol-ions needs one request file (or - for the standard input)");
    };
    let text = if src == "-" {
        let mut t = String::new();
        if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut t) {
            return Outcome::fail(&e.to_string());
        }
        t
    } else {
        match std::fs::read_to_string(src) {
            Ok(t) => t,
            Err(e) => return Outcome::fail(&format!("{src}: {e}")),
        }
    };
    match mzlab_feat::chem::ions::ions_json_set(&text, dec, ppm) {
        Ok(j) => crate::emit(format!("{j}\n"), &out),
        Err(e) => Outcome::fail(&e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::execute;

    fn run(a: &[&str]) -> crate::Outcome {
        execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn the_command_prints_the_function_json() {
        let req = r#"{"structures":[[{"el":"C","h":3},{"el":"O","h":1}]],"decimals":2}"#;
        let p = std::env::temp_dir().join(format!("mol_ions_{}.json", std::process::id()));
        std::fs::write(&p, req).unwrap();
        let o = run(&[
            "mol-ions",
            p.to_str().unwrap(),
            "--set",
            "decimals=5",
            "--set",
            "ppm=3",
        ]);
        std::fs::remove_file(&p).ok();
        assert_eq!(o.code, 0);
        let want = mzlab_feat::chem::ions::ions_json_set(req, Some(5.0), Some(3.0)).unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
        assert!(want.contains("\"decimals\":5,\"ppm\":3,"));
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["mol-ions"],
            vec!["mol-ions", "a", "b"],
            vec!["mol-ions", "--nope"],
            vec!["mol-ions", "x.json", "--set", "decimals=two"],
            vec!["mol-ions", "x.json", "--set", "colour=1"],
            vec!["mol-ions", "/no/such/file.json"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["mol-ions", "--help"]).code, 0);
    }
}
