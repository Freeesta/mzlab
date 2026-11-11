//! `mzlab-cli mol-cut MOLFILE|- --set bonds=A-B[,C-D...] [--set mode=...] [--set cation=...] [--set gap=N] [--out PATH]`: the `cut/1` of a cut across
//! bonds of a drawn structure (homolytic, heterolytic or «+H»), byte for byte what the cut tool of the Draw page and of the app computes.

use std::path::PathBuf;

use mzlab_core::json::Out;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-cut: the pieces of a cut across bonds of a structure, as JSON (cut/1).

usage: mzlab-cli mol-cut MOLFILE|- --set bonds=A-B[,C-D...] [--set NAME=VALUE...] [--out PATH]

  MOLFILE         a V2000 molfile (- reads the standard input); atoms are numbered from 0 in file order
  --set bonds     the cut bonds as atom pairs, e.g. 0-1,4-5 (several bonds of a ring give two pieces)
  --set mode      homolytic (default: each side keeps one electron, radical marks), heterolytic (+ on one side, - on the other,
                  single bonds only) or hydrogen (each side closes with one H per bond order)
  --set cation    heterolytic: the atom of each cut bond that takes the +, in the order of bonds (default the first atom of each pair)
  --set gap       how far the pieces move apart, in bond lengths (default 1)
  --out PATH      write there instead of the standard output

The answer is {\"format\": \"cut/1\", \"mode\", \"atoms\": [{i, charge, radical, h, pin}], \"pieces\": [{atoms, shift, label}]}:
every piece with formula, charge, exact mass, m/z (charged pieces only), unpaired electrons, electron count and kind. The pieces are
candidates to judge; no cut is ranked.";

/// The request text of the page from the `--set` values.
fn request(molfile: &str, sets: &[(String, String)]) -> Result<String, String> {
    let mut o = Out::default();
    o.raw("{\"molfile\":").str(molfile);
    for (k, v) in sets {
        let list = |sep: char| -> Result<Vec<Vec<i64>>, String> {
            v.split(',')
                .filter(|t| !t.trim().is_empty())
                .map(|t| {
                    t.split(sep)
                        .map(|n| {
                            n.trim().parse::<i64>().map_err(|_| {
                                format!("--set {k}: {v} is not a list of atom numbers")
                            })
                        })
                        .collect()
                })
                .collect()
        };
        match k.as_str() {
            "bonds" => {
                let pairs = list('-')?;
                if pairs.iter().any(|p| p.len() != 2) {
                    return Err(format!("--set bonds: {v} is not a list of pairs A-B"));
                }
                o.raw(",\"bonds\":[");
                for (n, p) in pairs.iter().enumerate() {
                    o.raw(if n > 0 { ",[" } else { "[" })
                        .int(p[0])
                        .raw(",")
                        .int(p[1])
                        .raw("]");
                }
                o.raw("]");
            }
            "cation" => {
                o.raw(",\"cation\":[");
                for (n, p) in list('-')?.iter().enumerate() {
                    o.raw(if n > 0 { "," } else { "" })
                        .int(*p.first().unwrap_or(&0));
                }
                o.raw("]");
            }
            "mode" => {
                o.raw(",\"mode\":").str(v);
            }
            "gap" => {
                let g: f64 = v
                    .parse()
                    .map_err(|_| format!("--set gap: {v} is not a number"))?;
                o.raw(",\"gap\":").num(g);
            }
            _ => {}
        }
    }
    o.raw("}");
    Ok(o.0)
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
    let [src] = pos.as_slice() else {
        return Outcome::fail("mol-cut needs one molfile (or - for the standard input)");
    };
    if let Some((k, _)) = sets
        .iter()
        .find(|(k, _)| !["bonds", "mode", "cation", "gap"].contains(&k.as_str()))
    {
        return Outcome::fail(&format!(
            "unknown --set name {k} (bonds, mode, cation, gap)"
        ));
    }
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
    let req = match request(&text, &sets) {
        Ok(r) => r,
        Err(e) => return Outcome::fail(&e),
    };
    match mzlab_feat::chem::cut::cut_json(&req) {
        Ok(j) => crate::emit(format!("{j}\n"), &out),
        Err(e) => Outcome::fail(&e.to_string()),
    }
}

#[cfg(test)]
#[path = "mol_cut_tests.rs"]
mod tests;
