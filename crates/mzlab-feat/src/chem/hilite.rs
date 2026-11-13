//! `hilite/1`: the atoms and bonds of the canvas that a piece of evidence points at, layer by layer, for the colours of Draw. A `moiety` layer is a
//! substructure found in the drawing; `sites` (atoms) and `cuts` (bonds) come numbered on the SMILES the evidence was computed on (`reatt/1`, a cut of
//! the MSn graph) and are carried onto the drawing through a match of that SMILES. In the LR world only what the student selected is kept. The colours
//! show where the evidence is, never that it is true. Same bytes in the molkit `.wasm`, the app (`feat/grafo`, `kind: "hilite"`) and `mzlab-cli mol-hilite`.

use crate::sortc::SortC;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out, Value};
use mzlab_mol::{Mol, SubOptions, SubQuery};

/// Above this many atoms on the canvas nothing is coloured (small molecules only).
const MAX_ATOMS: usize = 400;
/// Most layers in one request.
const MAX_LAYERS: usize = 16;

/// One layer of the answer: its atoms and bonds on the canvas (sorted), or why it is empty.
struct Layer<'a> {
    kind: &'a str,
    label: &'a str,
    atoms: Vec<u32>,
    bonds: Vec<(u32, u32)>,
    skip: Option<&'static str>,
}

fn bad() -> Error {
    Error::new("error.hilite.request")
}

/// A list of atom numbers below `n`.
fn atoms_of(v: Option<&Value>, n: usize) -> Result<Vec<u32>> {
    let mut out = Vec::new();
    for x in v.map_or(&[][..], Value::arr) {
        let f = x.f64().ok_or_else(bad)?;
        if f < 0.0 || f.fract() != 0.0 || f as usize >= n {
            return Err(bad());
        }
        out.push(f as u32);
    }
    Ok(out)
}

/// A list of bonds `[a, b]` with atom numbers below `n`.
fn pairs_of(v: Option<&Value>, n: usize) -> Result<Vec<(u32, u32)>> {
    let mut out = Vec::new();
    for p in v.map_or(&[][..], Value::arr) {
        let ab = atoms_of(Some(p), n)?;
        if ab.len() != 2 || ab[0] == ab[1] {
            return Err(bad());
        }
        out.push((ab[0], ab[1]));
    }
    Ok(out)
}

fn query_err(e: impl std::fmt::Display) -> Error {
    Error::new("error.hilite.query").with("detail", e.to_string())
}

/// The atom of the canvas of every atom of `smiles` (the first match of the whole SMILES in the drawing), or None when it does not occur.
fn carry(smiles: &str, canvas: &Mol) -> Result<Option<(Vec<Option<u32>>, usize)>> {
    let mol = Mol::from_smiles(smiles.trim()).map_err(query_err)?;
    let n = mol.atoms.len();
    // The columns of a match are the kept atoms of the query: every atom, or the heavy ones when the hydrogens written in the SMILES are merged.
    let kept: Vec<usize> = (0..n).collect();
    let heavy: Vec<usize> = (0..n).filter(|&i| mol.atoms[i].el != "H").collect();
    let q = SubQuery::exact(mol);
    let cols = if q.mapped_atoms() == n { kept } else { heavy };
    if cols.len() != q.mapped_atoms() {
        return Err(query_err("hydrogens"));
    }
    let opts = SubOptions {
        max_matches: 1,
        ..SubOptions::default()
    };
    let rows = q.matches(canvas, &opts).map_err(query_err)?;
    let Some(row) = rows.first() else {
        return Ok(None);
    };
    let mut to = vec![None; n];
    for (c, &i) in cols.iter().enumerate() {
        to[i] = row.get(c).copied();
    }
    Ok(Some((to, n)))
}

/// The bonds of the canvas between two atoms of `set`.
fn bonds_inside(canvas: &Mol, set: &[u32]) -> Vec<(u32, u32)> {
    canvas
        .bonds
        .iter()
        .filter(|b| set.contains(&b.a) && set.contains(&b.b))
        .map(|b| (b.a.min(b.b), b.a.max(b.b)))
        .collect()
}

