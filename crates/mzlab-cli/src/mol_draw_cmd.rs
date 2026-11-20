//! `mzlab-cli mol draw SPEC [--set NAME=VALUE...] [--out PATH]`: a structure drawn in 2D by the own layout (`draw/1`), as a molfile or, with
//! `--json`, as a scene of atoms, bonds and fragments. The same bytes as the molkit `.wasm` and the app (`chem::draw`).

use std::path::{Path, PathBuf};

use mzlab_core::json::Out;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli mol draw: a structure drawn in 2D by mzLab's own layout (draw/1).

usage: mzlab-cli mol draw SPEC [--set NAME=VALUE...] [--out PATH] [--json]

  SPEC           smiles:TEXT (or a bare SMILES), molfile:TEXT, a molfile path or - (the text has M  END), name:TEXT, inchikey:KEY
                 (or a bare 27-character key), InChI=... followed by its InChIKey. Bare text that is none of these is a name.
  --set format   molfile (default) or scene; --json is the scene (draw/1)
  --set layout   keep (default: a molfile with 2D coordinates keeps them) or new (lay it out again)
  --set db       a molecules/1 CSV (for example mzlab/web/molecole/catalogo.csv): names and keys are looked up only there
  --set name     the title line of the molfile (default: the molecule's name, or the title of the molfile)
  --out PATH     write there instead of the standard output

Angstrom, bond length 1.5; deterministic. Coordinates never decide stereo: wedges come from the stereo of the record. The scene lists
every automatic choice (spec kind, name hit and score, layout kept or new, V2000 or V3000) in `choices`; with a molfile they go to
standard error. An InChI alone is refused: mzLab never parses or hashes one.
Exit codes: 1 over 2000 atoms; 2 unparseable SMILES or molfile, bad option, a name or key without --set db; 4 missing file or db; 5 not in the db; 6 --out.";

const NAMES: [&str; 4] = ["format", "layout", "db", "name"];
const PREFIXES: [&str; 5] = ["smiles:", "molfile:", "name:", "inchikey:", "InChI="];

/// The words of the command line: SPEC, `--out`, `--json` and the four `--set` values (format, layout, db, name).
fn parse(
    args: Vec<String>,
) -> Result<(String, Option<PathBuf>, bool, [Option<String>; 4]), Outcome> {
    let (mut pos, mut out, mut json) = (Vec::new(), None::<PathBuf>, false);
    let mut set: [Option<String>; 4] = Default::default();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Err(Outcome::ok(format!("{HELP}\n").into_bytes())),
            "--json" => json = true,
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Err(Outcome::fail("--out needs a path")),
            },
            "--set" => match it.next().as_deref().and_then(|kv| kv.split_once('=')) {
                Some((k, v)) => match NAMES.iter().position(|n| *n == k) {
                    Some(i) => set[i] = Some(v.to_string()),
                    None => {
                        return Err(Outcome::fail(&format!(
                            "unknown --set name {k} (format, layout, db, name)"
                        )))
                    }
                },
                None => return Err(Outcome::fail("--set needs NAME=VALUE")),
            },
            o if o.starts_with("--") => return Err(Outcome::fail(&format!("unknown option {o}"))),
            _ => pos.push(a),
        }
    }
    let [spec] = pos.as_slice() else {
        return Err(Outcome::fail(
            "mol draw needs one SPEC (smiles:..., a molfile path, name:..., inchikey:...)",
        ));
    };
    if json && set[0].as_deref().is_some_and(|f| f != "scene") {
        return Err(Outcome::fail(
            "--json is the scene: it does not go with --set format=molfile",
        ));
    }
    Ok((spec.clone(), out, json, set))
}

