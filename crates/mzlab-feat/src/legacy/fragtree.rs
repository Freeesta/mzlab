//! The fragmentation tree of the parent as mzFinder draws it (`_two_level`, with `FormulaSpace`
//! and `substructures` + `mask_smiles`): the parent ion and its strongest MS2 fragments, each with the formulas and neutral
//! losses that explain it and the substructures of the SMILES that can be it. The JSON is the one of `api.frag_tree`.

use crate::sortc::SortC;
use std::collections::{HashMap, HashSet};

use mzlab_core::elements::{mass, mass_of, parse_formula, ELECTRON};
use mzlab_core::json::{round, Out};

use crate::chem::grafo;

mod mol;
pub use mol::*;

pub(crate) const MIN_REL: f64 = 5.0;
const MAX_ALT: usize = 6;
const MAX_SMILES: usize = 3;
const LR_NOTE: &str = "supposed: unit resolution";
/// Elements of `formula.VALENCE`, in the order of `element_order`, with their valence.
const ELEMENTS: [(&str, i64); 10] = [
    ("C", 4),
    ("H", 1),
    ("N", 3),
    ("O", 2),
    ("S", 2),
    ("P", 3),
    ("F", 1),
    ("Cl", 1),
    ("Br", 1),
    ("I", 1),
];

/// `formula.element_order`: C, H, N, O, S always, then the other elements present; None for an element the high-resolution mode does not know.
pub fn element_order<'a>(formulas: impl IntoIterator<Item = &'a str>) -> Option<Vec<&'static str>> {
    let mut present: HashSet<&str> = ["C", "H", "N", "O", "S"].into_iter().collect();
    for e in formulas {
        if !ELEMENTS.iter().any(|x| x.0 == e) || mass_of(e).is_none() {
            return None;
        }
        present.insert(ELEMENTS.iter().find(|x| x.0 == e).unwrap().0);
    }
    Some(
        ELEMENTS
            .iter()
            .map(|x| x.0)
            .filter(|e| present.contains(e))
            .collect(),
    )
}

/// `element_order` of one formula (None when it cannot be read or has an unknown element).
pub fn elements_of(formula: &str) -> Option<Vec<&'static str>> {
    let p = parse_formula(formula).ok()?;
    element_order(p.iter().map(|x| x.0.as_str()))
}

/// `formula.vec`: the counts of `f` in the order `els` (None when the formula cannot be read).
pub fn vec_of(f: &str, els: &[&str]) -> Option<Vec<i64>> {
    let p = parse_formula(f).ok()?;
    Some(
        els.iter()
            .map(|e| p.iter().find(|x| x.0 == *e).map_or(0, |x| x.1 as i64))
            .collect(),
    )
}

/// `formula.fmt`.
pub fn fmt(v: &[i64], els: &[&str]) -> String {
    let mut s = String::new();
    for (e, &n) in els.iter().zip(v) {
        if n != 0 {
            s += e;
            if n != 1 {
                s += &n.to_string();
            }
        }
    }
    s
}

/// `FormulaSpace(upper, els)` with its defaults: every formula below `upper` with carbon and RDBE >= -0.5, sorted by the m/z of the cation.
pub struct Space {
    pub(crate) g: Vec<Vec<i64>>,
    pub(crate) mass: Vec<f64>,
}

impl Space {
    pub(crate) fn new(upper: &[i64], els: &[&str]) -> Space {
        let m: Vec<f64> = els.iter().map(|e| mass_of(e).unwrap_or(f64::NAN)).collect();
        let val: Vec<i64> = els
            .iter()
            .map(|e| ELEMENTS.iter().find(|x| x.0 == *e).map_or(0, |x| x.1))
            .collect();
        let ic = els.iter().position(|e| *e == "C").unwrap_or(0);
        let mut rows: Vec<(f64, Vec<i64>)> = Vec::new();
        let mut cur = vec![0i64; upper.len()];
        loop {
            let rdbe = 1.0
                + 0.5
                    * cur
                        .iter()
                        .zip(&val)
                        .map(|(n, v)| (n * (v - 2)) as f64)
                        .fold(0.0, |a, b| a + b);
            if cur[ic] > 0 && rdbe >= -0.5 {
                let ms = cur
                    .iter()
                    .zip(&m)
                    .map(|(n, x)| *n as f64 * x)
                    .fold(0.0, |a, b| a + b)
                    - ELECTRON;
                rows.push((ms, cur.clone()));
            }
            // the next vector in row-major order (the last element fastest), as `np.meshgrid(..., indexing="ij")`
            let mut k = upper.len();
            loop {
                if k == 0 {
                    rows.sortc_by(|a, b| a.0.total_cmp(&b.0));
                    let (mass, g) = rows.into_iter().unzip();
                    return Space { g, mass };
                }
                k -= 1;
                if cur[k] < upper[k] {
                    cur[k] += 1;
                    break;
                }
                cur[k] = 0;
            }
        }
    }

