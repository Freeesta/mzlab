//! Molecular descriptors of a record: Wildman-Crippen logP and MR (typing in `crippen.rs`), TPSA, hydrogen-bond donors and acceptors,
//! rotatable bonds and molecular weight. Used by the retention-time prediction of the engines; our own definitions, not another toolkit's code.

use crate::crippen;
use crate::model::Mol;

/// Graph view of a record for the typing rules: heavy neighbours only, hydrogens counted (implicit plus explicit `[H]` atoms).
pub(crate) struct Ctx<'a> {
    pub m: &'a Mol,
    pub nb: Vec<Vec<Nb>>,
    pub hs: Vec<u32>,
    pub ar: Vec<bool>,
}

impl<'a> Ctx<'a> {
    pub fn new(m: &'a Mol) -> Self {
        let abond = m.aromatic_bonds();
        let n = m.atoms.len();
        let mut nb = vec![Vec::new(); n];
        let mut hs: Vec<u32> = m.atoms.iter().map(|a| a.h as u32).collect();
        for (k, b) in m.bonds.iter().enumerate() {
            let (a, c) = (b.a as usize, b.b as usize);
            for (x, y) in [(a, c), (c, a)] {
                if m.atoms[y].el == "H" {
                    hs[x] += 1;
                } else if m.atoms[x].el != "H" {
                    nb[x].push(Nb {
                        j: y,
                        o: b.o,
                        ab: abond[k],
                        k,
                    });
                }
            }
        }
        Ctx {
            m,
            nb,
            hs,
            ar: m.aromatic_atoms(),
        }
    }
    pub fn sym(&self, i: usize) -> &str {
        &self.m.atoms[i].el
    }
    pub fn chg(&self, i: usize) -> i8 {
        self.m.atoms[i].chg
    }
    /// Total connections (heavy neighbours plus hydrogens).
    pub fn x(&self, i: usize) -> usize {
        self.nb[i].len() + self.hs[i] as usize
    }
    /// Aliphatic, not hydrogen.
    pub fn ali(&self, i: usize) -> bool {
        !self.ar[i] && self.sym(i) != "H"
    }
    pub fn is_c(&self, i: usize) -> bool {
        self.ali(i) && self.sym(i) == "C"
    }
    /// Aliphatic N, O, P, S or halogen.
    pub fn hetero(&self, i: usize) -> bool {
        self.ali(i) && matches!(self.sym(i), "N" | "O" | "P" | "S" | "F" | "Cl" | "Br" | "I")
    }
    fn with_order(&self, i: usize, o: u8) -> Vec<usize> {
        self.nb[i]
            .iter()
            .filter(|b| b.o == o && !b.ab)
            .map(|b| b.j)
            .collect()
    }
}

/// One bond seen from an atom: the other end, the Kekulé order and whether the bond is aromatic.
#[derive(Clone, Copy)]
pub(crate) struct Nb {
    pub j: usize,
    pub o: u8,
    pub ab: bool,
    /// Index in `Mol::bonds`.
    pub k: usize,
}

impl Ctx<'_> {
    /// Neighbours through a plain single bond.
    pub fn singles(&self, i: usize) -> Vec<usize> {
        self.with_order(i, 1)
    }
    pub fn doubles(&self, i: usize) -> Vec<usize> {
        self.with_order(i, 2)
    }
    pub fn triples(&self, i: usize) -> Vec<usize> {
        self.with_order(i, 3)
    }
    /// Neighbours through an aromatic bond.
    pub fn arom_nb(&self, i: usize) -> Vec<usize> {
        self.nb[i].iter().filter(|b| b.ab).map(|b| b.j).collect()
    }
}

/// All descriptors of one molecule.
#[derive(Clone, Debug, PartialEq)]
pub struct Descriptors {
    pub logp: f64,
    pub mr: f64,
    pub tpsa: f64,
    pub hbd: u32,
    pub hba: u32,
    pub rotors: u32,
    /// None when an element has no tabulated weight.
    pub mw: Option<f64>,
}

/// Conventional average atomic weights of the elements a mass-spec user meets.
const WEIGHTS: &[(&str, f64)] = &[
    ("H", 1.008),
    ("B", 10.81),
    ("C", 12.011),
    ("N", 14.007),
    ("O", 15.999),
    ("F", 18.998),
    ("Na", 22.990),
    ("Mg", 24.305),
    ("Al", 26.982),
    ("Si", 28.085),
    ("P", 30.974),
    ("S", 32.06),
    ("Cl", 35.45),
    ("K", 39.098),
    ("Ca", 40.078),
    ("Fe", 55.845),
    ("Cu", 63.546),
    ("Zn", 65.38),
    ("As", 74.922),
    ("Se", 78.971),
    ("Br", 79.904),
    ("Sn", 118.71),
    ("I", 126.904),
    ("Pt", 195.08),
    ("Hg", 200.59),
    ("Pb", 207.2),
];

