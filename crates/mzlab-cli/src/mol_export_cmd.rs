//! `mzlab-cli mol-export MOLFILE|- [--set format=sdf|mol] [--set ion=ADDUCT] [--set name=TEXT] [--out PATH]`: the drawing as a plain
//! molfile or SDF (`sdf/1`), byte for byte the «Export» MOL/SDF of the Draw page and of the app.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-export: a drawing as a plain .mol or .sdf file (sdf/1).

usage: mzlab-cli mol-export MOLFILE|- [--set NAME=VALUE...] [--out PATH]

  MOLFILE        a V2000 or V3000 molfile (- reads the standard input)
  --set format   sdf (default: one record per connected structure, with data items) or mol (the whole drawing in one record)
  --set ion      none (default) or an adduct: [M+H]+ [M+NH4]+ [M+Na]+ [M+K]+ [M-H]- [M+Cl]- [M+HCOO]-
  --set name     the title line of every record (default empty)
  --out PATH     write there instead of the standard output

Every SDF record has the data items MZLAB_FORMULA, MZLAB_EXACT_MASS, MZLAB_ION and MZLAB_MZ (the adduct of a neutral structure, or
\"drawn\" for a charged one; none without an ion) and MZLAB_HASH (the mol1: id of the structure). V2000, or V3000 above 999 atoms or
bonds or with enhanced-stereo groups. The m/z is a candidate to look for, not an identification.";

const NAMES: [&str; 3] = ["format", "ion", "name"];

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out) = (Vec::new(), None);
    let mut set = [String::from("sdf"), String::from("none"), String::new()];
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            "--set" => match it.next().as_deref().and_then(|kv| kv.split_once('=')) {
                Some((k, v)) => match NAMES.iter().position(|n| *n == k) {
                    Some(i) => set[i] = v.to_string(),
                    None => return Outcome::fail(&format!("unknown --set name {k} (format, ion, name)")),
                },
                None => return Outcome::fail("--set needs NAME=VALUE"),
            },
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    let [src] = pos.as_slice() else {
        return Outcome::fail("mol-export needs one molfile (or - for the standard input)");
    };
    let text = match read(src) {
        Ok(t) => t,
        Err(e) => return Outcome::fail(&e),
    };
    match mzlab_feat::chem::sdf::export(&text, &set[0], &set[1], &set[2]) {
        Ok(t) => crate::emit(t, &out),
        Err(e) => Outcome::fail(&e.to_string()),
    }
}

/// The molfile text of a path, or of the standard input for `-`.
fn read(src: &str) -> Result<String, String> {
    if src != "-" {
        return std::fs::read_to_string(src).map_err(|e| format!("{src}: {e}"));
    }
    let mut t = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut t).map_err(|e| e.to_string())?;
    Ok(t)
}

#[cfg(test)]
#[path = "mol_export_tests.rs"]
mod tests;
