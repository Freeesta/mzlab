//! SMILES -> heavy-atom graph (port of the reading part of `TP_Mine/py/tpmine/hr/molgraph.py`): formula, functional groups and the cleavages of the
//! structure. The groups are the preconditions of `regole_tp/2` (`hypo::rules`); the formulas of the two sides of every cleavage go to
//! `hypo::Inputs::fragments`. No SMARTS and no aromaticity perception: a bond is aromatic when both atoms are written aromatic.
//! Hydrogen accounting of a cleavage: each side takes an H (`cut`); at an ether, ester, amide or sulfonamide bond one side takes OH instead and the other H
//! (`hydrolysis`, both orientations), as water would split it.

use std::collections::BTreeSet;

use mzlab_core::elements::{self, Formula};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;

use super::smiles::{self, normal_valence, RawAtom, RawBond};

/// Atoms the graph accepts (the localisation of the Python version stopped at 64).
pub const MAX_ATOMS: usize = 200;

// The impl comes first and the types after it: the size check of `tools/controlla.py` measures a function or struct up to the next one at the same indent.
impl Mol {
    /// One molecule; salts and mixtures (`.`) are refused.
    pub fn parse(smiles: &str) -> Result<Mol> {
        let (raw, written) = smiles::read(smiles)?;
        if raw.len() > MAX_ATOMS {
            return Err(Error::new("error.grafo.tooBig").with("max", MAX_ATOMS));
        }
        for a in &raw {
            if elements::mass_of(&a.el).is_none() || normal_valence(&a.el).is_none() {
                return Err(Error::new("error.grafo.element").with("element", &a.el));
            }
        }
        Ok(Mol::assemble(raw, written))
    }

    /// One molecule from a V2000 molfile written by the drawing page (`pieceMolfile`): heavy atoms in file order (so atom `i` here is atom `i + 1` of
    /// the file), implicit hydrogens by valence, `M  CHG` charges, bond type 4 aromatic. Disconnected pieces are accepted (the page cuts bonds).
    pub fn from_molfile(mf: &str) -> Result<Mol> {
        let bad = || Error::new("error.grafo.molfile");
        let lines: Vec<&str> = mf.lines().collect();
        let counts = lines.get(3).ok_or_else(bad)?;
        let num =
            |s: &str, a: usize, b: usize| s.get(a..b).and_then(|t| t.trim().parse::<i64>().ok());
        let (na, nb) = (
            num(counts, 0, 3).ok_or_else(bad)? as usize,
            num(counts, 3, 6).ok_or_else(bad)? as usize,
        );
        if na > MAX_ATOMS || lines.len() < 4 + na + nb {
            return Err(if na > MAX_ATOMS {
                Error::new("error.grafo.tooBig").with("max", MAX_ATOMS)
            } else {
                bad()
            });
        }
        let mut raw: Vec<RawAtom> = Vec::new();
        for l in &lines[4..4 + na] {
            let el = l.get(31..34).ok_or_else(bad)?.trim().to_string();
            if elements::mass_of(&el).is_none() || normal_valence(&el).is_none() {
                return Err(Error::new("error.grafo.element").with("element", &el));
            }
            raw.push(RawAtom {
                el,
                arom: false,
                h: None,
                charge: 0,
            });
        }
        let mut written: Vec<RawBond> = Vec::new();
        for l in &lines[4 + na..4 + na + nb] {
            let (a, b, t) = (num(l, 0, 3), num(l, 3, 6), num(l, 6, 9));
            let (Some(a), Some(b), Some(t)) = (a, b, t) else {
                return Err(bad());
            };
            if a < 1 || b < 1 || a as usize > na || b as usize > na {
                return Err(bad());
            }
            let arom = t == 4;
            if arom {
                raw[a as usize - 1].arom = true;
                raw[b as usize - 1].arom = true;
            }
            written.push((
                a as usize - 1,
                b as usize - 1,
                if arom { 1.5 } else { t.clamp(1, 3) as f32 },
                true,
            ));
        }
        for l in &lines[4 + na + nb..] {
            if let Some(rest) = l.strip_prefix("M  CHG") {
                let v: Vec<i32> = rest
                    .split_whitespace()
                    .filter_map(|t| t.parse().ok())
                    .collect();
                for p in v.get(1..).unwrap_or(&[]).chunks(2) {
                    if let [i, c] = p {
                        if *i >= 1 && (*i as usize) <= na {
                            raw[*i as usize - 1].charge = *c;
                        }
                    }
                }
            }
        }
        Ok(Mol::assemble(raw, written))
    }

