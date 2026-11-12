//! `mzlab-cli mol-mech REQUEST.json|- [--out PATH]`: the `mecc/1` check of the curved arrows of a mechanism drawn on a structure (each arrow starts at
//! a lone pair, a bond or a radical and ends at an atom or a bond; drawn lone pairs fit the atom), byte for byte what the Draw page and the app show.

use std::path::PathBuf;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol-mech: whether the curved arrows of a mechanism are well formed, as JSON (mecc/1). Form only: it never says
which mechanism is right.

usage: mzlab-cli mol-mech REQUEST.json|- [--out PATH]

  REQUEST.json  {\"molfile\": V2000 text, \"arrows\": [{\"kind\": \"pair\"|\"single\", \"from\": {\"lone\"|\"radical\": ATOM} or {\"bond\": [A, B]},
                \"to\": {\"atom\": ATOM} or {\"bond\": [A, B]}}, ...], \"lone\": [[ATOM, PAIRS], ...]}: atoms are 0-based positions of the
                molfile; pair = full head (two electrons), single = half head (one); - reads the standard input
  --out PATH    write there instead of the standard output

The answer is {\"format\": \"mecc/1\", \"arrows\": [{kind, from, to, electrons, ok, problem?}], \"lone\": [{atom, pairs, max?, ok}],
\"ok\"}; problem is noLonePair, noRadical, pairFromRadical, noBond or same.";

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
        return Outcome::fail("mol-mech needs one request file (or - for the standard input)");
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
    match mzlab_feat::chem::mech::mech_json(&text) {
        Ok(j) => crate::emit(format!("{j}\n"), &out),
        Err(e) => Outcome::fail(&e.to_string()),
    }
}

#[cfg(test)]
#[path = "mol_mech_tests.rs"]
mod tests;