impl Mol {
    /// Wildman-Crippen logP, hydrogens included in the sum.
    pub fn crippen_logp(&self) -> f64 {
        crippen::contributions(&Ctx::new(self)).0
    }

    /// Wildman-Crippen molar refractivity.
    pub fn crippen_mr(&self) -> f64 {
        crippen::contributions(&Ctx::new(self)).1
    }

    /// Topological polar surface area (Ertl 2000) from N and O only, the usual default that leaves out S and P.
    pub fn tpsa(&self) -> f64 {
        let c = Ctx::new(self);
        (0..self.atoms.len()).map(|i| tpsa_atom(&c, i)).sum()
    }
}

fn acceptor(c: &Ctx, i: usize) -> bool {
    let q = c.chg(i);
    match c.sym(i) {
        "O" => !c.ar[i] && q <= 0,
        "N" if q == 0 && !c.ar[i] => {
            !c.triples(i).is_empty()
                || !c.doubles(i).is_empty() && c.hs[i] == 0 && c.nb[i].len() == 2
                || c.doubles(i).is_empty()
                    && c.nb[i].iter().all(|b| {
                        b.o == 1
                            && !b.ab
                            && c.is_c(b.j)
                            && c.doubles(b.j).is_empty()
                            && c.triples(b.j).is_empty()
                    })
        }
        "N" if q == 0 => c.hs[i] == 0 && c.nb[i].len() == 2,
        _ => false,
    }
}

impl Mol {
    /// Hydrogen-bond donors: N and O atoms that carry at least one hydrogen.
    pub fn hbd(&self) -> u32 {
        let c = Ctx::new(self);
        (0..self.atoms.len())
            .filter(|&i| matches!(c.sym(i), "N" | "O") && c.hs[i] > 0)
            .count() as u32
    }

    /// Hydrogen-bond acceptors: neutral-or-anionic O except aromatic o, and N with a free lone pair (pyridine-type n, imine, nitrile and
    /// amine N whose neighbours are all sp3 carbons).
    pub fn hba(&self) -> u32 {
        let c = Ctx::new(self);
        (0..self.atoms.len()).filter(|&i| acceptor(&c, i)).count() as u32
    }

    /// Rotatable bonds: single, acyclic, between two non-terminal atoms, not next to a triple bond, not on an atom of an acyl group (amide,
    /// ester, acid, thio analogues), and not on a CF3 / tert-butyl carbon.
    pub fn rotatable_bonds(&self) -> u32 {
        let c = Ctx::new(self);
        (0..self.bonds.len()).filter(|&k| rotatable(&c, k)).count() as u32
    }

    /// Average molecular weight; isotope-labelled atoms count with their mass number.
    pub fn molecular_weight(&self) -> Option<f64> {
        let mut sum = 0.0;
        for a in &self.atoms {
            let w = if a.iso > 0 {
                a.iso as f64
            } else {
                WEIGHTS.iter().find(|(e, _)| *e == a.el)?.1
            };
            sum += w + a.h as f64 * 1.008;
        }
        Some(sum)
    }

    /// Every descriptor at once.
    pub fn descriptors(&self) -> Descriptors {
        let (logp, mr) = crippen::contributions(&Ctx::new(self));
        Descriptors {
            logp,
            mr,
            tpsa: self.tpsa(),
            hbd: self.hbd(),
            hba: self.hba(),
            rotors: self.rotatable_bonds(),
            mw: self.molecular_weight(),
        }
    }
}

/// True when the bond lies on a ring (the ends stay connected without it).
fn in_ring(c: &Ctx, k: usize) -> bool {
    let b = &c.m.bonds[k];
    let (from, to) = (b.a as usize, b.b as usize);
    let mut seen = vec![false; c.nb.len()];
    let mut stack = vec![from];
    seen[from] = true;
    while let Some(u) = stack.pop() {
        for n in &c.nb[u] {
            if (u == from && n.j == to) || seen[n.j] {
                continue;
            }
            if n.j == to {
                return true;
            }
            seen[n.j] = true;
            stack.push(n.j);
        }
    }
    false
}

