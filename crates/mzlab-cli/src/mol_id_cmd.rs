//! `mzlab-cli mol-id MOLFILE|- [--set smiles=TEXT] [--set catalogue=CSV] [--out PATH]`: is a drawn structure in the molecule catalogue (`molid/1`),
//! byte for byte what «Is this in the catalogue?» of the Draw page and of the app answers. The catalogue of the site is built in.

use std::path::PathBuf;

use mzlab_core::json::Out;

use crate::Outcome;

/// The catalogue the site serves (`static/molecole/catalogo.csv`), so the command answers as the page does with no file given.
const CATALOGUE: &str = include_str!("../../../mzlab/web/molecole/catalogo.csv");

pub const HELP: &str = "mzlab-cli mol-id: is a structure in the molecule catalogue, as JSON (molid/1).

usage: mzlab-cli mol-id MOLFILE|- [--set catalogue=CSV] [--out PATH]
       mzlab-cli mol-id --set smiles=TEXT [--set catalogue=CSV] [--out PATH]

  MOLFILE          a V2000 or V3000 molfile (- reads the standard input)
  --set smiles     a SMILES in place of the molfile
  --set catalogue  a catalogue CSV in the format of the site (default: the catalogue built in)
  --out PATH       write there instead of the standard output

The answer is {\"format\": \"molid/1\", \"key\", \"skeleton\", \"compared\", \"exact\": [{id, nome, formula, carica, massa, smiles}],
\"near\": [{..., differs: [stereo|isotopes]}]}: exact = the same structure (any atom order or Kekulé form), near = the same skeleton
with other stereo or isotopes. It names the drawing, never a measured signal.";

fn read(src: &str) -> Result<String, String> {
    if src == "-" {
        let mut t = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut t).map_err(|e| e.to_string())?;
        Ok(t)
    } else {
        std::fs::read_to_string(src).map_err(|e| format!("{src}: {e}"))
    }
}

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out, mut sets) = (Vec::new(), None, Vec::new());
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            "--set" => match it.next().as_deref().and_then(|kv| kv.split_once('=')) {
                Some((k, v)) => sets.push((k.to_string(), v.to_string())),
                None => return Outcome::fail("--set needs NAME=VALUE"),
            },
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    if let Some((k, _)) = sets
        .iter()
        .find(|(k, _)| !["smiles", "catalogue"].contains(&k.as_str()))
    {
        return Outcome::fail(&format!("unknown --set name {k} (smiles, catalogue)"));
    }
    let set = |n: &str| {
        sets.iter()
            .rev()
            .find(|(k, _)| k == n)
            .map(|(_, v)| v.clone())
    };
    let smiles = set("smiles");
    let mut o = Out::default();
    match (pos.as_slice(), &smiles) {
        ([src], None) => match read(src) {
            Ok(t) => o.raw("{\"molfile\":").str(&t),
            Err(e) => return Outcome::fail(&e),
        },
        ([], Some(s)) => o.raw("{\"smiles\":").str(s),
        _ => {
            return Outcome::fail(
                "mol-id needs one molfile (or - for the standard input) or --set smiles=TEXT",
            )
        }
    };
    let cat = match set("catalogue") {
        Some(p) => match read(&p) {
            Ok(t) => t,
            Err(e) => return Outcome::fail(&e),
        },
        None => CATALOGUE.to_string(),
    };
    o.raw(",\"catalogo\":").str(&cat).raw("}");
    match mzlab_feat::chem::identity::identity_json(&o.0) {
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
        let o = run(&["mol-id", "--set", "smiles=Oc1ccc(NC(C)=O)cc1"]);
        assert_eq!(o.code, 0);
        let req = mzlab_core::json::Out::default()
            .raw("{\"smiles\":")
            .str("Oc1ccc(NC(C)=O)cc1")
            .raw(",\"catalogo\":")
            .str(super::CATALOGUE)
            .raw("}")
            .0
            .clone();
        let want = mzlab_feat::chem::identity::identity_json(&req).unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
        assert!(want.contains("\"nome\":\"acetaminophen\""));
    }

    #[test]
    fn usage_errors_exit_2() {
        for a in [
            vec!["mol-id"],
            vec!["mol-id", "a", "b"],
            vec!["mol-id", "a.mol", "--set", "smiles=C"],
            vec!["mol-id", "--set", "nope=1"],
            vec!["mol-id", "/no/such/file.mol"],
            vec!["mol-id", "--set", "smiles="],
        ] {
            assert_eq!(run(&a).code, 2, "{a:?}");
        }
        assert_eq!(run(&["mol-id", "--help"]).code, 0);
    }
}
