//! `mzlab-cli mol-label REQUEST.json|- [--out PATH]`: the `etich/1` of the structures of a drawing (formula with isotopes, charge, monoisotopic
//! and nominal mass, m/z, and what changes across each arrow), byte for byte what the Draw labels of the page and the app show; `mol-anal` the same
//! way gives the `anal/1` of one structure (the analysis card of Draw: average mass, mass %, theoretical isotope pattern).

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-label: formula, charge and masses of drawn structures and the change across reaction arrows, as JSON (etich/1).

usage: mzlab-cli mol-label REQUEST.json|- [--out PATH]

  REQUEST.json  {\"structures\": [[{\"el\", \"iso\", \"h\", \"q\"}, ...], ...], \"arrows\": [[before, after], ...]}: element, mass number
                (0 or absent = natural), implicit hydrogens and charge of every atom; - reads the standard input
                optional \"deltas\": [\"+O\", ...] by structure: the delta of its brackets, added to formula and masses
  --out PATH    write there instead of the standard output

The answer is {\"format\": \"etich/1\", \"structures\": [{formula, text, charge, mass, nominal, mz, delta?} or {error, label}],
\"arrows\": [{gain, loss, isotopologue, dm, dnominal} or null]}.";

pub const HELP_ANAL: &str = "mzlab-cli mol-anal: average mass, mass % per element and the theoretical isotope pattern of one drawn structure, as JSON (anal/1).

usage: mzlab-cli mol-anal REQUEST.json|- [--out PATH]

  REQUEST.json  {\"atoms\": [{\"el\", \"iso\", \"h\", \"q\", \"r\"}, ...]}: the atoms of one structure as mol-label reads them; - reads the standard input
  --out PATH    write there instead of the standard output

The answer is {\"format\": \"anal/1\", \"structure\": {the etich/1 fields}, \"average\", \"composition\": [{el, count, pct}],
\"pattern\": [{off, mz, rel, abund}]}: sticks grouped by nominal mass, m/z of the ion (mass of a neutral), % of the tallest and of the total.
Theory for the drawn formula: nothing is compared with a spectrum.";

pub fn run(args: Vec<String>) -> Outcome {
    run_json(
        args,
        "mol-label",
        HELP,
        mzlab_feat::chem::labels::labels_json,
    )
}

pub fn run_anal(args: Vec<String>) -> Outcome {
    run_json(
        args,
        "mol-anal",
        HELP_ANAL,
        mzlab_feat::chem::analysis::analysis_json,
    )
}

fn run_json(
    args: Vec<String>,
    name: &str,
    help: &str,
    f: fn(&str) -> mzlab_core::error::Result<String>,
) -> Outcome {
    let (mut pos, mut out) = (Vec::new(), None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{help}\n").into_bytes()),
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    let [src] = pos.as_slice() else {
        return Outcome::fail(&format!(
            "{name} needs one request file (or - for the standard input)"
        ));
    };
    match read_request(src).map(|t| f(&t)) {
        Ok(Ok(j)) => crate::emit(format!("{j}\n"), &out),
        Ok(Err(e)) => Outcome::fail_error(&e),
        Err(e) => Outcome::fail(&e),
    }
}

/// The request file, or the standard input for `-`.
fn read_request(src: &str) -> Result<String, String> {
    if src == "-" {
        let mut t = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut t).map_err(|e| e.to_string())?;
        return Ok(t);
    }
    std::fs::read_to_string(src).map_err(|e| format!("{src}: {e}"))
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
        let want = mzlab_feat::chem::labels::labels_json(req).unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
    }

    #[test]
    fn mol_anal_prints_the_function_json() {
        let req = r#"{"atoms":[{"el":"C","h":3},{"el":"Br"}]}"#;
        let p = std::env::temp_dir().join(format!("mol_anal_{}.json", std::process::id()));
        std::fs::write(&p, req).unwrap();
        let o = run(&["mol-anal", p.to_str().unwrap()]);
        std::fs::remove_file(&p).ok();
        assert_eq!(o.code, 0);
        let want = mzlab_feat::chem::analysis::analysis_json(req).unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
        assert_eq!(run(&["mol-anal"]).code, 2);
        assert_eq!(run(&["mol-anal", "--help"]).code, 0);
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
