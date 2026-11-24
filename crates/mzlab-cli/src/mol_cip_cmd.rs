//! `mzlab-cli mol cip REQUEST.json|- [--out PATH]`: the `cip/1` of a drawing (R/S of its defined centres, E/Z of its defined double bonds),
//! byte for byte what the CIP labels of Draw show on the page and in the app.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol cip: the CIP descriptors of the stereo drawn in a structure (R/S, r/s, E/Z), as JSON (cip/1).

usage: mzlab-cli mol cip REQUEST.json|- [--out PATH]

  REQUEST.json  {\"molfile\": \"...\"}: the V2000 or V3000 molfile of the drawing; - reads the standard input
  --out PATH    write there instead of the standard output

The answer is {\"schema\": \"cip/1\", \"atoms\": [{atom, label}], \"bonds\": [{a, b, label}]} with atoms 0-based in the order of the
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
        return Outcome::fail("mol cip needs one request file (or - for the standard input)");
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
#[path = "mol_cip_cmd_tests.rs"]
mod tests;
