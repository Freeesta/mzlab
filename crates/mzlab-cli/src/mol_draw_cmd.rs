//! `mzlab-cli mol draw SPEC [--format molfile|scene|ket|cdxml|svg|png] [--set NAME=VALUE...] [--out PATH]`: a structure drawn in 2D by the own
//! layout (`draw/1`), as a molfile, a KET or CDXML text, a picture (SVG text, PNG bytes) or, with `--json`, as a scene of atoms, bonds and fragments
//! (the KET or CDXML text in `file`; a picture wrapped as `draw-image/1`). The same bytes as the molkit `.wasm` and the app (`chem::draw`).

use mzlab_core::error::Error;
use std::path::{Path, PathBuf};

use mzlab_core::json::Out;

use crate::Outcome;
use mzlab_feat::chem::draw::Format;

pub const HELP: &str = "mzlab-cli mol draw: a structure drawn in 2D by mzLab's own layout (draw/1), as a molfile, a scene or a picture.

usage: mzlab-cli mol draw SPEC [--format molfile|scene|ket|cdxml|svg|png] [--set NAME=VALUE...] [--out PATH] [--json]

  SPEC           smiles:TEXT (or a bare SMILES), molfile:TEXT, a molfile path or - (the text has M  END), name:TEXT, inchikey:KEY
                 (or a bare 27-character key), InChI=... followed by its InChIKey. Bare text that is none of these is a name.
  --format F     molfile (default), scene, ket, cdxml, svg or png (= --set format=F); with --json ket or cdxml go into the scene (file),
                 svg or png are wrapped as draw-image/1
  --set layout   keep (default: a molfile with 2D coordinates keeps them) or new (lay it out again)
  --set db       a molecules/1 CSV (for example mzlab/web/molecules/catalogue.csv): names and keys are looked up only there
  --set name     the title of the molfile or picture
  --set scale    pixels per angstrom of svg/png, 8 to 80 (default 24)
  --set cip      on (default) or off: R/S and E/Z marks of the picture
  --set highlight  0,1,0-1@red;5@blue: atoms, bonds a-b, a colour name (red orange yellow green cyan blue purple pink gray)
  --out PATH     write there instead of the standard output

KET and CDXML are open formats written from their public documentation, not tested against ChemDraw or Ketcher.
Angstrom, bond length 1.5; deterministic. Coordinates never decide stereo: wedges come from the stereo of the record. The scene lists
every automatic choice (spec kind, name hit and score, layout kept or new, V2000 or V3000) in `choices`; with a bare file they go to
standard error. The PNG has its own stroke font. An InChI alone is refused: mzLab never parses or hashes one.
Exit codes: 1 over 2000 atoms, an atom that is no element (err.mol.draw.element); 2 unparseable SMILES, bad option, a name or key without --set db; 4 missing file or db, unreadable molfile or request; 5 not in the db; 6 --out.";

const NAMES: [&str; 7] = [
    "format",
    "layout",
    "db",
    "name",
    "scale",
    "cip",
    "highlight",
];
const PREFIXES: [&str; 5] = ["smiles:", "molfile:", "name:", "inchikey:", "InChI="];

/// SPEC, `--out`, `--json` and the `--set` values in the order of `NAMES`.
type Words = (String, Option<PathBuf>, bool, [Option<String>; 7]);

