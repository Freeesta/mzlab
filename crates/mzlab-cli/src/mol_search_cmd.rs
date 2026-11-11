//! `mzlab-cli mol-search --sub QUERY TARGET... [--set stereo=1] [--set max=N] [--out PATH]`: where a drawn moiety occurs in the given
//! structures (`sub/1`), byte for byte what the `.wasm` `sub` and the desktop route `feat/grafo` (`kind: "sub"`) answer.

use std::path::PathBuf;

use mzlab_core::json::Out;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-search: where a substructure occurs in structures, as JSON (sub/1).

usage: mzlab-cli mol-search --sub QUERY TARGET... [--set NAME=VALUE...] [--out PATH]

  QUERY           the substructure: a molfile (V2000/V3000, with atom lists, generic atoms A Q X * and query bonds) or a
                  SMILES; a path to a file or the text itself (- reads the standard input)
  TARGET          one or more structures, each a path to a molfile or a SMILES file, or a SMILES
  --set stereo    1 = defined stereo centres and double bonds of the query must be the same in the target (default 0)
  --set max       at most this many matches per target (1-1000, default 1000)
  --out PATH      write there instead of the standard output

The answer is {\"format\": \"sub/1\", \"atoms\", \"stereo\", \"hits\": [{i, n, maps}], \"errors\": [{i, error}]}: for every target that
contains the query, its distinct matches as the target atoms (from 0) of each query atom; explicit query hydrogens ask for at least that
many hydrogens and have no column. A hit is a suspect to check, never an identification.";

/// The text of an argument: a file when one exists at that path, the standard input for -, else the argument itself.
fn text(arg: &str) -> Result<String, String> {
    if arg == "-" {
        let mut t = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut t).map_err(|e| e.to_string())?;
        return Ok(t);
    }
    let p = std::path::Path::new(arg);
    if p.is_file() {
        return std::fs::read_to_string(p).map_err(|e| format!("{arg}: {e}"));
    }
    Ok(arg.to_string())
}

/// The request of the `.wasm` from the command line.
fn request(query: &str, targets: &[String], sets: &[(String, String)]) -> Result<String, String> {
    let mut o = Out::default();
    o.raw("{\"query\":")
        .str(&text(query)?)
        .raw(",\"targets\":[");
    for (k, t) in targets.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(&text(t)?);
    }
    o.raw("]");
    for (k, v) in sets {
        match k.as_str() {
            "stereo" => match v.as_str() {
                "1" | "true" => o.raw(",\"stereo\":true"),
                "0" | "false" => o.raw(",\"stereo\":false"),
                _ => return Err(format!("--set stereo: {v} is not 0 or 1")),
            },
            "max" => match v.parse::<u32>() {
                Ok(n) if (1..=1000).contains(&n) => o.raw(",\"max\":").int(i64::from(n)),
                _ => return Err(format!("--set max: {v} is not a number from 1 to 1000")),
            },
            _ => return Err(format!("unknown --set name {k} (stereo, max)")),
        };
    }
    o.raw("}");
    Ok(o.0)
}

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out, mut sets, mut sub) = (Vec::new(), None, Vec::new(), false);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--sub" => sub = true,
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
    if !sub {
        return Outcome::fail("mol-search needs --sub (the substructure search)");
    }
    let Some((query, targets)) = pos.split_first().filter(|(_, t)| !t.is_empty()) else {
        return Outcome::fail("mol-search --sub needs a query and at least one target");
    };
    let req = match request(query, targets, &sets) {
        Ok(r) => r,
        Err(e) => return Outcome::fail(&e),
    };
    match mzlab_feat::chem::substructure::search_json(&req) {
        Ok(j) => crate::emit(format!("{j}\n"), &out),
        Err(e) => Outcome::fail(&e.to_string()),
    }
}

#[cfg(test)]
#[path = "mol_search_tests.rs"]
mod tests;
