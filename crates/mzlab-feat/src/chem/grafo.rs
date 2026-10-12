//! SMILES -> heavy-atom graph (port of the reading part of `TP_Mine/py/tpmine/hr/molgraph.py`): formula, functional groups and the cleavages of the
//! structure. The groups are the preconditions of `regole_tp/2` (`hypo::rules`); the formulas of the two sides of every cleavage go to
//! `hypo::Inputs::fragments`. No SMARTS and no aromaticity perception: a bond is aromatic when both atoms are written aromatic.
//! Hydrogen accounting of a cleavage: each side takes an H (`cut`); at an ether, ester, amide or sulfonamide bond one side takes OH instead and the other H
//! (`hydrolysis`, both orientations), as water would split it.

use std::collections::BTreeSet;

use mzlab_core::elements::{self, Formula};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;

/// Atoms the graph accepts (the localisation of the Python version stopped at 64).
pub const MAX_ATOMS: usize = 200;

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
    adj: Vec<Vec<(usize, usize)>>,
}

fn bad(smiles: &str) -> Error {
    Error::new("error.grafo.smiles").with("smiles", smiles)
}

fn normal_valence(el: &str) -> Option<u32> {
    Some(match el {
        "B" => 3,
        "C" => 4,
        "N" => 3,
        "O" => 2,
        "P" => 3,
        "S" => 2,
        "F" | "Cl" | "Br" | "I" => 1,
        _ => return None,
    })
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
        .unwrap_or_default()
}

struct RawAtom {
    el: String,
    arom: bool,
    h: Option<u32>,
    charge: i32,
}

/// `[13CH3+]`, `[nH]`, `[O-]`: isotope, element, chirality, hydrogens, charge, atom class.
fn bracket(t: &str, smiles: &str) -> Result<RawAtom> {
    let b: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let start = i;
    if i < b.len() && b[i].is_ascii_uppercase() {
        i += 1;
        if i < b.len() && b[i].is_ascii_lowercase() {
            i += 1;
        }
    } else {
        while i < b.len() && b[i].is_ascii_lowercase() && i - start < 2 {
            i += 1;
        }
    }
    let name: String = b[start..i].iter().collect();
    if name.is_empty() {
        return Err(bad(smiles));
    }
    let arom = name.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    while i < b.len() && b[i] == '@' {
        i += 1;
    }
    let mut h = 0;
    if i < b.len() && b[i] == 'H' {
        i += 1;
        let s = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        h = if s == i {
            1
        } else {
            b[s..i]
                .iter()
                .collect::<String>()
                .parse()
                .map_err(|_| bad(smiles))?
        };
    }
    let mut charge = 0;
    if i < b.len() && (b[i] == '+' || b[i] == '-') {
        let sign = if b[i] == '+' { 1 } else { -1 };
        let c0 = b[i];
        i += 1;
        let s = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        charge = if s < i {
            sign * b[s..i]
                .iter()
                .collect::<String>()
                .parse::<i32>()
                .map_err(|_| bad(smiles))?
        } else {
            let mut n = 1;
            while i < b.len() && b[i] == c0 {
                n += 1;
                i += 1;
            }
            sign * n
        };
    }
    if i < b.len() && b[i] == ':' {
        i = b.len();
    }
    if i != b.len() {
        return Err(bad(smiles));
    }
    Ok(RawAtom {
        el: cap(&name),
        arom,
        h: Some(h),
        charge,
    })
}