fn rotatable(c: &Ctx, k: usize) -> bool {
    let b = &c.m.bonds[k];
    let (a, z) = (b.a as usize, b.b as usize);
    b.o == 1
        && !in_ring(c, k)
        && ((!acyl(c, a) && basic(c, a) && basic(c, z))
            || (!acyl(c, z) && basic(c, z) && basic(c, a)))
}

/// An atom that may end a rotatable bond: not terminal, not on a triple bond, not a CF3 / CCl3 / CBr3 / tert-butyl carbon.
fn basic(c: &Ctx, i: usize) -> bool {
    let nbs = &c.nb[i];
    if nbs.len() < 2 || c.sym(i) == "H" || nbs.iter().any(|b| b.o == 3) {
        return false;
    }
    let terminal = |sym: &str| {
        nbs.iter()
            .filter(|b| c.nb[b.j].len() == 1 && c.sym(b.j) == sym)
            .count()
            >= 3
    };
    !(c.sym(i) == "C" && ["C", "F", "Cl", "Br"].iter().any(|s| terminal(s)))
}

fn polar(c: &Ctx, j: usize) -> bool {
    matches!(c.sym(j), "N" | "O" | "S")
}

/// A carbonyl-type carbon (three neighbours, double bond to N, O or S).
fn carbonyl(c: &Ctx, j: usize) -> bool {
    c.sym(j) == "C"
        && c.nb[j].len() == 3
        && c.nb[j].iter().any(|b| b.o == 2 && !b.ab && polar(c, b.j))
}

/// The first atom of a bond may not be part of an acyl group: the carbonyl carbon single-bonded to a non-terminal N, O or S outside a
/// ring, or that N, O or S. (The second atom of the bond is exempt, so the N-aryl bond of an anilide still counts.)
fn acyl(c: &Ctx, i: usize) -> bool {
    let nbs = &c.nb[i];
    if carbonyl(c, i) {
        return nbs.iter().any(|b| {
            b.o == 1 && !b.ab && polar(c, b.j) && c.nb[b.j].len() > 1 && !in_ring(c, b.k)
        });
    }
    polar(c, i)
        && c.nb[i].len() > 1
        && nbs
            .iter()
            .any(|b| b.o == 1 && !b.ab && carbonyl(c, b.j) && !in_ring(c, b.k))
}

/// Ertl contribution of one N or O atom (surface in square angstroms).
fn tpsa_atom(c: &Ctx, i: usize) -> f64 {
    let (h, n, q) = (c.hs[i], c.nb[i].len(), c.chg(i));
    let dbl = !c.doubles(i).is_empty();
    match (c.sym(i), c.ar[i]) {
        ("O", true) => 13.14,
        ("O", false) => match (q, h, n) {
            (0, 1, 1) => 20.23,
            (0, 2, 0) => 31.5,
            (0, 0, 2) => {
                let nbs: Vec<usize> = c.nb[i].iter().map(|b| b.j).collect();
                if c.nb[nbs[0]].iter().any(|b| b.j == nbs[1]) {
                    12.53
                } else {
                    9.23
                }
            }
            (0, 0, 1) => 17.07,
            (-1, 0, 1) if c.nb[i][0].o == 1 && c.sym(c.nb[i][0].j) == "N" => 14.39,
            (-1, 0, 1) => 23.06,
            _ => 0.0,
        },
        ("N", true) => match (q, h) {
            (0, 0) if n == 2 => 12.89,
            (0, 0) if c.arom_nb(i).len() == 3 => 4.41,
            (0, 0) => 4.93,
            (0, _) => 15.79,
            (_, 1) => 14.14,
            _ => 3.88,
        },
        ("N", false) if q == 0 => {
            if !c.triples(i).is_empty() {
                23.79
            } else if dbl {
                if h == 1 {
                    23.85
                } else {
                    12.36
                }
            } else {
                match (h, n) {
                    (0, _) => 3.24,
                    (1, _) => 12.03,
                    _ => 26.02,
                }
            }
        }
        ("N", false) if q < 0 && n == 1 && dbl => 31.52,
        ("N", false) if q > 0 => {
            if c.nb[i].iter().any(|b| c.sym(b.j) == "O") {
                11.68
            } else if n == 2 && c.doubles(i).len() == 2 {
                4.88
            } else if dbl && h == 2 {
                25.59
            } else if c.x(i) == 4 {
                [0.0, 4.44, 16.61, 27.64][(h as usize).min(3)]
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}