/// The words of the command line.
fn parse(args: Vec<String>) -> Result<Words, Outcome> {
    let (mut pos, mut out, mut json) = (Vec::new(), None::<PathBuf>, false);
    let mut set: [Option<String>; 7] = Default::default();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Err(Outcome::ok(format!("{HELP}\n").into_bytes())),
            "--json" => json = true,
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Err(Outcome::fail("--out needs a path")),
            },
            "--format" => match it.next() {
                Some(f) => set[0] = Some(f),
                None => {
                    return Err(Outcome::fail(
                        "--format needs molfile, scene, ket, cdxml, svg or png",
                    ))
                }
            },
            "--set" => match it.next().as_deref().and_then(|kv| kv.split_once('=')) {
                Some((k, v)) => match NAMES.iter().position(|n| *n == k) {
                    Some(i) => set[i] = Some(v.to_string()),
                    None => {
                        return Err(Outcome::fail(&format!(
                        "unknown --set name {k} (format, layout, db, name, scale, cip, highlight)"
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
    if json && set[0].as_deref().is_some_and(|f| f == "molfile") {
        return Err(Outcome::fail(
            "--json does not go with the molfile format: use scene, ket, cdxml, svg or png",
        ));
    }
    Ok((spec.clone(), out, json, set))
}

/// The request of the engine: SPEC text, the format, the other settings, the database text. `--json` is the scene, or wraps a picture.
fn request(spec: &str, json: bool, set: &[Option<String>; 7], db: Option<&str>) -> String {
    let format = match (set[0].as_deref(), json) {
        (Some(f), _) => f,
        (None, true) => "scene",
        (None, false) => "molfile",
    };
    let mut req = Out::default();
    req.raw("{\"spec\":").str(spec).raw(",\"format\":").str(
        if json && matches!(format, "ket" | "cdxml") {
            "scene"
        } else {
            format
        },
    );
    // a scene carries the KET or CDXML text of the drawing; a picture is wrapped as draw-image/1
    if json && matches!(format, "ket" | "cdxml") {
        req.raw(",\"file\":").str(format);
    } else if json && format != "scene" {
        req.raw(",\"json\":true");
    }
    for (k, v) in NAMES.iter().zip(set).skip(1) {
        if let (Some(v), false) = (v, *k == "db") {
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
    let db = match set[2].as_deref().map(read_db).transpose() {
        Ok(d) => d,
        Err(o) => return o,
    };
    match mzlab_feat::chem::draw::answer(&request(&spec, json, &set, db.as_deref())) {
        Ok(a) => {
            let json_out = a.wrapped || a.format == Format::Scene;
            let mut bytes = a.bytes;
            if json_out {
                bytes.push(b'\n');
            }
            let mut o = crate::emit(bytes, &out);
            if o.code == 0 && !json_out {
                for (label, list) in [("choice", &a.drawn.choices), ("warning", &a.drawn.warnings)]
                {
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
        "err.mol.draw.needDb" => o.with_hint("names and keys are looked up only in a molecules/1 CSV: add --set db=PATH (for example mzlab/web/molecules/catalogue.csv)"),
        "err.mol.draw.not_found" if !param("nearest").is_empty() => o.with_hint(&format!("nearest names: {}", param("nearest"))),
        "err.mol.draw.inchi" => o.with_hint("write the InChIKey: inchikey:KEY, or the InChI followed by its InChIKey"),
        _ => o,
    }
}

/// Most bytes read for a molfile (a 2000-atom one is about 200 KB) and for a `--set db` CSV (the shipped catalog is 360 KB).
const MAX_MOLFILE: u64 = 8 << 20;
const MAX_DB: u64 = 64 << 20;

/// Why a text was not read.
#[derive(Debug, PartialEq)]
enum NotRead {
    TooBig(u64),
    NotText,
    Unreadable,
}

/// The text of `r` up to `max` bytes.
fn read_capped(r: impl std::io::Read, max: u64) -> Result<String, NotRead> {
    let mut t = String::new();
    std::io::Read::read_to_string(&mut r.take(max + 1), &mut t).map_err(|e| {
        if e.kind() == std::io::ErrorKind::InvalidData {
            NotRead::NotText
        } else {
            NotRead::Unreadable
        }
    })?;
    if t.len() as u64 > max {
        return Err(NotRead::TooBig(max));
    }
    Ok(t)
}

/// The words of a failed read for a message about `what`.
fn why(r: &NotRead, what: &str) -> String {
    match r {
        NotRead::TooBig(max) => format!("{what} is more than {max} bytes"),
        _ => format!("{what} is not UTF-8 text"),
    }
}

fn molfile_error(why: &str) -> Outcome {
    Outcome::fail_error(&Error::new("err.mol.draw.molfile").with("message", why))
}

fn unreadable(path: &str) -> Outcome {
    Outcome::fail_error(&Error::new("err.cli.file").with("path", path))
}

/// The `--set db=PATH` CSV: a missing or unreadable file is `err.cli.file` (exit 4), an oversized or non-UTF-8 one `err.mol.draw.db` (exit 4).
fn read_db(path: &str) -> Result<String, Outcome> {
    let file = std::fs::File::open(path).map_err(|_| unreadable(path))?;
    read_capped(file, MAX_DB).map_err(|e| match e {
        NotRead::Unreadable => unreadable(path),
        _ => Outcome::fail_error(&Error::new("err.mol.draw.db").with("message", why(&e, path))),
    })
}

/// The text the library reads for `spec`: itself, or the molfile it names (`-` is the standard input). A path that is not there is an
/// unreadable file (exit 4), not a name.
fn resolve(spec: &str) -> Result<String, Outcome> {
    if spec == "-" {
        return read_capped(std::io::stdin(), MAX_MOLFILE).map_err(|e| match e {
            NotRead::Unreadable => unreadable("-"),
            _ => molfile_error(&why(&e, "the standard input")),
        });
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
            Err(Outcome::fail_error(
                &Error::new("err.cli.file").with("path", spec),
            ))
        } else {
            Ok(spec.to_string())
        };
    }
    let file = std::fs::File::open(spec).map_err(|_| unreadable(spec))?;
    let text = read_capped(file, MAX_MOLFILE).map_err(|e| match e {
        NotRead::Unreadable => unreadable(spec),
        _ => molfile_error(&why(&e, spec)),
    })?;
    if text.contains("M  END") {
        return Ok(text);
    }
    if pathlike {
        return Err(molfile_error(&format!("{spec} has no M  END line")));
    }
    Ok(spec.to_string())
}

#[cfg(test)]
#[path = "mol_draw_tests.rs"]
mod tests;
