//! `mzlab-cli mol-cip REQUEST.json|- [--out PATH]`: the `cip/1` of a drawing (R/S of its defined centres, E/Z of its defined double bonds),
//! byte for byte what the CIP labels of Draw show on the page and in the app.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-cip: the CIP descriptors of the stereo drawn in a structure (R/S, r/s, E/Z), as JSON (cip/1).

usage: mzlab-cli mol-cip REQUEST.json|- [--out PATH]

  REQUEST.json  {\"molfile\": \"...\"}: the V2000 or V3000 molfile of the drawing; - reads the standard input
  --out PATH    write there instead of the standard output

The answer is {\"format\": \"cip/1\", \"atoms\": [{atom, label}], \"bonds\": [{a, b, label}]} with atoms 0-based in the order of the
molfile; only stereo the drawing defines gets a label, and a unit whose ligands the rules cannot tell apart gets none. \"skip\" (query, kekule,
tooBig) says why no label was given. Mass spectrometry cannot tell stereoisomers apart: the labels describe the drawing, not a sample.";

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
        return Outcome::fail("mol-cip needs one request file (or - for the standard input)");
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
    match mzlab_feat::chem::cip::cip_json(&text) {
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
            serde_json::from_str(include_str!("../../../tests/dati/cip_disegno.json")).unwrap();
        let req = fx["request"].to_string();
        let p = std::env::temp_dir().join(format!("mol_cip_{}.json", std::process::id()));
        std::fs::write(&p, req).unwrap();
        let o = run(&["mol-cip", p.to_str().unwrap()]);
        std::fs::remove_file(&p).ok();
        assert_eq!(o.code, 0);
        assert_eq!(
            String::from_utf8(o.stdout).unwrap(),
            format!("{}\n", fx["cip"].as_str().unwrap())
        );
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["mol-cip"],
            vec!["mol-cip", "a", "b"],
            vec!["mol-cip", "--nope"],
            vec!["mol-cip", "/no/such/file.json"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["mol-cip", "--help"]).code, 0);
    }
}
