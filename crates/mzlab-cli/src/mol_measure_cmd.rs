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
#[path = "mol_measure_cmd_tests.rs"]
mod tests;
