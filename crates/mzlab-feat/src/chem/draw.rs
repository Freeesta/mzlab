//! `draw/1`: a structure given as text (SMILES, molfile, a name, an InChIKey) drawn in 2D by the own layout of `mzlab-mol`, as a molfile or
//! as a scene of atoms, bonds and fragments in angstrom. Names and keys resolve only through a `molecules/1` CSV given in the request; an
//! InChI is never parsed here. Every automatic choice goes to the `choices` of the scene. Same bytes in the molkit `.wasm`, the app
//! (`feat/grafo`, `kind: "draw"`) and `mzlab-cli mol draw`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out, Value};
use mzlab_core::molecules;
use mzlab_mol::Mol;

/// Above this many atoms the drawing refuses (a drawing, not a library).
const MAX_ATOMS: usize = 2000;

/// What the answer is: the molfile text or the `draw/1` scene.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Format {
    Molfile,
    Scene,
}

/// An automatic choice or a warning: a stable key and the value the choice took.
#[derive(Clone, PartialEq, Debug)]
pub struct Note {
    pub key: &'static str,
    pub value: String,
}

/// A drawn structure: its record (with the coordinates of the drawing), the title line and what was decided on the way.
pub struct Drawn {
    pub kind: &'static str,
    pub title: String,
    pub mol: Mol,
    pub choices: Vec<Note>,
    pub warnings: Vec<Note>,
}

struct Request<'a> {
    spec: &'a str,
    format: Format,
    keep: bool,
    db: Option<&'a str>,
    name: Option<&'a str>,
}

pub(super) fn note(key: &'static str, value: impl ToString) -> Note {
    Note {
        key,
        value: value.to_string(),
    }
}

fn read_request(v: &Value) -> Result<Request<'_>> {
    let text = |k: &str| v.get(k).and_then(Value::str);
    let spec = text("spec").filter(|s| !s.trim().is_empty());
    let bad = || Error::new("err.mol.draw.request");
    // a setting with two values: true when it is the second
    let setting = |name: &'static str, ok: [&str; 2], default: &str| -> Result<bool> {
        let want = text(name).unwrap_or(default);
        match ok.iter().position(|o| *o == want) {
            Some(i) => Ok(i == 1),
            None => Err(Error::new("err.mol.draw.set")
                .with("name", name)
                .with("value", want)),
        }
    };
    Ok(Request {
        spec: spec.ok_or_else(bad)?,
        format: if setting("format", ["molfile", "scene"], "molfile")? {
            Format::Scene
        } else {
            Format::Molfile
        },
        keep: !setting("layout", ["keep", "new"], "keep")?,
        db: text("db"),
        name: text("name"),
    })
}

/// The answer to a request `{"spec", "format": "molfile"|"scene", "layout": "keep"|"new", "db": CSV text, "name": title}`.
pub fn draw_json(request: &str) -> Result<String> {
    let (d, format) = draw(request)?;
    Ok(match format {
        Format::Molfile => d.mol.to_molfile(&d.title),
        Format::Scene => scene(&d),
    })
}

/// The drawn structure of a request and the format asked for.
pub fn draw(request: &str) -> Result<(Drawn, Format)> {
    let v = json::parse(request.as_bytes()).map_err(|_| Error::new("err.mol.draw.request"))?;
    let r = read_request(&v)?;
    let (kind, body) = classify(r.spec);
    let mut d = match kind {
        "smiles" => from_smiles(body)?,
        "molfile" => from_molfile(body, r.keep)?,
        _ => super::draw_db::from_db(kind, body, r.db)?,
    };
    too_big(&d.mol)?;
    if kind != "molfile" {
        lay_out(&mut d);
        d.choices.push(note("layout", "new"));
    }
    d.kind = kind;
    d.choices.insert(0, note("spec_kind", kind));
    if let Some(n) = r.name {
        d.title = n.to_string();
    }
    Ok((d, r.format))
}

/// The size limit, checked before any layout work.
fn too_big(m: &Mol) -> Result<()> {
    if m.atoms.len() > MAX_ATOMS {
        return Err(Error::new("err.mol.draw.tooBig").with("max", MAX_ATOMS));
    }
    Ok(())
}

