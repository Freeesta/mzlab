//! `mzlab-cli mol-check REQUEST.json|- [--out PATH]`: the `check/1` of a drawing (what is wrong in it: valence, charge, isotope, wedges,
//! aromatic rings, query atoms, separate pieces), byte for byte what the Check panel of Draw shows on the page and in the app.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-check: what is wrong in a drawing (valence, charge, isotope, wedges, aromatic rings, query atoms), as JSON (check/1).

usage: mzlab-cli mol-check REQUEST.json|- [--out PATH]

  REQUEST.json  {\"molfile\": \"...\"}: the V2000 or V3000 molfile of the drawing; - reads the standard input
  --out PATH    write there instead of the standard output

The answer is {\"format\": \"check/1\", \"atoms\", \"pieces\", \"warnings\": [{code, level, atoms, bonds, ...}]}: codes valence,
valenceCharge, charge, isotope, kekule, wedgeMultiple, wedgeNoCentre, wedgeUndecided (level warn) and query, queryBond, pieces (level note);
atoms are 0-based in the order of the molfile. The check speaks of the drawing only, never of data.";

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out) = (Vec::new(), None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    let [src] = pos.as_slice() else {
        return Outcome::fail("mol-check needs one request file (or - for the standard input)");
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
    match mzlab_feat::chem::check::check_json(&text) {
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
        let req = r#"{"molfile":"\n  t\n\n  1  0  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\nM  CHG  1   1   2\nM  END\n"}"#;
        let p = std::env::temp_dir().join(format!("mol_check_{}.json", std::process::id()));
        std::fs::write(&p, req).unwrap();
        let o = run(&["mol-check", p.to_str().unwrap()]);
        std::fs::remove_file(&p).ok();
        assert_eq!(o.code, 0);
        assert!(String::from_utf8_lossy(&o.stdout).contains(r#""code":"charge""#));
        let want = mzlab_feat::chem::check::check_json(req).unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["mol-check"],
            vec!["mol-check", "a", "b"],
            vec!["mol-check", "--nope"],
            vec!["mol-check", "/no/such/file.json"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["mol-check", "--help"]).code, 0);
    }
}