    fn assemble(raw: Vec<RawAtom>, written: Vec<RawBond>) -> Mol {
        let bonds = typed_bonds(&raw, written);
        let mut atoms = smiles::fill(&raw, &bonds);
        for a in atoms.iter_mut().filter(|a| a.charge != 0) {
            // a charge moves the valence: N+ and O+ take one more, O- one less, C+/C- one less
            let up = matches!(a.el.as_str(), "N" | "P" | "O" | "S");
            let d = if up { a.charge } else { -a.charge.abs() };
            a.h = (a.h as i32 + d).max(0) as u32;
        }
        let mut adj = vec![Vec::new(); atoms.len()];
        for (k, b) in bonds.iter().enumerate() {
            adj[b.a].push((b.b, k));
            adj[b.b].push((b.a, k));
        }
        Mol { atoms, bonds, adj }
    }

    pub fn formula(&self) -> Formula {
        self.formula_of(&vec![true; self.atoms.len()])
    }

    /// Neutral formula of the atoms in `keep` (heavy atoms and the hydrogens they carry).
    pub(super) fn formula_of(&self, keep: &[bool]) -> Formula {
        let mut f: Formula = Vec::new();
        let mut add = |el: &str, n: u64| match f.iter_mut().find(|(e, _)| e == el) {
            Some(x) => x.1 += n,
            None => f.push((el.to_string(), n)),
        };
        for (i, a) in self.atoms.iter().enumerate() {
            if keep[i] {
                add(&a.el, 1);
                if a.h > 0 {
                    add("H", a.h as u64);
                }
            }
        }
        f
    }

    pub(super) fn el(&self, i: usize) -> &str {
        &self.atoms[i].el
    }

    pub(super) fn heavy(&self, i: usize) -> usize {
        self.adj[i].len()
    }

    /// Double-bonded oxygens of `i`.
    pub(super) fn dbl_o(&self, i: usize) -> usize {
        self.adj[i]
            .iter()
            .filter(|(j, k)| self.el(*j) == "O" && self.bonds[*k].order == 2.0)
            .count()
    }

    pub(super) fn is_carbonyl(&self, i: usize) -> bool {
        self.el(i) == "C" && self.dbl_o(i) == 1
    }

    pub(super) fn is_sulfonyl(&self, i: usize) -> bool {
        self.el(i) == "S" && self.dbl_o(i) >= 2
    }

    /// Acyl or sulfonyl carbon/sulfur: the amine and alcohol next to it are an amide, ester or sulfonamide.
    pub(super) fn is_acyl(&self, i: usize) -> bool {
        self.is_carbonyl(i) || self.is_sulfonyl(i) || (self.el(i) == "S" && self.dbl_o(i) == 1)
    }

    pub(super) fn sp3_c(&self, i: usize) -> bool {
        self.el(i) == "C"
            && !self.atoms[i].arom
            && self.adj[i].iter().all(|(_, k)| self.bonds[*k].order == 1.0)
    }

    /// Bonds on a ring: cutting one does not split the molecule.
    pub(super) fn ring_bonds(&self) -> Vec<bool> {
        (0..self.bonds.len())
            .map(|k| {
                let (a, b) = (self.bonds[k].a, self.bonds[k].b);
                let mut seen = vec![false; self.atoms.len()];
                seen[a] = true;
                let mut st = vec![a];
                while let Some(u) = st.pop() {
                    for (v, kk) in &self.adj[u] {
                        if *kk != k && !seen[*v] {
                            seen[*v] = true;
                            st.push(*v);
                        }
                    }
                }
                seen[b]
            })
            .collect()
    }

