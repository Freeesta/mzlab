//! `mzlab-cli mol-link REQUEST.json|- [--out PATH]`: the `link/1` of a structure (the PubChem, ChEBI and BioTransformer URLs, percent-encoded),
//! byte for byte what the lookup rows of the Draw right-click menu open in the page and the app.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-link: the outside lookups of a drawn structure (PubChem, ChEBI, BioTransformer) as JSON (link/1).

usage: mzlab-cli mol-link REQUEST.json|- [--out PATH]

  REQUEST.json  {\"smiles\": text, \"inchi\": text (optional), \"part\": true|false}: part = a selection;
                - reads the standard input
  --out PATH    write there instead of the standard output

The answer is {\"format\": \"link/1\", \"part\", \"preferred\": the PubChem row to put first, \"links\": [{\"id\", \"url\", \"copy\"}]};
copy is the text to put on the clipboard before opening url (null when the URL carries the structure). Nothing is fetched.";

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
        return Outcome::fail("mol-link needs one request file (or - for the standard input)");
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
    match mzlab_feat::chem::link::link_json(&text) {
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
        let req = r#"{"smiles":"C#N","inchi":"InChI=1S/CHN/c1-2/h1H","part":true}"#;
        let p = std::env::temp_dir().join(format!("mol_link_{}.json", std::process::id()));
        std::fs::write(&p, req).unwrap();
        let o = run(&["mol-link", p.to_str().unwrap()]);
        std::fs::remove_file(&p).ok();
        assert_eq!(o.code, 0);
        let want = mzlab_feat::chem::link::link_json(req).unwrap();
        assert!(want.contains("InChI%3D1S%2FCHN"), "{want}");
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["mol-link"],
            vec!["mol-link", "a", "b"],
            vec!["mol-link", "--nope"],
            vec!["mol-link", "/no/such/file.json"],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["mol-link", "--help"]).code, 0);
    }
}
