//! `mzlab-cli mol measure REQUEST.json|- [--out PATH]`: the `descriptors/1` of a drawing (Crippen logP and MR, TPSA, donors, acceptors,
//! rotatable bonds, average weight and the `analysis/1` of the formula), the bytes of `mzlab_feat::chem::descriptors::descriptors_json`, the
//! function mzDraw, the molkit `.wasm` and the app call.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol measure: logP, MR, TPSA, H-bond donors and acceptors, rotatable bonds, average weight and the formula analysis of a drawn structure, as JSON (descriptors/1).

usage: mzlab-cli mol measure REQUEST.json|- [--out PATH]

  REQUEST.json  {\"molfile\": \"...\"}: the V2000 or V3000 molfile of the drawing; - reads the standard input
  --out PATH    write there instead of the standard output

The answer is {\"schema\": \"descriptors/1\", \"logp\", \"mr\" (Wildman-Crippen), \"tpsa\" (Ertl, N and O), \"hbd\", \"hba\", \"rotors\",
\"mw\" (conventional atomic weights, null when an element has none), \"analysis\": {the analysis/1 of mol anal}}, 4 decimals.
Over 2000 atoms is exit 1 (error.descr.tooBig); a file that is no molfile is exit 4 (error.descr.molfile). Theory for the drawn structure:
nothing is fitted to data.";

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
        return Outcome::fail("mol measure needs one request file (or - for the standard input)");
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
    match mzlab_feat::chem::descriptors::descriptors_json(&text) {
        Ok(j) => crate::emit(format!("{j}\n"), &out),
        Err(e) => Outcome::fail_error(&e),
    }
}

#[cfg(test)]
mod tests {
    use crate::execute;

    fn run(a: &[&str]) -> crate::Outcome {
        execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    fn tmp(name: &str, body: &str) -> String {
        let p = std::env::temp_dir().join(format!("mol_descr_{}_{name}", std::process::id()));
        std::fs::write(&p, body).unwrap();
        p.to_str().unwrap().to_string()
    }

    #[test]
    fn the_command_prints_the_function_json() {
        let fx: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/dati/descriptors_disegno.json"
            ))
            .unwrap(),
        )
        .unwrap();
        for c in fx["cases"].as_array().unwrap() {
            let req = c["request"].to_string();
            let p = tmp("req.json", &req);
            let o = run(&["mol", "measure", &p]);
            assert_eq!(o.code, 0, "{}", o.stderr);
            let want = mzlab_feat::chem::descriptors::descriptors_json(&req).unwrap();
            assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
            assert_eq!(want, c["descriptors"].as_str().unwrap(), "{}", c["name"]);
        }
    }

    #[test]
    fn errors_have_their_exit_codes() {
        for a in [
            vec!["mol", "measure"],
            vec!["mol", "measure", "a", "b"],
            vec!["mol", "measure", "--nope"],
            vec!["mol", "measure", "/no/such/file.json"],
        ] {
            assert_eq!(run(&a).code, crate::exit::usage_or_missing(&a), "{a:?}");
        }
        assert_eq!(run(&["mol", "measure", "--help"]).code, 0);
        let bad = tmp("bad.json", r#"{"molfile":"nonsense"}"#);
        let o = run(&["mol", "measure", &bad, "--json"]);
        assert_eq!(o.code, 4);
        let e: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(e["error_key"], "error.descr.molfile");
        let not_json = tmp("not.json", "{");
        assert_eq!(run(&["mol", "measure", &not_json]).code, 4);
    }
}