/// Lays the drawing out; when the work budget of the layout ran out the cleaning stopped early and the scene says so.
fn lay_out(d: &mut Drawn) {
    let (c, cut) = d.mol.layout_2d_checked();
    d.mol.coords = Some(c);
    if cut {
        d.choices
            .push(note("layout_budget", "cleaning stopped at the work limit"));
    }
}

/// The kind of a spec (`smiles`, `molfile`, `name`, `inchikey`, `inchi`), as `draw` reads it.
pub fn kind_of(spec: &str) -> &'static str {
    classify(spec).0
}

/// The kind of a spec and the text that follows its prefix. Auto-detection: a molfile text, an InChI, a 27-character key, a text made of
/// SMILES tokens only, otherwise a name.
fn classify(spec: &str) -> (&'static str, &str) {
    let t = spec.trim();
    for (p, kind) in [
        ("smiles:", "smiles"),
        ("name:", "name"),
        ("inchikey:", "inchikey"),
    ] {
        if let Some(r) = t.strip_prefix(p) {
            return (kind, r.trim());
        }
    }
    if let Some(r) = spec.strip_prefix("molfile:") {
        return ("molfile", r);
    }
    if spec.contains("M  END") {
        return ("molfile", spec);
    }
    if t.starts_with("InChI=") {
        // an InChI is never parsed or hashed here: only the InChIKey written beside it can find the molecule
        let key = t
            .split_whitespace()
            .map(|w| w.strip_prefix("InChIKey=").unwrap_or(w))
            .find(|w| molecules::is_inchikey(w));
        return key.map_or(("inchi", t), |k| ("inchikey", k));
    }
    if molecules::is_inchikey(t) {
        return ("inchikey", t);
    }
    if smiles_tokens(t) {
        return ("smiles", t);
    }
    ("name", t)
}

/// Whether a text is made of SMILES tokens only: bracket atoms, the organic subset (`Cl` and `Br` as two letters, aromatic in lower case),
/// bonds, branches, ring digits, `%nn` and dots, with at least one atom.
fn smiles_tokens(t: &str) -> bool {
    let c: Vec<char> = t.chars().collect();
    let (mut i, mut atoms) = (0, 0);
    while i < c.len() {
        match c[i] {
            '[' => {
                let Some(j) = c[i..].iter().position(|&x| x == ']') else {
                    return false;
                };
                i += j;
                atoms += 1;
            }
            'C' if c.get(i + 1) == Some(&'l') => {
                i += 1;
                atoms += 1;
            }
            'B' if c.get(i + 1) == Some(&'r') => {
                i += 1;
                atoms += 1;
            }
            'B' | 'C' | 'N' | 'O' | 'P' | 'S' | 'F' | 'I' | 'b' | 'c' | 'n' | 'o' | 'p' | 's'
            | '*' => atoms += 1,
            '0'..='9'
            | '('
            | ')'
            | '='
            | '#'
            | '$'
            | ':'
            | '/'
            | '\\'
            | '.'
            | '%'
            | '@'
            | '+'
            | '-' => {}
            _ => return false,
        }
        i += 1;
    }
    atoms > 0
}

fn blank(kind: &'static str, mol: Mol) -> Drawn {
    Drawn {
        kind,
        title: String::new(),
        mol,
        choices: Vec::new(),
        warnings: Vec::new(),
    }
}

pub(super) fn from_smiles(text: &str) -> Result<Drawn> {
    let m = Mol::from_smiles(text)
        .map_err(|e| Error::new("err.mol.draw.smiles").with("message", e.to_string()))?;
    Ok(blank("smiles", m))
}

