//! `mzlab-cli mol-label REQUEST.json|- [--out PATH]`: the `etich/1` of the structures of a drawing (formula with isotopes, charge, monoisotopic
//! and nominal mass, m/z, and what changes across each arrow), byte for byte what the Draw labels of the page and the app show.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-label: formula, charge and masses of drawn structures and the change across reaction arrows, as JSON (etich/1).

usage: mzlab-cli mol-label REQUEST.json|- [--out PATH]

  REQUEST.json  {\"structures\": [[{\"el\", \"iso\", \"h\", \"q\"}, ...], ...], \"arrows\": [[before, after], ...]}: element, mass number
                (0 or absent = natural), implicit hydrogens and charge of every atom; - reads the standard input
  --out PATH    write there instead of the standard output

The answer is {\"format\": \"etich/1\", \"structures\": [{formula, text, charge, mass, nominal, mz} or {error, label}],
\"arrows\": [{gain, loss, isotopologue, dm, dnominal} or null]}.";

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
        return Outcome::fail("mol-label needs one request file (or - for the standard input)");
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
    match mzlab_feat::chem::etichette::labels_json(&text) {
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
        let req = r#"{"structures":[[{"el":"C","iso":13,"h":4}]]}"#;
        let p = std::env::temp_dir().join(format!("mol_label_{}.json", std::process::id()));
        std::fs::write(&p, req).unwrap();
        let o = run(&["mol-label", p.to_str().unwrap()]);
        std::fs::remove_file(&p).ok();
        assert_eq!(o.code, 0);
        let want = mzlab_feat::chem::etichette::labels_json(req).unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["mol-label"],
            vec!["mol-label", "a", "b"],
            vec!["mol-label", "--nope"],
            vec!["mol-label", "/no/such/file.json"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["mol-label", "--help"]).code, 0);
    }
}
