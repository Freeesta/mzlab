//! Canonical atom order of a record: partition refinement on the constitution, then a search over the tied atoms that keeps the smallest
//! core JSON, so the stereo elements are the last invariant and two writings of one molecule give the same record byte for byte.
//! Automorphisms found on the way prune the search. The order is ours and versioned by `CAN_VERSION`; it is not RDKit's or InChI's.

use crate::model::Mol;
use std::collections::HashMap;

/// Prefix of `Mol::can`; changes when the algorithm changes, so stored `props` can be told apart and remapped.
pub const CAN_VERSION: &str = "mol1";
/// Leaves of the search tree visited at most; past it the best leaf so far is used (only molecules with huge symmetry get there).
const LEAF_LIMIT: usize = 20_000;

impl Mol {
    /// New index of every atom in the canonical order: a permutation, the argument of `renumbered`.
    pub fn canonical_order(&self) -> Vec<u32> {
        let n = self.atoms.len();
        let mut adj = vec![Vec::new(); n];
        for b in &self.bonds {
            adj[b.a as usize].push((b.b as usize, b.o));
            adj[b.b as usize].push((b.a as usize, b.o));
        }
        let mut s = Search {
            mol: self,
            adj,
            leaves: HashMap::new(),
            best: None,
            autos: Vec::new(),
            visited: 0,
        };
        let start = s.refine(s.initial());
        s.explore(start, &mut Vec::new());
        s.best.map_or_else(|| (0..n as u32).collect(), |(_, p)| p)
    }

    /// Constitution classes (rank of every atom after refinement, stereo ignored): label independent, equal for atoms no bond pattern separates.
    pub(crate) fn constitution_classes(&self) -> Vec<usize> {
        let mut adj = vec![Vec::new(); self.atoms.len()];
        for b in &self.bonds {
            adj[b.a as usize].push((b.b as usize, b.o));
            adj[b.b as usize].push((b.a as usize, b.o));
        }
        let s = Search {
            mol: self,
            adj,
            leaves: HashMap::new(),
            best: None,
            autos: Vec::new(),
            visited: 0,
        };
        s.refine(s.initial())
    }

    /// The same molecule with its atoms in canonical order (layers moved along, `id` kept as it is).
    pub fn canonical(&self) -> Mol {
        let order = self.canonical_order();
        self.renumbered(&order).unwrap_or_else(|_| self.clone())
    }

    /// Identity string of the molecule: the version and the core hash of the canonical record; stereo (and its status) is part of it.
    pub fn can(&self) -> String {
        format!("{CAN_VERSION}:{}", self.canonical().core_hash_hex())
    }
}

impl Search<'_> {
    /// Ranks of the atom invariants (element, isotope, charge, radical, hydrogens, degree).
    fn initial(&self) -> Vec<usize> {
        let key = |i: usize| {
            let a = &self.mol.atoms[i];
            (a.el.clone(), a.iso, a.chg, a.rad, a.h, self.adj[i].len())
        };
        let keys: Vec<_> = (0..self.adj.len()).map(key).collect();
        rank(&keys)
    }

    /// Equitable refinement: classes split by the multiset of (neighbour class, bond order) until no class splits any more.
    fn refine(&self, mut cls: Vec<usize>) -> Vec<usize> {
        let mut count = cls.iter().collect::<std::collections::BTreeSet<_>>().len();
        loop {
            let keys: Vec<(usize, Vec<(usize, u8)>)> = (0..cls.len())
                .map(|i| {
                    let mut near: Vec<(usize, u8)> =
                        self.adj[i].iter().map(|&(j, o)| (cls[j], o)).collect();
                    near.sort_unstable();
                    (cls[i], near)
                })
                .collect();
            let next = rank(&keys);
            let now = next.iter().max().map_or(0, |m| m + 1);
            cls = next;
            if now == count {
                return cls;
            }
            count = now;
        }
    }

    /// Visits the tree of individualisations; at a discrete partition the renumbered core JSON is the certificate.
    fn explore(&mut self, cls: Vec<usize>, prefix: &mut Vec<usize>) {
        let n = cls.len();
        let mut size = vec![0usize; n];
        for &c in &cls {
            size[c] += 1;
        }
        let Some(target) = (0..n).find(|&c| size[c] > 1) else {
            self.leaf(cls);
            return;
        };
        let members: Vec<usize> = (0..n).filter(|&i| cls[i] == target).collect();
        let mut done: Vec<usize> = Vec::new();
        for v in members {
            if self.visited >= LEAF_LIMIT {
                return;
            }
            if done.iter().any(|&w| self.same_orbit(prefix, w, v)) {
                continue;
            }
            let split: Vec<usize> = (0..n)
                .map(|i| cls[i] * 2 + usize::from(cls[i] == target && i != v))
                .collect();
            prefix.push(v);
            let next = self.refine(split);
            self.explore(next, prefix);
            prefix.pop();
            done.push(v);
        }
    }

    fn leaf(&mut self, perm: Vec<usize>) {
        self.visited += 1;
        let new_of_old: Vec<u32> = perm.iter().map(|&p| p as u32).collect();
        let Ok(m) = self.mol.renumbered(&new_of_old) else {
            return;
        };
        let cert = m.core_json();
        if let Some(first) = self.leaves.get(&cert) {
            // the same certificate through two labellings: their composition is an automorphism of the record
            let mut by_label = vec![0usize; perm.len()];
            for (old, &l) in first.iter().enumerate() {
                by_label[l as usize] = old;
            }
            self.autos
                .push(perm.iter().map(|&p| by_label[p]).collect());
            return;
        }
        if self.best.as_ref().is_none_or(|(b, _)| cert < *b) {
            self.best = Some((cert.clone(), new_of_old.clone()));
        }
        self.leaves.insert(cert, new_of_old);
    }

    /// Whether `v` and `w` are in one orbit of the automorphisms found so far that fix every atom of `prefix`.
    fn same_orbit(&self, prefix: &[usize], w: usize, v: usize) -> bool {
        let mut seen = vec![w];
        let mut i = 0;
        while i < seen.len() {
            let x = seen[i];
            i += 1;
            for g in self.autos.iter().filter(|g| prefix.iter().all(|&p| g[p] == p)) {
                if g[x] == v {
                    return true;
                }
                if !seen.contains(&g[x]) {
                    seen.push(g[x]);
                }
            }
        }
        false
    }
}

struct Search<'a> {
    mol: &'a Mol,
    adj: Vec<Vec<(usize, u8)>>,
    /// Certificate (core JSON) of every leaf seen, with its labelling.
    leaves: HashMap<String, Vec<u32>>,
    best: Option<(String, Vec<u32>)>,
    autos: Vec<Vec<usize>>,
    visited: usize,
}

/// Dense rank of every key among the sorted distinct keys.
fn rank<K: Ord + Clone>(keys: &[K]) -> Vec<usize> {
    let mut sorted: Vec<&K> = keys.iter().collect();
    sorted.sort_unstable();
    sorted.dedup();
    keys.iter()
        .map(|k| sorted.binary_search(&k).unwrap_or(0))
        .collect()
}