/// A molfile: its coordinates are kept when it has 2D ones that are not all at the origin and `keep` is on; else it is laid out again.
fn from_molfile(text: &str, keep: bool) -> Result<Drawn> {
    let m = Mol::from_molfile(text)
        .map_err(|e| Error::new("err.mol.draw.molfile").with("message", e.to_string()))?;
    let mut d = blank(
        "molfile",
        Mol::new(Vec::new(), Vec::new(), Vec::new(), Vec::new()),
    );
    let version = if text.lines().nth(3).is_some_and(|l| l.contains("V3000")) {
        "V3000"
    } else {
        "V2000"
    };
    d.choices.push(note("molfile_version", version));
    d.title = text.lines().next().unwrap_or("").trim().to_string();
    let flat = m.coords.as_ref().is_some_and(|c| c.dim == 2);
    let spread = m
        .coords
        .as_ref()
        .is_some_and(|c| c.xy.iter().any(|r| r.iter().take(2).any(|v| *v != 0.0)));
    let (why, relay) = match (keep, flat, spread) {
        (false, ..) => ("layout=new", true),
        (_, false, true) => ("3D coordinates", true),
        (_, _, false) => ("all atoms at the origin", true),
        _ => ("2D coordinates of the file", false),
    };
    d.choices.push(note(
        "layout",
        format!("{} ({why})", if relay { "new" } else { "kept" }),
    ));
    d.mol = m;
    too_big(&d.mol)?;
    if relay {
        lay_out(&mut d);
    }
    Ok(d)
}

fn notes(o: &mut Out, key: &str, list: &[Note]) {
    o.raw(",").str(key).raw(":[");
    for (i, n) in list.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"key\":")
            .str(n.key)
            .raw(",\"value\":")
            .str(&n.value)
            .raw("}");
    }
    o.raw("]");
}

/// A coordinate already rounded to 4 decimals, written shortest (`1.299`, `0`) by integer arithmetic: the float formatter of `Out::num`
/// costs the `.wasm` about 14 KB.
fn coord(v: f64) -> String {
    let s = mzlab_mol::fixed(v, 4);
    let t = s.trim_end_matches('0').trim_end_matches('.');
    // a tiny negative rounds to "-0"; the scene writes 0
    if t == "-0" {
        "0".into()
    } else {
        t.into()
    }
}

/// The `draw/1` scene: atoms with their position in angstrom, bonds as written in the molfile (a wedge belongs to its first atom), the
/// connected pieces and what was chosen on the way.
pub fn scene(d: &Drawn) -> String {
    let m = &d.mol;
    let xy = |i: usize, k: usize| {
        m.coords
            .as_ref()
            .and_then(|c| c.xy.get(i))
            .and_then(|r| r.get(k))
            .copied()
            .unwrap_or(0.0)
            + 0.0
    };
    let mut o = Out::default();
    o.raw("{\"schema\":\"draw/1\",\"spec_kind\":")
        .str(d.kind)
        .raw(",\"title\":")
        .str(&d.title)
        .raw(",\"atoms\":[");
    for (i, a) in m.atoms.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"i\":")
            .int(i as i64)
            .raw(",\"el\":")
            .str(&a.el);
        o.raw(",\"chg\":")
            .int(i64::from(a.chg))
            .raw(",\"iso\":")
            .int(i64::from(a.iso))
            .raw(",\"h\":")
            .int(i64::from(a.h));
        o.raw(",\"x_a\":")
            .raw(&coord(xy(i, 0)))
            .raw(",\"y_a\":")
            .raw(&coord(xy(i, 1)))
            .raw("}");
    }
    o.raw("],\"bonds\":[");
    for (i, (a, b, order, wedge)) in m.written_bonds().into_iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"a\":")
            .int(i64::from(a))
            .raw(",\"b\":")
            .int(i64::from(b));
        o.raw(",\"o\":")
            .int(i64::from(order))
            .raw(",\"wedge\":")
            .int(i64::from(wedge))
            .raw("}");
    }
    o.raw("],\"fragments\":[");
    for (i, f) in m.layout_pieces().iter().enumerate() {
        o.raw(if i > 0 { ",[" } else { "[" });
        for (k, a) in f.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).int(*a as i64);
        }
        o.raw("]");
    }
    o.raw("]");
    notes(&mut o, "choices", &d.choices);
    notes(&mut o, "warnings", &d.warnings);
    o.raw("}");
    o.0
}

#[cfg(test)]
#[path = "draw_tests.rs"]
mod tests;