    /// `candidates(mz, ppm, abs_da)`: indices within tolerance, smallest |error| first.
    fn candidates(&self, mz: f64, ppm: f64, abs_da: f64) -> Vec<usize> {
        let tol = (mz * ppm * 1e-6).max(abs_da);
        let a = self.mass.partition_point(|&x| x < mz - tol);
        let b = self.mass.partition_point(|&x| x <= mz + tol);
        let mut idx: Vec<usize> = (a..b.max(a)).collect();
        idx.sortc_by(|&x, &y| {
            (self.mass[x] - mz)
                .abs()
                .total_cmp(&(self.mass[y] - mz).abs())
        });
        idx
    }
}

/// `_loss_str`: the formula of parent minus fragment, None when it is not a sub-formula or is empty.
pub(crate) fn loss_str(par: &[i64], v: &[i64], els: &[&str]) -> Option<String> {
    let d: Vec<i64> = par.iter().zip(v).map(|(a, b)| a - b).collect();
    (!d.iter().any(|&x| x < 0) && d.iter().any(|&x| x != 0)).then(|| fmt(&d, els))
}

pub(crate) fn loss_mass(s: &str) -> Option<f64> {
    parse_formula(s).ok().map(|f| round(mass(&f), 4))
}

/// The inputs of `_two_level`.
pub struct TwoLevel<'a> {
    /// The ion formula (counts in the order `els`), None when unknown.
    pub ion: Option<&'a [i64]>,
    pub els: &'a [&'a str],
    pub mz0: f64,
    pub mzs: &'a [f64],
    pub rel: &'a [f64],
    pub smiles: Option<&'a str>,
    pub ppm: f64,
    pub abs_da: f64,
    pub supposed: bool,
    pub mode: &'a str,
    pub n_scans: Option<i64>,
}

/// One explanation of a fragment.
struct Alt {
    formula: Option<String>,
    loss: String,
    err: f64,
    loss_source: bool,
}

fn alternatives(t: &TwoLevel, space: Option<&Space>, losses: &[(&str, f64)], m: f64) -> Vec<Alt> {
    let mut alts = Vec::new();
    match (space, t.ion) {
        (Some(sp), Some(ion)) => {
            for c in sp.candidates(m, t.ppm, t.abs_da).into_iter().take(80) {
                let v = &sp.g[c];
                let Some(ls) = loss_str(ion, v, t.els) else {
                    continue;
                };
                alts.push(Alt {
                    formula: Some(fmt(v, t.els)),
                    loss_source: losses.iter().any(|x| x.0 == ls),
                    err: round(sp.mass[c] - m, 4),
                    loss: ls,
                });
            }
            alts.sortc_by(|a, b| {
                (!a.loss_source)
                    .cmp(&!b.loss_source)
                    .then(a.err.abs().total_cmp(&b.err.abs()))
                    .then(a.loss.chars().count().cmp(&b.loss.chars().count()))
            });
        }
        _ => {
            for &(name, lm) in losses {
                if ((t.mz0 - m) - lm).abs() <= t.abs_da {
                    alts.push(Alt {
                        formula: None,
                        loss: name.into(),
                        err: round(t.mz0 - lm - m, 4),
                        loss_source: true,
                    });
                }
            }
            alts.sortc_by(|a, b| a.err.abs().total_cmp(&b.err.abs()));
        }
    }
    alts.truncate(MAX_ALT);
    alts
}

/// The first keys of a node: the root (`P`, the ion) or a fragment of it.
fn node_head(o: &mut Out, id: &str, mz: f64, formula: Option<&str>, rel: f64) {
    let root = id == "P";
    o.raw("{\"id\":").str(id).raw(",\"parent\":");
    o.raw(if root { "null" } else { "\"P\"" });
    o.raw(",\"level\":")
        .int(if root { 1 } else { 2 })
        .raw(",\"kind\":")
        .str(if root { "ion" } else { "frag" })
        .raw(",\"mz\":")
        .num(round(mz, 4));
    o.raw(",\"formula\":");
    match formula {
        Some(f) => o.str(f),
        None => o.raw("null"),
    };
    o.raw(",\"rel\":").num(rel);
}

fn node_tail(o: &mut Out, n_scans: Option<i64>, smiles: &[(String, f64)]) {
    o.raw(",\"ppm\":null,\"n_scans\":");
    match n_scans {
        Some(n) => o.int(n),
        None => o.raw("null"),
    };
    o.raw(",\"ce\":null,\"ghost\":false,\"smiles\":[");
    for (i, (s, w)) in smiles.iter().enumerate() {
        o.raw(if i > 0 {
            ",{\"smiles\":"
        } else {
            "{\"smiles\":"
        })
        .str(s)
        .raw(",\"score\":")
        .num(*w)
        .raw("}");
    }
    o.raw("]");
}

