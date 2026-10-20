//! Aromaticity of a record, perceived from its Kekulé form: a flag derived from the structure, never stored in it. Our rule (Hückel 4n+2 on rings
//! of 5 to 7 atoms and on pairs of such rings fused along one bond), not another toolkit's model; `Mol::aromatic_bonds` is what the graph view
//! of the engines (`mzlab-feat::chem::grafo`) uses to keep the flag it always had from lower-case SMILES.

use crate::model::Mol;
use std::collections::{BTreeSet, VecDeque};

impl Mol {
    /// For every entry of `bonds`: whether the bond belongs to an aromatic ring.
    pub fn aromatic_bonds(&self) -> Vec<bool> {
        let g = Graph::new(self);
        let rings = g.rings();
        let mut flag = vec![false; self.bonds.len()];
        let small: Vec<&Ring> = rings
            .iter()
            .filter(|r| (5..=7).contains(&r.atoms.len()))
            .collect();
        for r in &small {
            if huckel(g.electrons(&r.atoms)) {
                r.bonds.iter().for_each(|&b| flag[b] = true);
            }
        }
        // two rings that share one bond (azulene: 5 + 7 with 10 electrons; imidazo[1,2-a]pyridine: 5 + 6 with 10 around a bridgehead N)
        for (i, x) in small.iter().enumerate() {
            for y in &small[i + 1..] {
                let shared = x.bonds.iter().filter(|b| y.bonds.contains(b)).count();
                if shared != 1 {
                    continue;
                }
                let union: BTreeSet<usize> = x.atoms.iter().chain(&y.atoms).copied().collect();
                let union: Vec<usize> = union.into_iter().collect();
                if huckel(g.electrons(&union)) {
                    // the bond the two rings share is aromatic only if a ring is aromatic alone
                    x.bonds
                        .iter()
                        .chain(&y.bonds)
                        .filter(|b| !(x.bonds.contains(b) && y.bonds.contains(b)))
                        .for_each(|&b| flag[b] = true);
                }
            }
        }
        flag
    }

    /// For every atom: whether it sits in an aromatic ring.
    pub fn aromatic_atoms(&self) -> Vec<bool> {
        let mut out = vec![false; self.atoms.len()];
        for (b, _) in self.aromatic_bonds().iter().enumerate().filter(|(_, &f)| f) {
            out[self.bonds[b].a as usize] = true;
            out[self.bonds[b].b as usize] = true;
        }
        out
    }
}

impl<'a> Graph<'a> {
    fn new(m: &'a Mol) -> Self {
        let mut adj = vec![Vec::new(); m.atoms.len()];
        for (k, b) in m.bonds.iter().enumerate() {
            adj[b.a as usize].push((b.b as usize, k));
            adj[b.b as usize].push((b.a as usize, k));
        }
        let mut g = Graph {
            m,
            adj,
            in_ring: vec![false; m.atoms.len()],
        };
        for r in g.rings() {
            r.atoms.iter().for_each(|&a| g.in_ring[a] = true);
        }
        g
    }

    /// The shortest ring through every ring bond, each ring once (sorted by atoms).
    fn rings(&self) -> Vec<Ring> {
        let mut seen: BTreeSet<Vec<usize>> = BTreeSet::new();
        let mut out = Vec::new();
        for (k, b) in self.m.bonds.iter().enumerate() {
            let Some(path) = self.shortest(b.a as usize, b.b as usize, k) else {
                continue;
            };
            let mut key = path.clone();
            key.sort_unstable();
            if !seen.insert(key.clone()) {
                continue;
            }
            let bonds = self.ring_bonds(&path);
            out.push(Ring { atoms: key, bonds });
        }
        out
    }

    /// Atoms of the shortest path from `s` to `t` that does not use bond `skip`.
    fn shortest(&self, s: usize, t: usize, skip: usize) -> Option<Vec<usize>> {
        let mut prev = vec![usize::MAX; self.adj.len()];
        prev[s] = s;
        let mut queue = VecDeque::from([s]);
        while let Some(u) = queue.pop_front() {
            if u == t {
                let mut path = vec![t];
                while *path.last()? != s {
                    path.push(prev[*path.last()?]);
                }
                return Some(path);
            }
            for &(v, k) in &self.adj[u] {
                if k != skip && prev[v] == usize::MAX {
                    prev[v] = u;
                    queue.push_back(v);
                }
            }
        }
        None
    }

    fn ring_bonds(&self, path: &[usize]) -> Vec<usize> {
        let mut out = Vec::new();
        for (i, &a) in path.iter().enumerate() {
            let b = path[(i + 1) % path.len()];
            if let Some(&(_, k)) = self.adj[a].iter().find(|&&(v, _)| v == b) {
                out.push(k);
            }
        }
        out.sort_unstable();
        out
    }

    /// π electrons of the atoms taken together, or `None` when one of them cannot be part of an aromatic ring.
    fn electrons(&self, atoms: &[usize]) -> Option<usize> {
        atoms
            .iter()
            .map(|&a| self.donated(a))
            .sum::<Option<usize>>()
    }

    /// π electrons one ring atom gives: 1 for a double bond inside the ring system, 2 for a lone pair, 0 for an empty p orbital or a double bond
    /// to an exocyclic hetero atom or carbon of a carbonyl-like group.
    fn donated(&self, a: usize) -> Option<usize> {
        let at = &self.m.atoms[a];
        if at.rad != 0 || !self.in_ring[a] {
            return None;
        }
        let double: Vec<usize> = self.adj[a]
            .iter()
            .filter(|&&(_, k)| self.m.bonds[k].o == 2)
            .map(|&(v, _)| v)
            .collect();
        if self.adj[a].iter().any(|&(_, k)| self.m.bonds[k].o == 3) || double.len() > 1 {
            return None;
        }
        if let Some(&v) = double.first() {
            if self.in_ring[v] {
                return Some(1);
            }
            let exo = self.m.atoms[v].el.as_str();
            return (at.el == "C" && matches!(exo, "O" | "S" | "N" | "Se")).then_some(0);
        }
        let total = self.adj[a].len() + at.h as usize;
        match (at.el.as_str(), at.chg) {
            ("C", -1) if total == 3 => Some(2),
            ("C", 1) | ("B", 0) if total == 3 => Some(0),
            ("N" | "P" | "As", 0) | ("N", -1) if total == 3 => Some(2),
            ("O" | "S" | "Se", 0) if total == 2 => Some(2),
            _ => None,
        }
    }
}

struct Ring {
    atoms: Vec<usize>,
    bonds: Vec<usize>,
}

struct Graph<'a> {
    m: &'a Mol,
    /// (neighbour, bond index)
    adj: Vec<Vec<(usize, usize)>>,
    in_ring: Vec<bool>,
}

fn huckel(electrons: Option<usize>) -> bool {
    electrons.is_some_and(|e| e % 4 == 2)
}
