//! `mzlab-cli mol-hilite REQUEST.json|- [--out PATH]`: the `hilite/1` of a drawing (the atoms and bonds a piece of evidence points at, layer by
//! layer), byte for byte what the evidence colours of Draw show on the page and in the app.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-hilite: the atoms and bonds of a drawing that a piece of evidence points at, as JSON (hilite/1).

usage: mzlab-cli mol-hilite REQUEST.json|- [--out PATH]

  REQUEST.json  {\"molfile\": \"...\", \"world\": \"hr\"|\"lr\", \"selected\": [atom], \"layers\": [...]}; - reads the standard input
  --out PATH    write there instead of the standard output

A layer is {kind: \"moiety\", query} (a SMILES or molfile found in the drawing), {kind: \"sites\", smiles, atoms} or {kind: \"cuts\", smiles,
bonds: [[a, b]]} (numbered from 0 on that SMILES, carried onto the drawing through a match of it), each with an optional label. The answer is
{\"format\": \"hilite/1\", \"layers\": [{kind, label, atoms, bonds}]} with atoms 0-based in the order of the molfile; \"skip\" (noMatch, lr,
tooBig) says why a layer is empty. In the LR world only the selected atoms are kept. A coloured atom is where the evidence points, not a proof.";

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
        return Outcome::fail("mol-hilite needs one request file (or - for the standard input)");
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
    match mzlab_feat::chem::hilite::hilite_json(&text) {
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
        // the fixture the app and the molkit `.wasm` are checked against too (tests/test_molkit_wasm.py)
        let fx: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/dati/hilite_disegno.json")).unwrap();
        let req = fx["request"].to_string();
        let p = std::env::temp_dir().join(format!("mol_hilite_{}.json", std::process::id()));
        std::fs::write(&p, req).unwrap();
        let o = run(&["mol-hilite", p.to_str().unwrap()]);
        std::fs::remove_file(&p).ok();
        assert_eq!(o.code, 0);
        assert_eq!(
            String::from_utf8(o.stdout).unwrap(),
            format!("{}\n", fx["hilite"].as_str().unwrap())
        );
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["mol-hilite"],
            vec!["mol-hilite", "a", "b"],
            vec!["mol-hilite", "--nope"],
            vec!["mol-hilite", "/no/such/file.json"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["mol-hilite", "--help"]).code, 0);
    }
}