/// The request of the engine: SPEC text, the format, the layout and name choices, the database text.
fn request(spec: &str, json: bool, set: &[Option<String>; 4], db: Option<&str>) -> String {
    let mut req = Out::default();
    req.raw("{\"spec\":")
        .str(spec)
        .raw(",\"format\":")
        .str(if json {
            "scene"
        } else {
            set[0].as_deref().unwrap_or("molfile")
        });
    for (k, v) in [("layout", &set[1]), ("name", &set[3])] {
        if let Some(v) = v {
            req.raw(",").str(k).raw(":").str(v);
        }
    }
    if let Some(d) = db {
        req.raw(",\"db\":").str(d);
    }
    req.raw("}");
    req.0
}

pub fn run(args: Vec<String>) -> Outcome {
    let (spec, out, json, set) = match parse(args) {
        Ok(p) => p,
        Err(o) => return o,
    };
    let spec = match resolve(&spec) {
        Ok(s) => s,
        Err(o) => return o,
    };
    let db = match set[2]
        .as_deref()
        .map(|p| std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}")))
        .transpose()
    {
        Ok(d) => d,
        Err(e) => return Outcome::fail(&e),
    };
    match mzlab_feat::chem::draw::draw(&request(&spec, json, &set, db.as_deref())) {
        Ok((d, f)) => {
            let text = match f {
                mzlab_feat::chem::draw::Format::Scene => {
                    format!("{}\n", mzlab_feat::chem::draw::scene(&d))
                }
                mzlab_feat::chem::draw::Format::Molfile => d.mol.to_molfile(&d.title),
            };
            let mut o = crate::emit(text, &out);
            if o.code == 0 && f == mzlab_feat::chem::draw::Format::Molfile {
                for (label, list) in [("choice", &d.choices), ("warning", &d.warnings)] {
                    o.stderr += &list
                        .iter()
                        .map(|n| format!("{label}: {} = {}\n", n.key, n.value))
                        .collect::<String>();
                }
            }
            o
        }
        Err(e) => hinted(Outcome::fail_error(&e), &e),
    }
}

/// The failure with the hint that names the way out.
fn hinted(o: Outcome, e: &mzlab_core::error::Error) -> Outcome {
    let param = |k: &str| {
        e.params
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    };
    match e.key {
        "err.mol.draw.needDb" => o.with_hint("names and keys are looked up only in a molecules/1 CSV: add --set db=PATH (for example mzlab/web/molecole/catalogo.csv)"),
        "err.mol.draw.not_found" if !param("nearest").is_empty() => o.with_hint(&format!("nearest names: {}", param("nearest"))),
        "err.mol.draw.inchi" => o.with_hint("write the InChIKey: inchikey:KEY, or the InChI followed by its InChIKey"),
        _ => o,
    }
}

/// The text the library reads for `spec`: itself, or the molfile it names (`-` is the standard input). A path that is not there is an
/// unreadable file (exit 4), not a name.
fn resolve(spec: &str) -> Result<String, Outcome> {
    if spec == "-" {
        let mut t = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut t)
            .map_err(|e| Outcome::fail(&e.to_string()))?;
        return Ok(t);
    }
    if PREFIXES.iter().any(|p| spec.starts_with(p)) {
        return Ok(spec.to_string());
    }
    let pathlike = spec.starts_with('/')
        || spec.starts_with("./")
        || spec.starts_with("../")
        || [".mol", ".sdf", ".mdl"].iter().any(|e| spec.ends_with(e));
    if !Path::new(spec).is_file() {
        let guess = pathlike && mzlab_feat::chem::draw::kind_of(spec) == "name";
        return if guess {
            Err(Outcome::fail(&format!(
                "{spec}: No such file or directory (os error 2)"
            )))
        } else {
            Ok(spec.to_string())
        };
    }
    let text = std::fs::read_to_string(spec).map_err(|e| Outcome::fail(&format!("{spec}: {e}")))?;
    Ok(if text.contains("M  END") || pathlike {
        text
    } else {
        spec.to_string()
    })
}

#[cfg(test)]
#[path = "mol_draw_tests.rs"]
mod tests;
