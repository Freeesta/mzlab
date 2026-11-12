//! `mzlab-cli mol-suspects REQUEST.json|- [--out PATH]`: the `tray/1` of the structure tray of Draw (formula and mass of drawn structures, the
//! suspect list as CSV, a list read back), byte for byte what the Draw page and the app write.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-suspects: the structure tray of Draw as a suspect list (tray/1). The list holds candidates to look for,
never what a sample contains.

usage: mzlab-cli mol-suspects REQUEST.json|- [--out PATH]

  REQUEST.json  {\"op\": \"entries\", \"structures\": [{\"name\", \"smiles\", \"atoms\": [{\"el\", \"iso\", \"h\", \"q\", \"r\"}, ...]}]}: formula,
                charge and monoisotopic mass of each structure;
                {\"op\": \"csv\", \"entries\": [{\"name\", \"smiles\", \"formula\", \"mass\", \"charge\"}], \"adducts\": [\"[M+H]+\", ...]}: the
                suspect list as CSV (name,smiles,formula,mass,charge,adducts; the columns screen.suspects and mzFinder read);
                {\"op\": \"read\", \"text\": CSV}: the entries of a list back, with its adducts and the rows skipped without a mass;
                {\"op\": \"adducts\"}: the adducts of the m/z calculator a list may name; - reads the standard input
  --out PATH    write there instead of the standard output";

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
        return Outcome::fail("mol-suspects needs one request file (or - for the standard input)");
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
    // the CSV ends with its own line break; a JSON answer gets one, as every mol-* command
    match mzlab_feat::chem::tray::tray_text(&text) {
        Ok(j) if j.ends_with('\n') => crate::emit(j, &out),
        Ok(j) => crate::emit(format!("{j}\n"), &out),
        Err(e) => Outcome::fail(&e.to_string()),
    }
}

#[cfg(test)]
#[path = "mol_suspects_tests.rs"]
mod tests;