/// The `alternatives` of a fragment, closing its node.
fn write_alts(o: &mut Out, alts: &[Alt]) {
    o.raw(",\"alternatives\":[");
    for (k, a) in alts.iter().enumerate() {
        o.raw(if k > 0 {
            ",{\"formula\":"
        } else {
            "{\"formula\":"
        });
        match &a.formula {
            Some(f) => o.str(f),
            None => o.raw("null"),
        };
        o.raw(",\"loss\":").str(&a.loss).raw(",\"err\":").num(a.err);
        o.raw(",\"source\":")
            .str(if a.loss_source { "loss" } else { "subformula" })
            .raw("}");
    }
    o.raw("]}");
}

/// `_two_level(...)`: the parent and its strongest fragments (at most 12, >= 5 % of the base peak, below the precursor).
pub fn two_level(t: &TwoLevel, losses: &[(&str, f64)]) -> String {
    let top = if t.rel.is_empty() {
        1.0
    } else {
        t.rel.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    };
    let mut ord: Vec<usize> = (0..t.rel.len()).collect();
    ord.sortc_by(|&a, &b| t.rel[b].total_cmp(&t.rel[a]));
    let keep: Vec<usize> = ord
        .into_iter()
        .filter(|&j| t.rel[j] * 100.0 / top >= MIN_REL && t.mzs[j] < t.mz0 - 1.0)
        .take(12)
        .collect();
    let space = t.ion.map(|ion| Space::new(ion, t.els));
    let subs = t.smiles.and_then(Graph::parse).and_then(substructures);
    let mut o = Out::default();
    o.raw("{\"ok\":true,\"mode\":")
        .str(t.mode)
        .raw(",\"supposed\":")
        .raw(if t.supposed { "true" } else { "false" });
    o.raw(",\"note\":")
        .str(if t.supposed { LR_NOTE } else { "" })
        .raw(",\"nodes\":[");
    let root_f = t.ion.map(|v| fmt(v, t.els));
    node_head(&mut o, "P", t.mz0, root_f.as_deref(), 100.0);
    o.raw(",\"loss\":null,\"loss_mass\":null");
    let root_sm: Vec<(String, f64)> = t
        .smiles
        .map(|s| vec![(s.to_string(), 1.0)])
        .unwrap_or_default();
    node_tail(&mut o, t.n_scans, &root_sm);
    o.raw(",\"alternatives\":[]}");
    for (q, &j) in keep.iter().enumerate() {
        let m = t.mzs[j];
        let alts = alternatives(t, space.as_ref(), losses, m);
        let best = alts.first();
        let mut sm = Vec::new();
        if let (Some(s), Some(Some(f))) = (subs.as_ref(), best.map(|b| b.formula.as_deref())) {
            if let Some(v) = vec_of(f, t.els) {
                let (idx, w) = s.candidates(&v);
                if !idx.is_empty() {
                    sm = s.top_smiles(&idx, &w);
                }
            }
        }
        o.raw(",");
        let rel = round(t.rel[j] * 100.0 / top, 1);
        node_head(
            &mut o,
            &format!("F{}", q + 1),
            m,
            best.and_then(|b| b.formula.as_deref()),
            rel,
        );
        o.raw(",\"loss\":");
        match best {
            Some(b) => o
                .str(&b.loss)
                .raw(",\"loss_mass\":")
                .opt(loss_mass(&b.loss)),
            None => o.raw("null,\"loss_mass\":").num(round(t.mz0 - m, 2)),
        };
        node_tail(&mut o, t.n_scans, &sm);
        write_alts(&mut o, &alts);
    }
    o.raw("]}");
    o.0
}

/// `fragtree.hr` of a Dig (no MSn file): the tree of the DDA MS2 of the parent, or the note when there is none.
pub fn of_dig(d: &super::dig::Dig) -> String {
    let Some(p2) = d.parent_ms2.as_ref().filter(|c| !c.mz.is_empty()) else {
        return r#"{"ok":false,"mode":"hr","note":"no MSn file and no MS2 of the parent: no fragmentation tree","nodes":[]}"#.into();
    };
    let ion = d.parent.ion_formula();
    let els = elements_of(&ion).unwrap_or_default();
    let vec = vec_of(&ion, &els).filter(|_| !els.is_empty());
    let t = TwoLevel {
        ion: vec.as_deref(),
        els: &els,
        mz0: d.parent.mz,
        mzs: &p2.mz,
        rel: &p2.rel,
        smiles: d.parent.smiles.as_deref(),
        ppm: 5.0,
        abs_da: 0.002,
        supposed: false,
        mode: "hr",
        n_scans: Some(p2.n_scans as i64),
    };
    two_level(&t, &super::edge::Iimn::new().losses)
}

/// `fragtree.compare_hr` without the localisation of the product (the Dig in Rust has none): the empty answer.
pub const COMPARE_NONE: &str = r#"{"ok":false,"note":"no fragment of the product with a known relation to the parent","marks":{}}"#;