impl Mol {
    /// One molecule; salts and mixtures (`.`) are refused.
    pub fn parse(smiles: &str) -> Result<Mol> {
        let s: Vec<char> = smiles.trim().chars().collect();
        if s.is_empty() {
            return Err(bad(smiles));
        }
        let mut raw: Vec<RawAtom> = Vec::new();
        let mut bonds: Vec<(usize, usize, f32, bool)> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        let mut open: Vec<(String, usize, f32, bool)> = Vec::new();
        let (mut prev, mut order, mut explicit): (Option<usize>, f32, bool) = (None, 1.0, false);
        let mut p = 0;
        while p < s.len() {
            let c = s[p];
            let two: String = s[p..(p + 2).min(s.len())].iter().collect();
            let atom = if c == '[' {
                let end = s[p..]
                    .iter()
                    .position(|x| *x == ']')
                    .ok_or_else(|| bad(smiles))?
                    + p;
                let a = bracket(&s[p + 1..end].iter().collect::<String>(), smiles)?;
                p = end + 1;
                Some(a)
            } else if two == "Cl" || two == "Br" {
                p += 2;
                Some(RawAtom {
                    el: two,
                    arom: false,
                    h: None,
                    charge: 0,
                })
            } else if "BCNOPSFI".contains(c) {
                p += 1;
                Some(RawAtom {
                    el: c.to_string(),
                    arom: false,
                    h: None,
                    charge: 0,
                })
            } else if "bcnops".contains(c) {
                p += 1;
                Some(RawAtom {
                    el: cap(&c.to_string()),
                    arom: true,
                    h: None,
                    charge: 0,
                })
            } else {
                None
            };
            if let Some(a) = atom {
                raw.push(a);
                let i = raw.len() - 1;
                if let Some(pr) = prev {
                    bonds.push((pr, i, order, explicit));
                }
                prev = Some(i);
                (order, explicit) = (1.0, false);
                continue;
            }
            p += 1;
            match c {
                '(' => stack.push(prev.ok_or_else(|| bad(smiles))?),
                ')' => prev = Some(stack.pop().ok_or_else(|| bad(smiles))?),
                '=' => (order, explicit) = (2.0, true),
                '#' => (order, explicit) = (3.0, true),
                '-' => (order, explicit) = (1.0, true),
                ':' => (order, explicit) = (1.5, true),
                '/' | '\\' => {}
                '.' => return Err(Error::new("error.grafo.mixture")),
                '%' | '0'..='9' => {
                    let key = if c == '%' {
                        let k: String = s[p..(p + 2).min(s.len())].iter().collect();
                        p += 2;
                        k
                    } else {
                        c.to_string()
                    };
                    let here = prev.ok_or_else(|| bad(smiles))?;
                    if let Some(k) = open.iter().position(|o| o.0 == key) {
                        let (_, j, o, ex) = open.remove(k);
                        bonds.push((j, here, o.max(order), ex || explicit));
                    } else {
                        open.push((key, here, order, explicit));
                    }
                    (order, explicit) = (1.0, false);
                }
                _ => return Err(bad(smiles)),
            }
        }
        if !open.is_empty() || !stack.is_empty() || raw.is_empty() {
            return Err(bad(smiles));
        }
        if raw.len() > MAX_ATOMS {
            return Err(Error::new("error.grafo.tooBig").with("max", MAX_ATOMS));
        }
        for a in &raw {
            if elements::mass_of(&a.el).is_none() || normal_valence(&a.el).is_none() {
                return Err(Error::new("error.grafo.element").with("element", &a.el));
            }
        }
        let bonds: Vec<Bond> = bonds
            .into_iter()
            .map(|(a, b, mut o, ex)| {
                if o == 1.0 && !ex && raw[a].arom && raw[b].arom {
                    o = 1.5;
                }
                Bond { a, b, order: o }
            })
            .collect();
        let mut used = vec![0f32; raw.len()];
        for b in &bonds {
            let v = if b.order == 1.5 { 1.0 } else { b.order };
            used[b.a] += v;
            used[b.b] += v;
        }
        let atoms: Vec<Atom> = raw
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let h = match r.h {
                    Some(h) => h,
                    None => {
                        let extra = if r.arom && (r.el == "C" || r.el == "N") {
                            1.0
                        } else {
                            0.0
                        };
                        let u = used[i] + extra;
                        let normal = normal_valence(&r.el).unwrap_or(0) as f32;
                        let free = if u > normal && matches!(r.el.as_str(), "N" | "P" | "S") {
                            let ladder: &[f32] = if r.el == "S" { &[4.0, 6.0] } else { &[5.0] };
                            ladder.iter().find(|v| **v >= u).map_or(0.0, |v| v - u)
                        } else {
                            normal - u
                        };
                        free.round().max(0.0) as u32
                    }
                };
                Atom {
                    el: r.el.clone(),
                    arom: r.arom,
                    h,
                    charge: r.charge,
                }
            })
            .collect();
        let mut adj = vec![Vec::new(); atoms.len()];
        for (k, b) in bonds.iter().enumerate() {
            adj[b.a].push((b.b, k));
            adj[b.b].push((b.a, k));
        }
        Ok(Mol { atoms, bonds, adj })
    }

    pub fn formula(&self) -> Formula {
        self.formula_of(&vec![true; self.atoms.len()])
    }

    /// Neutral formula of the atoms in `keep` (heavy atoms and the hydrogens they carry).
    fn formula_of(&self, keep: &[bool]) -> Formula {
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

    fn el(&self, i: usize) -> &str {
        &self.atoms[i].el
    }

    fn heavy(&self, i: usize) -> usize {
        self.adj[i].len()
    }

    /// Double-bonded oxygens of `i`.
    fn dbl_o(&self, i: usize) -> usize {
        self.adj[i]
            .iter()
            .filter(|(j, k)| self.el(*j) == "O" && self.bonds[*k].order == 2.0)
            .count()
    }

    fn is_carbonyl(&self, i: usize) -> bool {
        self.el(i) == "C" && self.dbl_o(i) == 1
    }

    fn is_sulfonyl(&self, i: usize) -> bool {
        self.el(i) == "S" && self.dbl_o(i) >= 2
    }

    /// Acyl or sulfonyl carbon/sulfur: the amine and alcohol next to it are an amide, ester or sulfonamide.
    fn is_acyl(&self, i: usize) -> bool {
        self.is_carbonyl(i) || self.is_sulfonyl(i) || (self.el(i) == "S" && self.dbl_o(i) == 1)
    }

    fn sp3_c(&self, i: usize) -> bool {
        self.el(i) == "C"
            && !self.atoms[i].arom
            && self.adj[i].iter().all(|(_, k)| self.bonds[*k].order == 1.0)
    }

    /// Bonds on a ring: cutting one does not split the molecule.
    fn ring_bonds(&self) -> Vec<bool> {
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
    fn rings(&self) -> Vec<Vec<usize>> {
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

    /// Names of the functional groups present, sorted. They are the keys of the `gruppi` preconditions of `regole_tp/2`.
    pub fn groups(&self) -> Vec<&'static str> {
        let mut g: BTreeSet<&'static str> = BTreeSet::new();
        for i in 0..self.atoms.len() {
            let a = &self.atoms[i];
            let nb = &self.adj[i];
            let has_h = a.h > 0;
            match a.el.as_str() {
                "F" => {
                    g.insert("halogen");
                    g.insert("fluoro");
                }
                "Cl" => {
                    g.insert("halogen");
                    g.insert("chloro");
                }
                "Br" => {
                    g.insert("halogen");
                    g.insert("bromo");
                }
                "I" => {
                    g.insert("halogen");
                    g.insert("iodo");
                }
                "C" => {
                    if a.arom {
                        g.insert("aromatic_ring");
                    }
                    if self.is_carbonyl(i) {
                        let o_h = nb.iter().any(|(j, k)| {
                            self.el(*j) == "O"
                                && self.bonds[*k].order == 1.0
                                && (self.atoms[*j].h > 0 || self.atoms[*j].charge < 0)
                        });
                        let o_c = nb.iter().any(|(j, k)| {
                            self.el(*j) == "O"
                                && self.bonds[*k].order == 1.0
                                && self.atoms[*j].h == 0
                                && self.heavy(*j) == 2
                        });
                        let n = nb.iter().any(|(j, _)| self.el(*j) == "N");
                        let carbons = nb.iter().filter(|(j, _)| self.el(*j) == "C").count();
                        if o_h {
                            g.insert("carboxylic_acid");
                        } else if o_c {
                            g.insert("ester");
                        } else if n {
                            g.insert("amide");
                        } else if has_h || carbons == 1 && self.heavy(i) == 2 {
                            g.insert("aldehyde");
                        } else if carbons == 2 {
                            g.insert("ketone");
                        }
                    }
                    if self.sp3_c(i) && has_h && nb.iter().any(|(j, _)| self.hetero_alpha(*j)) {
                        g.insert("alpha_ch_hetero");
                    }
                }
                "N" => {
                    if self.el(i) == "N"
                        && nb.iter().filter(|(j, _)| self.el(*j) == "O").count() >= 2
                    {
                        g.insert("nitro");
                    } else if self.n_amine(i) {
                        g.insert(match (a.h, self.heavy(i)) {
                            (2, _) => "amine_primary",
                            (1, _) => "amine_secondary",
                            _ => "amine_tertiary",
                        });
                        if nb.iter().any(|(j, _)| self.atoms[*j].arom) {
                            g.insert("amine_aromatic");
                        }
                        for (j, _) in nb {
                            if self.sp3_c(*j) {
                                g.insert("n_alkyl");
                                self.alkyl_kind(*j, i, &mut g);
                            }
                        }
                    } else if nb.iter().any(|(j, _)| self.is_acyl(*j)) {
                        for (j, _) in nb {
                            if self.sp3_c(*j) {
                                g.insert("n_alkyl");
                                self.alkyl_kind(*j, i, &mut g);
                            }
                        }
                        if nb.iter().any(|(j, _)| self.is_sulfonyl(*j)) {
                            g.insert("sulfonamide");
                        }
                    } else if a.arom && nb.iter().any(|(j, _)| self.atoms[*j].arom) {
                        g.insert("heteroaromatic");
                    }
                }
                "O" => {
                    let singles = nb.iter().all(|(_, k)| self.bonds[*k].order == 1.0);
                    if a.arom {
                        g.insert("heteroaromatic");
                    } else if singles && has_h && self.heavy(i) == 1 {
                        let c = nb[0].0;
                        if self.atoms[c].arom {
                            g.insert("phenol");
                        } else if self.sp3_c(c) {
                            g.insert("alcohol");
                        }
                    } else if singles && self.heavy(i) == 2 && !has_h {
                        let (x, y) = (nb[0].0, nb[1].0);
                        if self.el(x) == "C"
                            && self.el(y) == "C"
                            && !self.is_acyl(x)
                            && !self.is_acyl(y)
                        {
                            g.insert("ether");
                            if self.sp3_c(x) || self.sp3_c(y) {
                                g.insert("o_alkyl");
                            }
                        }
                    }
                }
                "S" => {
                    if self.is_sulfonyl(i) {
                        g.insert("sulfonyl");
                    } else if self.dbl_o(i) == 1 {
                        g.insert("sulfoxide");
                    } else if self.dbl_o(i) == 0 && (self.heavy(i) == 2 || a.arom) {
                        g.insert("sulfide");
                    }
                    if a.arom {
                        g.insert("heteroaromatic");
                    }
                }
                _ => {}
            }
        }
        for r in self.rings() {
            let sat = r.iter().all(|i| !self.atoms[*i].arom)
                && self
                    .bonds
                    .iter()
                    .filter(|b| r.contains(&b.a) && r.contains(&b.b))
                    .all(|b| b.order == 1.0);
            let hetero = r.iter().any(|i| matches!(self.el(*i), "N" | "O" | "S"));
            if sat && hetero {
                g.insert("ring_hetero_saturated");
            }
        }
        g.into_iter().collect()
    }

    /// Heteroatom whose neighbouring C-H is activated (·OH abstracts there): an amine N, an ether or alcohol O.
    fn hetero_alpha(&self, j: usize) -> bool {
        match self.el(j) {
            "N" => self.n_amine(j) || self.adj[j].iter().any(|(x, _)| self.is_acyl(*x)),
            "O" => {
                !self.atoms[j].arom
                    && self.adj[j]
                        .iter()
                        .all(|(x, k)| self.bonds[*k].order == 1.0 && !self.is_acyl(*x))
            }
            _ => false,
        }
    }

    /// Amine nitrogen: not aromatic, no double bond, no charge, not next to an acyl or sulfonyl group.
    fn n_amine(&self, i: usize) -> bool {
        let a = &self.atoms[i];
        self.el(i) == "N"
            && !a.arom
            && a.charge == 0
            && self.adj[i].iter().all(|(j, k)| {
                self.bonds[*k].order == 1.0 && !self.is_acyl(*j) && self.el(*j) != "O"
            })
    }

    /// Methyl, isopropyl and tert-butyl on N (the literature rule of the N-dealkylation names them).
    fn alkyl_kind(&self, c: usize, n: usize, g: &mut BTreeSet<&'static str>) {
        let rest: Vec<usize> = self.adj[c]
            .iter()
            .map(|(j, _)| *j)
            .filter(|j| *j != n)
            .collect();
        let all_c = rest.iter().all(|j| self.sp3_c(*j));
        let h = self.atoms[c].h;
        if rest.is_empty() && h == 3 {
            g.insert("n_methyl");
        } else if all_c && rest.len() == 2 && h == 1 && rest.iter().all(|j| self.heavy(*j) == 1) {
            g.insert("n_isopropyl");
        } else if all_c && rest.len() == 3 && h == 0 && rest.iter().all(|j| self.heavy(*j) == 1) {
            g.insert("n_tbutyl");
        }
    }

    /// The atoms on the side of `bond.a` and of `bond.b` when the (non-ring) bond is cut.
    fn sides(&self, k: usize) -> (Vec<bool>, Vec<bool>) {
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

fn add_h(mut f: Formula, h: u64, o: u64) -> Formula {
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

fn hydrolysable(m: &Mol, k: usize) -> &'static str {
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

impl Mol {
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
