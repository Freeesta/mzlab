//! `sel/1`: what the right-click «Info» of the drawing page shows for the selection or the whole drawing. The page sends the V2000 molfile of the
//! piece as drawn, read by the molfile reader of the cuts (`graph::Mol::from_molfile`: open valences filled with H); formula, charge, monoisotopic
//! mass and m/z are the ones of `etich/1` (same code), the average weight is `mzlab_mol::Mol::molecular_weight`, rings are the cycle count of the
//! graph, aromatic bonds are the ones written aromatic (type 4) plus the Hückel perception of `mzlab-mol` on the Kekulé ones (our rule). Only the
//! graph goes to `mzlab-mol`, not its molfile reader, so the `.wasm` `grafo` stays small. Values come from the structure only: nothing is fitted to data.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out, Value};
use mzlab_mol::{Atom, Bond, Mol};

use super::graph;
use super::labels::{shape, shape_json, AtomIn};

/// The `M  RAD` code of every atom (0 = none), as `etich/1` reads it.
fn rad_codes(mf: &str, n: usize) -> Vec<i64> {
    let mut r = vec![0; n];
    for l in mf.lines().filter(|l| l.starts_with("M  RAD")) {
        let v: Vec<i64> = l[6..]
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        for p in v.get(1..).unwrap_or(&[]).chunks(2) {
            if let [a, c] = p {
                if (1..=n as i64).contains(a) {
                    r[*a as usize - 1] = *c;
                }
            }
        }
    }
    r
}

/// Number of connected pieces of `n` atoms joined by `edges` (union-find).
fn pieces(n: usize, edges: impl Iterator<Item = (usize, usize)>) -> usize {
    let mut up: Vec<usize> = (0..n).collect();
    fn root(up: &mut [usize], mut i: usize) -> usize {
        while up[i] != i {
            up[i] = up[up[i]];
            i = up[i];
        }
        i
    }
    let mut k = n;
    for (a, b) in edges {
        let (x, y) = (root(&mut up, a), root(&mut up, b));
        if x != y {
            up[x] = y;
            k -= 1;
        }
    }
    k
}

/// The record `mzlab-mol` perceives: Kekulé orders, a bond written aromatic as single (it is flagged by `aromatic`), unpaired electrons from the codes.
fn record(g: &graph::Mol, rad: &[i64]) -> Mol {
    let unpaired = |c: i64| match c {
        2 => 1,
        3 => 2,
        _ => 0,
    };
    let atoms = g.atoms.iter().zip(rad).map(|(a, r)| Atom {
        el: a.el.clone(),
        iso: a.iso as u16,
        chg: a.charge as i8,
        rad: unpaired(*r),
        h: a.h.min(255) as u8,
    });
    let bonds = g.bonds.iter().map(|b| Bond {
        a: b.a.min(b.b) as u32,
        b: b.a.max(b.b) as u32,
        o: if b.order == 1.5 { 1 } else { b.order as u8 },
    });
    Mol::new(atoms.collect(), bonds.collect(), Vec::new(), Vec::new())
}

/// Aromatic atoms and aromatic rings: the bonds written aromatic plus the Hückel ones; rings = cycles of the aromatic bonds alone
/// (bonds + their pieces - atoms, the other atoms being pieces of one): benzene 1, naphthalene 2.
fn aromatic(g: &graph::Mol, m: &Mol) -> (usize, usize) {
    let n = g.atoms.len();
    let flags: Vec<bool> = m
        .aromatic_bonds()
        .iter()
        .zip(&g.bonds)
        .map(|(p, b)| *p || b.order == 1.5)
        .collect();
    let edges: Vec<(usize, usize)> = g
        .bonds
        .iter()
        .zip(&flags)
        .filter(|(_, f)| **f)
        .map(|(b, _)| (b.a, b.b))
        .collect();
    let mut on = vec![false; n];
    for &(a, b) in &edges {
        on[a] = true;
        on[b] = true;
    }
    let rings = (edges.len() + pieces(n, edges.iter().copied())).saturating_sub(n);
    (on.iter().filter(|x| **x).count(), rings)
}

/// `sel/1` of a request `{"molfile": V2000 text, "part": bool}` (`part` = a selection, echoed so the page can say the numbers are for the piece):
/// `{"format", "part", "label": the etich/1 structure, "average", "heavy", "atoms", "pieces", "rings", "aromaticAtoms", "aromaticRings"}`.
/// `average` is null when an element has no tabulated weight; a molfile the reader refuses keeps its key (`error.grafo.molfile`, `error.grafo.element`, ...).
pub fn sel_json(request: &str) -> Result<String> {
    let v = json::parse(request.as_bytes()).map_err(|_| Error::new("error.sel.request"))?;
    let mf = v
        .get("molfile")
        .and_then(Value::str)
        .ok_or_else(|| Error::new("error.sel.request"))?;
    let part = v.get("part").and_then(Value::bool).unwrap_or(false);
    let g = graph::Mol::from_molfile(mf)?;
    let rad = rad_codes(mf, g.atoms.len());
    let atoms: Vec<AtomIn> = g
        .atoms
        .iter()
        .zip(&rad)
        .map(|(a, r)| AtomIn {
            el: &a.el,
            iso: f64::from(a.iso),
            h: f64::from(a.h),
            q: f64::from(a.charge),
            r: *r as f64,
        })
        .collect();
    let s = shape(&atoms)?;
    let m = record(&g, &rad);
    let n = g.atoms.len();
    let np = pieces(n, g.bonds.iter().map(|b| (b.a, b.b)));
    let (na, ar_rings) = aromatic(&g, &m);
    let mut o = Out::default();
    o.raw("{\"format\":\"sel/1\",\"part\":")
        .raw(if part { "true" } else { "false" })
        .raw(",\"label\":");
    shape_json(&mut o, &s);
    o.raw(",\"average\":").opt(m.molecular_weight());
    o.raw(",\"heavy\":")
        .int(g.atoms.iter().filter(|a| a.el != "H").count() as i64);
    o.raw(",\"atoms\":")
        .int(n as i64)
        .raw(",\"pieces\":")
        .int(np as i64);
    o.raw(",\"rings\":")
        .int((g.bonds.len() + np).saturating_sub(n) as i64);
    o.raw(",\"aromaticAtoms\":")
        .int(na as i64)
        .raw(",\"aromaticRings\":")
        .int(ar_rings as i64)
        .raw("}");
    Ok(o.0)
}

#[cfg(test)]
#[path = "sel_tests.rs"]
mod tests;
