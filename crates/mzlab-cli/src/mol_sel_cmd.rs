//! `mzlab-cli mol-sel REQUEST.json|- [--out PATH]`: the `sel/1` of a selection or of the whole drawing (formula, charge, monoisotopic and average
//! mass, m/z, heavy atoms, rings and aromatic rings), byte for byte what the Info of the Draw right-click menu shows in the page and the app.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-sel: formula, charge, masses, heavy atoms, rings and aromatic rings of a drawn structure, as JSON (sel/1).

usage: mzlab-cli mol-sel REQUEST.json|- [--out PATH]

  REQUEST.json  {\"molfile\": V2000 text, \"part\": true|false}: the structure as drawn (open valences filled with H), part = a selection;
                - reads the standard input
  --out PATH    write there instead of the standard output

The answer is {\"format\": \"sel/1\", \"part\", \"label\": {formula, text, charge, mass, nominal, mz, ...} (as etich/1), \"average\",
\"heavy\", \"atoms\", \"pieces\", \"rings\", \"aromaticAtoms\", \"aromaticRings\"}.";

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
        return Outcome::fail("mol-sel needs one request file (or - for the standard input)");
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
    match mzlab_feat::chem::sel::sel_json(&text) {
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
        let mf = "\n  t\n\n  1  0  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\nM  END";
        let req = format!("{{\"molfile\":{mf:?},\"part\":true}}");
        let p = std::env::temp_dir().join(format!("mol_sel_{}.json", std::process::id()));
        std::fs::write(&p, &req).unwrap();
        let o = run(&["mol-sel", p.to_str().unwrap()]);
        std::fs::remove_file(&p).ok();
        assert_eq!(o.code, 0);
        let want = mzlab_feat::chem::sel::sel_json(&req).unwrap();
        assert!(want.contains("\"formula\":\"CH4\""), "{want}");
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["mol-sel"],
            vec!["mol-sel", "a", "b"],
            vec!["mol-sel", "--nope"],
            vec!["mol-sel", "/no/such/file.json"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["mol-sel", "--help"]).code, 0);
    }
}