fn layer<'a>(v: &'a Value, canvas: &Mol) -> Result<Layer<'a>> {
    let kind = v.get("kind").and_then(Value::str).ok_or_else(bad)?;
    let label = v.get("label").and_then(Value::str).unwrap_or("");
    let mut l = Layer {
        kind,
        label,
        atoms: Vec::new(),
        bonds: Vec::new(),
        skip: None,
    };
    match kind {
        "moiety" => {
            // Not trimmed: a molfile starts with a header line that may be blank.
            let text = v.get("query").and_then(Value::str).unwrap_or("");
            if text.trim().is_empty() {
                return Err(query_err("empty"));
            }
            let q = SubQuery::from_text(text).map_err(query_err)?;
            let opts = SubOptions {
                max_matches: 1000,
                ..SubOptions::default()
            };
            for m in q.matches(canvas, &opts).map_err(query_err)? {
                l.atoms.extend(&m);
                l.bonds.extend(bonds_inside(canvas, &m));
            }
        }
        "sites" | "cuts" => {
            let smiles = v.get("smiles").and_then(Value::str).ok_or_else(bad)?;
            let Some((to, n)) = carry(smiles, canvas)? else {
                l.skip = Some("noMatch");
                return Ok(l);
            };
            if kind == "sites" {
                l.atoms = atoms_of(v.get("atoms"), n)?
                    .into_iter()
                    .filter_map(|a| to[a as usize])
                    .collect();
            } else {
                for (a, b) in pairs_of(v.get("bonds"), n)? {
                    if let (Some(x), Some(y)) = (to[a as usize], to[b as usize]) {
                        let (x, y) = (x.min(y), x.max(y));
                        if canvas
                            .bonds
                            .iter()
                            .any(|k| (k.a.min(k.b), k.a.max(k.b)) == (x, y))
                        {
                            l.bonds.push((x, y));
                        }
                    }
                }
            }
        }
        _ => return Err(bad()),
    }
    if l.atoms.is_empty() && l.bonds.is_empty() {
        l.skip = Some("noMatch");
    }
    Ok(l)
}

/// The `hilite/1` of a request `{"molfile", "world": "hr"|"lr", "selected": [atom], "layers": [{"kind", "label", ...}]}`: `moiety` takes `query`
/// (SMILES or molfile), `sites` takes `smiles` and `atoms`, `cuts` takes `smiles` and `bonds` (`[a, b]`), numbered from 0 on that SMILES. Atoms of the
/// answer are numbered from 0 as in the molfile; a layer with nothing on the canvas says why in `skip` (`noMatch`, `lr`, `tooBig`).
pub fn hilite_json(request: &str) -> Result<String> {
    let v = json::parse(request.as_bytes()).map_err(|_| bad())?;
    let mf = v.get("molfile").and_then(Value::str).ok_or_else(bad)?;
    let lr = match v.get("world").and_then(Value::str).unwrap_or("hr") {
        "hr" => false,
        "lr" => true,
        _ => return Err(bad()),
    };
    let d = Mol::drawing_from_molfile(mf).map_err(|_| Error::new("error.hilite.molfile"))?;
    let canvas = d.mol;
    let n = canvas.atoms.len();
    let selected = atoms_of(v.get("selected"), n)?;
    let wanted = v.get("layers").map_or(&[][..], Value::arr);
    if wanted.len() > MAX_LAYERS {
        return Err(bad());
    }
    let mut layers = Vec::with_capacity(wanted.len());
    for w in wanted {
        let mut l = if n > MAX_ATOMS {
            Layer {
                kind: w.get("kind").and_then(Value::str).ok_or_else(bad)?,
                label: w.get("label").and_then(Value::str).unwrap_or(""),
                atoms: Vec::new(),
                bonds: Vec::new(),
                skip: Some("tooBig"),
            }
        } else {
            layer(w, &canvas)?
        };
        if lr && l.skip.is_none() {
            l.atoms.retain(|a| selected.contains(a));
            l.bonds
                .retain(|(a, b)| selected.contains(a) && selected.contains(b));
            if l.atoms.is_empty() && l.bonds.is_empty() {
                l.skip = Some("lr");
            }
        }
        l.atoms.sortc();
        l.atoms.dedup();
        l.bonds.sortc();
        l.bonds.dedup();
        layers.push(l);
    }
    let mut o = Out::default();
    o.raw("{\"format\":\"hilite/1\",\"layers\":[");
    for (i, l) in layers.iter().enumerate() {
        o.raw(if i > 0 { ",{\"kind\":" } else { "{\"kind\":" });
        o.str(l.kind)
            .raw(",\"label\":")
            .str(l.label)
            .raw(",\"atoms\":[");
        for (k, a) in l.atoms.iter().enumerate() {
            if k > 0 {
                o.raw(",");
            }
            o.int(i64::from(*a));
        }
        o.raw("],\"bonds\":[");
        for (k, (a, b)) in l.bonds.iter().enumerate() {
            o.raw(if k > 0 { ",[" } else { "[" });
            o.int(i64::from(*a)).raw(",").int(i64::from(*b)).raw("]");
        }
        o.raw("]");
        if let Some(s) = l.skip {
            o.raw(",\"skip\":").str(s);
        }
        o.raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}