    /// Atoms of the smallest ring through each ring bond (duplicates removed).
    pub(super) fn rings(&self) -> Vec<Vec<usize>> {
        let on_ring = self.ring_bonds();
        let mut found: BTreeSet<Vec<usize>> = BTreeSet::new();
        for (k, bd) in self.bonds.iter().enumerate() {
            if !on_ring[k] {
                continue;
            }
            let mut prev: Vec<Option<usize>> = vec![None; self.atoms.len()];
            let mut seen = vec![false; self.atoms.len()];
            seen[bd.a] = true;
            let mut q = vec![bd.a];
            'bfs: while !q.is_empty() {
                let mut nq = Vec::new();
                for u in q {
                    for (v, kk) in &self.adj[u] {
                        if *kk == k || seen[*v] {
                            continue;
                        }
                        seen[*v] = true;
                        prev[*v] = Some(u);
                        if *v == bd.b {
                            break 'bfs;
                        }
                        nq.push(*v);
                    }
                }
                q = nq;
            }
            if seen[bd.b] {
                let (mut ring, mut u) = (Vec::new(), Some(bd.b));
                while let Some(x) = u {
                    ring.push(x);
                    u = prev[x];
                }
                ring.sort_unstable();
                found.insert(ring);
            }
        }
        found.into_iter().collect()
    }

    /// The atoms on the side of `bond.a` and of `bond.b` when the (non-ring) bond is cut.
    pub(super) fn sides(&self, k: usize) -> (Vec<bool>, Vec<bool>) {
        let mut a = vec![false; self.atoms.len()];
        a[self.bonds[k].a] = true;
        let mut st = vec![self.bonds[k].a];
        while let Some(u) = st.pop() {
            for (v, kk) in &self.adj[u] {
                if *kk != k && !a[*v] {
                    a[*v] = true;
                    st.push(*v);
                }
            }
        }
        let b = a.iter().map(|x| !x).collect();
        (a, b)
    }

    /// Every cleavage of a single bond outside the rings, with the formulas of its products. Rings are not opened here (the group
    /// `ring_hetero_saturated` is the precondition of the ring-opening rules).
    pub fn cleavages(&self) -> Vec<Cleavage> {
        let on_ring = self.ring_bonds();
        let mut out = Vec::new();
        for (k, bd) in self.bonds.iter().enumerate() {
            if on_ring[k] || bd.order != 1.0 {
                continue;
            }
            let (sa, sb) = self.sides(k);
            let (fa, fb) = (self.formula_of(&sa), self.formula_of(&sb));
            let kind = hydrolysable(self, k);
            let mut set: BTreeSet<String> = BTreeSet::new();
            let mut put = |f: Formula| {
                set.insert(elements::fmt(&f));
            };
            put(add_h(fa.clone(), 1, 0));
            put(add_h(fb.clone(), 1, 0));
            if kind != "cut" {
                put(add_h(fa, 1, 1));
                put(add_h(fb, 1, 1));
            }
            out.push(Cleavage {
                a: bd.a,
                b: bd.b,
                kind,
                products: set.into_iter().collect(),
            });
        }
        out
    }

    /// The distinct products of all the cleavages that `from_fragments` accepts for this parent: no element beyond the parent (hydrogens up to +3),
    /// not the parent itself. Sorted.
    pub fn fragment_formulas(&self) -> Vec<String> {
        let parent = elements::fmt(&self.formula());
        let pc: Vec<(String, u64)> = self.formula();
        let have = |e: &str| pc.iter().find(|(x, _)| x == e).map_or(0, |(_, n)| *n);
        let mut set: BTreeSet<String> = BTreeSet::new();
        for c in self.cleavages() {
            for p in c.products {
                let ok = elements::parse_formula(&p).is_ok_and(|f| {
                    f.iter()
                        .all(|(e, n)| *n <= have(e) + if e == "H" { 3 } else { 0 })
                });
                if ok && p != parent {
                    set.insert(p);
                }
            }
        }
        set.into_iter().collect()
    }
}

