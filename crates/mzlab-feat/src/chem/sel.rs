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
    // the record `mzlab-mol` perceives: Kekulé orders, a bond written aromatic as single (it is flagged below), unpaired electrons from the codes
    let unpaired = |c: i64| match c {
        2 => 1,
        3 => 2,
        _ => 0,
    };
    let m = Mol::new(
        g.atoms
            .iter()
            .zip(&rad)
            .map(|(a, r)| Atom {
                el: a.el.clone(),
                iso: a.iso as u16,
                chg: a.charge as i8,
                rad: unpaired(*r),
                h: a.h.min(255) as u8,
            })
            .collect(),
        g.bonds
            .iter()
            .map(|b| Bond {
                a: b.a.min(b.b) as u32,
                b: b.a.max(b.b) as u32,
                o: if b.order == 1.5 { 1 } else { b.order as u8 },
            })
            .collect(),
        Vec::new(),
        Vec::new(),
    );
    let n = g.atoms.len();
    let ends = || g.bonds.iter().map(|b| (b.a, b.b));
    let np = pieces(n, ends());
    let rings = (g.bonds.len() + np).saturating_sub(n);
    let ar_bond: Vec<bool> = m
        .aromatic_bonds()
        .iter()
        .zip(&g.bonds)
        .map(|(p, b)| *p || b.order == 1.5)
        .collect();
    let mut ar_atom = vec![false; n];
    ends()
        .zip(&ar_bond)
        .filter(|(_, f)| **f)
        .for_each(|((a, b), _)| {
            ar_atom[a] = true;
            ar_atom[b] = true;
        });
    let na = ar_atom.iter().filter(|x| **x).count();
    let nb = ar_bond.iter().filter(|x| **x).count();
    // cycles of the aromatic bonds alone (bonds + their pieces - atoms, the other atoms being pieces of one): benzene 1, naphthalene 2
    let arom_edges = ends().zip(&ar_bond).filter(|(_, f)| **f).map(|(e, _)| e);
    let ar_rings = (nb + pieces(n, arom_edges)).saturating_sub(n);
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
        .int(np as i64)
        .raw(",\"rings\":")
        .int(rings as i64);
    o.raw(",\"aromaticAtoms\":")
        .int(na as i64)
        .raw(",\"aromaticRings\":")
        .int(ar_rings as i64)
        .raw("}");
    Ok(o.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// V2000 of atoms `els` with bonds `(a, b, type)` (1-based) and property lines.
    fn mf(els: &[&str], bonds: &[(usize, usize, u8)], props: &[&str]) -> String {
        let mut s = format!(
            "\n  t\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
            els.len(),
            bonds.len()
        );
        for (i, e) in els.iter().enumerate() {
            s += &format!(
                "{:>10.4}{:>10.4}{:>10.4} {e:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n",
                i as f64, 0.0, 0.0
            );
        }
        for (a, b, o) in bonds {
            s += &format!("{a:>3}{b:>3}{o:>3}  0  0  0  0\n");
        }
        props.iter().for_each(|p| s += &format!("{p}\n"));
        s + "M  END"
    }
    fn sel(mf: &str, part: bool) -> serde_json::Value {
        let req = serde_json::json!({ "molfile": mf, "part": part }).to_string();
        serde_json::from_str(&sel_json(&req).unwrap()).unwrap()
    }
    const BENZENE: [(usize, usize, u8); 6] = [
        (1, 2, 2),
        (2, 3, 1),
        (3, 4, 2),
        (4, 5, 1),
        (5, 6, 2),
        (6, 1, 1),
    ];

    #[test]
    fn kekule_benzene_is_one_aromatic_ring() {
        let v = sel(&mf(&["C"; 6], &BENZENE, &[]), false);
        assert_eq!(v["format"], "sel/1");
        assert_eq!(v["label"]["formula"], "C6H6");
        assert!((v["label"]["mass"].as_f64().unwrap() - 78.04695).abs() < 1e-5);
        assert!((v["average"].as_f64().unwrap() - 78.114).abs() < 1e-3);
        assert_eq!(
            (
                v["heavy"].as_i64(),
                v["rings"].as_i64(),
                v["aromaticRings"].as_i64(),
                v["aromaticAtoms"].as_i64()
            ),
            (Some(6), Some(1), Some(1), Some(6))
        );
        assert!(v["label"]["mz"].is_null() && v["part"] == false);
    }

    #[test]
    fn benzene_written_aromatic_counts_the_same() {
        let b: Vec<_> = BENZENE.iter().map(|&(a, c, _)| (a, c, 4)).collect();
        let v = sel(&mf(&["C"; 6], &b, &[]), false);
        assert_eq!(
            (
                v["label"]["formula"].as_str(),
                v["rings"].as_i64(),
                v["aromaticRings"].as_i64(),
                v["aromaticAtoms"].as_i64()
            ),
            (Some("C6H6"), Some(1), Some(1), Some(6)),
            "{v}"
        );
    }

    #[test]
    fn naphthalene_two_rings_cyclohexane_none_aromatic() {
        let mut b = BENZENE.to_vec();
        b[5] = (6, 1, 1);
        b.extend([(1, 7, 1), (7, 8, 2), (8, 9, 1), (9, 10, 2), (10, 6, 1)]);
        b[0] = (1, 2, 2);
        let v = sel(&mf(&["C"; 10], &b, &[]), false);
        assert_eq!(
            (
                v["label"]["formula"].as_str(),
                v["rings"].as_i64(),
                v["aromaticRings"].as_i64()
            ),
            (Some("C10H8"), Some(2), Some(2)),
            "{v}"
        );
        let hex: Vec<_> = (1..=6).map(|i| (i, i % 6 + 1, 1)).collect();
        let v = sel(&mf(&["C"; 6], &hex, &[]), true);
        assert_eq!(
            (
                v["label"]["formula"].as_str(),
                v["rings"].as_i64(),
                v["aromaticRings"].as_i64(),
                v["part"].as_bool()
            ),
            (Some("C6H12"), Some(1), Some(0), Some(true))
        );
    }

    #[test]
    fn charge_radical_and_pieces() {
        // acetate: CC(=O)[O-]
        let v = sel(
            &mf(
                &["C", "C", "O", "O"],
                &[(1, 2, 1), (2, 3, 2), (2, 4, 1)],
                &["M  CHG  1   4  -1"],
            ),
            false,
        );
        assert_eq!(
            (v["label"]["text"].as_str(), v["label"]["charge"].as_i64()),
            (Some("C2H3O2-"), Some(-1))
        );
        assert!(
            (v["label"]["mz"].as_f64().unwrap() - 59.01385).abs() < 1e-4,
            "{v}"
        );
        // methyl radical and a separate methane: two pieces, one unpaired electron
        let v = sel(&mf(&["C", "C"], &[], &["M  RAD  1   1   2"]), true);
        assert_eq!(
            (
                v["label"]["formula"].as_str(),
                v["pieces"].as_i64(),
                v["label"]["unpaired"].as_i64()
            ),
            (Some("C2H7"), Some(2), Some(1)),
            "{v}"
        );
    }

    #[test]
    fn errors_keep_their_key() {
        assert_eq!(sel_json("nope").unwrap_err().key, "error.sel.request");
        assert_eq!(
            sel_json("{\"molfile\":\"x\"}").unwrap_err().key,
            "error.grafo.molfile"
        );
        assert_eq!(sel_json("{}").unwrap_err().key, "error.sel.request");
    }
}