#[derive(Debug, Clone)]
pub struct Atom {
    pub el: String,
    pub arom: bool,
    pub h: u32,
    pub charge: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Bond {
    pub a: usize,
    pub b: usize,
    /// 1, 2, 3 or 1.5 (aromatic).
    pub order: f32,
}

#[derive(Debug, Clone)]
pub struct Mol {
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    pub(super) adj: Vec<Vec<(usize, usize)>>,
}

/// The bonds with their order; one between two aromatic atoms with no symbol written is aromatic.
fn typed_bonds(raw: &[RawAtom], written: Vec<RawBond>) -> Vec<Bond> {
    written
        .into_iter()
        .map(|(a, b, o, ex)| {
            let arom = o == 1.0 && !ex && raw[a].arom && raw[b].arom;
            Bond {
                a,
                b,
                order: if arom { 1.5 } else { o },
            }
        })
        .collect()
}

/// One cut of a bond outside the rings.
#[derive(Debug, Clone, PartialEq)]
pub struct Cleavage {
    pub a: usize,
    pub b: usize,
    /// `cut`, or `ether`, `ester`, `amide`, `sulfonamide` when the bond is one water can split.
    pub kind: &'static str,
    /// Formulas of the products, Hill order, distinct.
    pub products: Vec<String>,
}

pub(super) fn add_h(mut f: Formula, h: u64, o: u64) -> Formula {
    for (el, n) in [("H", h), ("O", o)] {
        if n == 0 {
            continue;
        }
        match f.iter_mut().find(|(e, _)| e == el) {
            Some(x) => x.1 += n,
            None => f.push((el.to_string(), n)),
        }
    }
    f
}

pub(super) fn hydrolysable(m: &Mol, k: usize) -> &'static str {
    let (a, b) = (m.bonds[k].a, m.bonds[k].b);
    let pair = |x: usize, y: usize| -> &'static str {
        // `x` is the O or N, `y` the other end
        if m.el(x) == "O" && m.atoms[x].h == 0 && m.heavy(x) == 2 {
            if m.is_carbonyl(y) || m.adj[x].iter().any(|(j, _)| *j != y && m.is_carbonyl(*j)) {
                return "ester";
            }
            if m.el(y) == "C" {
                return "ether";
            }
        }
        if m.el(x) == "N" && m.is_carbonyl(y) {
            return "amide";
        }
        if m.el(x) == "N" && m.is_sulfonyl(y) {
            return "sulfonamide";
        }
        ""
    };
    for (x, y) in [(a, b), (b, a)] {
        let r = pair(x, y);
        if !r.is_empty() {
            return r;
        }
    }
    "cut"
}

/// `{"formula", "atoms", "groups":[..], "fragments":[..], "cuts":[{"a","b","kind","products":[..]}]}`: what the page needs from a SMILES.
pub fn analyse_json(smiles: &str) -> Result<String> {
    let m = Mol::parse(smiles)?;
    let mut o = Out::default();
    let strs = |o: &mut Out, v: &[String]| {
        o.raw("[");
        for (i, s) in v.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(s);
        }
        o.raw("]");
    };
    o.raw("{\"format\":\"grafo/1\",\"formula\":")
        .str(&elements::fmt(&m.formula()));
    o.raw(",\"atoms\":").int(m.atoms.len() as i64);
    let groups: Vec<String> = m.groups().iter().map(|g| g.to_string()).collect();
    o.raw(",\"groups\":");
    strs(&mut o, &groups);
    o.raw(",\"fragments\":");
    strs(&mut o, &m.fragment_formulas());
    o.raw(",\"cuts\":[");
    for (i, c) in m.cleavages().iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"a\":")
            .int(c.a as i64)
            .raw(",\"b\":")
            .int(c.b as i64)
            .raw(",\"kind\":")
            .str(c.kind)
            .raw(",\"products\":");
        strs(&mut o, &c.products);
        o.raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}
