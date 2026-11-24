//! Canonical atom order of a record: partition refinement on the constitution, then a search over the tied atoms that keeps the smallest
//! core JSON, so the stereo elements are the last invariant and two writings of one molecule give the same record byte for byte.
//! Automorphisms prune the search: those of each component and the swaps of equal components are known before it starts, the others are
//! found on the way, and their orbits at a node grow in a union-find, so many equal components (a TP import) cost a polynomial. Many
//! pieces of one constitution with different stereo would cost every interleaving: past a bound the pieces are ordered alone and sorted.
//! The order is ours and versioned by `CAN_VERSION`; it is not RDKit's or InChI's.

use crate::model::Mol;
use crate::sortm::SortM;
use std::cell::Cell;
use std::collections::HashMap;

/// Prefix of `Mol::can`; changes when the algorithm changes, so stored `props` can be told apart and remapped.
pub const CAN_VERSION: &str = "mol1";
/// Leaves of the search tree visited at most; past it the best leaf so far is used (only molecules with huge symmetry get there).
const LEAF_LIMIT: usize = 20_000;
/// Largest whole search (in leaves, counted without labels) kept for records with tied units of different stereo; past it the units
/// are ordered alone, so many of them cost linear time and the order stays canonical where the search would stop at `LEAF_LIMIT`.
const SEARCH_SIZE: u64 = 1_000;

impl Mol {
    /// New index of every atom in the canonical order: a permutation, the argument of `renumbered`.
    pub fn canonical_order(&self) -> Vec<u32> {
        self.canonical_order_counted().0
    }

    /// The canonical order and the work units it took (refinement rounds, leaves, orbit steps and checks, each counted per atom touched).
    pub(crate) fn canonical_order_counted(&self) -> (Vec<u32>, u64) {
        let mut s = Search::new(self);
        let start = s.refine(s.initial());
        match s.by_units(&start) {
            Units::Order(order) => (order, s.work.get()),
            Units::Autos(autos) => {
                s.autos = autos;
                s.explore(start, &mut Vec::new());
                let order = s
                    .best
                    .map_or_else(|| (0..self.atoms.len() as u32).collect(), |(_, p)| p);
                (order, s.work.get())
            }
        }
    }

    /// Constitution classes (rank of every atom after refinement, stereo ignored): label independent, equal for atoms no bond pattern separates.
    pub(crate) fn constitution_classes(&self) -> Vec<usize> {
        let s = Search::new(self);
        s.refine(s.initial())
    }

    /// The same molecule with its atoms in canonical order (layers moved along, `id` kept as it is).
    pub fn canonical(&self) -> Mol {
        self.canonical_counted().0
    }

    /// The canonical record and the work units its order took.
    pub(crate) fn canonical_counted(&self) -> (Mol, u64) {
        let (order, work) = self.canonical_order_counted();
        (
            self.renumbered(&order).unwrap_or_else(|_| self.clone()),
            work,
        )
    }

    /// Identity string of the molecule: the version and the core hash of the canonical record; stereo (and its status) is part of it.
    pub fn can(&self) -> String {
        format!("{CAN_VERSION}:{}", self.canonical().core_hash_hex())
    }
}

impl<'a> Search<'a> {
    fn new(mol: &'a Mol) -> Search<'a> {
        let mut adj = vec![Vec::new(); mol.atoms.len()];
        for b in &mol.bonds {
            adj[b.a as usize].push((b.b as usize, b.o));
            adj[b.b as usize].push((b.a as usize, b.o));
        }
        Search {
            mol,
            adj,
            leaves: HashMap::new(),
            best: None,
            autos: Vec::new(),
            visited: 0,
            work: Cell::new(0),
        }
    }

    /// What the units (connected pieces joined by the enhanced-stereo groups that cross them) give before any search. Each unit is
    /// searched alone (equal writings once). Two units the refinement ties (the same classes) but with different canonical records
    /// (equal constitution, different stereo) make the search try every interleaving of them; past `SEARCH_SIZE` leaves the order is
    /// then the units sorted by canonical record, each in its own order, one after the other. Otherwise the whole search runs.
    fn by_units(&self, cls: &[usize]) -> Units {
        let (label, count) = self.units();
        if count < 2 {
            return Units::Autos(self.component_autos(None));
        }
        let Some(units) = self.searched(&label, count) else {
            return Units::Autos(Vec::new());
        };
        let (members, found, known) = &units;
        // Units with equal refinement classes (as sorted lists) are the ones the search cannot tell apart. The size of the whole
        // search does not depend on the labels: the interleavings of tied units with different records (a multinomial per set of tied
        // units) times the distinct leaves of every unit. A small one keeps the search, and with it the order it always gave.
        let sig: Vec<Vec<usize>> = members
            .iter()
            .map(|m| {
                let mut c: Vec<usize> = m.iter().map(|&x| cls[x]).collect();
                c.sortm();
                c
            })
            .collect();
        let cert = |k: usize| &known[&found[k]];
        let mut by_sig: Vec<usize> = (0..count).collect();
        by_sig.sortm_by(|&x, &y| (&sig[x], &cert(x).0).cmp(&(&sig[y], &cert(y).0)));
        let (mut apart, mut size, mut total, mut same) = (false, 1u64, 0u64, 0u64);
        for (w, &k) in by_sig.iter().enumerate() {
            let prev = w.checked_sub(1).map(|v| by_sig[v]);
            let tied = prev.is_some_and(|j| sig[j] == sig[k]);
            let equal = tied && prev.is_some_and(|j| cert(j).0 == cert(k).0);
            apart |= tied && !equal;
            total = if tied { total + 1 } else { 1 };
            same = if equal { same + 1 } else { 1 };
            // exact below the bound; past it the value stays past it (every factor is at least 1)
            size = (size * total / same)
                .saturating_mul(cert(k).3)
                .min(SEARCH_SIZE + 1);
        }
        if !apart || size <= SEARCH_SIZE {
            let same = count == self.mol.piece_labels().1;
            return Units::Autos(self.component_autos(if same { Some(units) } else { None }));
        }
        let mut by_cert: Vec<usize> = (0..count).collect();
        by_cert.sortm_by(|&a, &b| known[&found[a]].0.cmp(&known[&found[b]].0));
        let mut order = vec![0u32; cls.len()];
        let mut at = 0u32;
        for k in by_cert {
            for (&x, &l) in members[k].iter().zip(&known[&found[k]].1) {
                order[x] = at + l;
            }
            at += members[k].len() as u32;
        }
        Units::Order(order)
    }

    /// Each part (atoms with one `label`) searched alone, equal writings once: the atoms of every part in ascending order, the
    /// written core JSON of every part, and what the search of each writing gives.
    fn searched(&self, label: &[usize], count: usize) -> Option<Searched> {
        let mut members = vec![Vec::new(); count];
        for (x, &k) in label.iter().enumerate() {
            members[k].push(x);
        }
        let mut known: HashMap<String, Piece> = HashMap::new();
        let mut found = Vec::new();
        for p in self.mol.part_cores(label, count)? {
            let written = p.core_json();
            if !known.contains_key(&written) {
                let mut s = Search::new(&p);
                let start = s.refine(s.initial());
                s.explore(start, &mut Vec::new());
                self.work.set(self.work.get() + s.work.get());
                let (cert, order) = s.best.unwrap_or_default();
                let distinct = s.leaves.len() as u64;
                known.insert(written.clone(), (cert, order, s.autos, distinct));
            }
            found.push(written);
        }
        Some((members, found, known))
    }

    /// Automorphisms the connected components give before any search: the symmetries of each component (from its own search) and the
    /// swap of two components with the same canonical record (`pieces`: their searches when already made). Each is kept only if the whole
    /// record renumbered by it gives the same core JSON, so a group across components can only drop one; they prune whole subtrees whose
    /// leaves repeat certificates already seen, so the first smallest leaf, and with it the order, is the one the plain search finds.
    fn component_autos(&self, pieces: Option<Searched>) -> Vec<Vec<usize>> {
        let pieces = match pieces {
            Some(p) => p,
            None => {
                let (label, count) = self.mol.piece_labels();
                if count < 2 {
                    return Vec::new();
                }
                let Some(p) = self.searched(&label, count) else {
                    return Vec::new();
                };
                p
            }
        };
        let (members, found, known) = &pieces;
        let n = self.adj.len();
        let mut candidates: Vec<Vec<usize>> = Vec::new();
        let mut last_of: HashMap<&str, usize> = HashMap::new();
        for (k, written) in found.iter().enumerate() {
            let (cert, order, autos, _) = &known[written];
            for g in autos {
                let mut whole: Vec<usize> = (0..n).collect();
                for (x, &y) in g.iter().enumerate() {
                    whole[members[k][x]] = members[k][y];
                }
                candidates.push(whole);
            }
            if let Some(j) = last_of.insert(cert.as_str(), k) {
                let other: (&[usize], &[u32]) = (&members[j], &known[&found[j]].1);
                candidates.push(swap(n, other, (&members[k], order)));
            }
        }
        self.verified(candidates)
    }

    /// The unit of every atom (connected pieces joined by the groups over more than one of them), numbered in the order of its first
    /// atom, and the number of units.
    fn units(&self) -> (Vec<usize>, usize) {
        let (piece, count) = self.mol.piece_labels();
        let mut join: Vec<usize> = (0..count).collect();
        fn root(join: &mut [usize], mut x: usize) -> usize {
            while join[x] != x {
                join[x] = join[join[x]];
                x = join[x];
            }
            x
        }
        for g in &self.mol.groups {
            for w in g.atoms.windows(2) {
                let (a, b) = (
                    root(&mut join, piece[w[0] as usize]),
                    root(&mut join, piece[w[1] as usize]),
                );
                join[a.max(b)] = a.min(b);
            }
        }
        // pieces are numbered in the order of their first atom and a root is the smallest piece, so roots keep that order
        let mut number = vec![usize::MAX; count];
        let mut units = 0;
        let mut label = vec![0usize; piece.len()];
        for (x, &p) in piece.iter().enumerate() {
            let r = root(&mut join, p);
            if number[r] == usize::MAX {
                number[r] = units;
                units += 1;
            }
            label[x] = number[r];
        }
        (label, units)
    }

    /// The candidates that are automorphisms of the whole record (the same core JSON once renumbered by them).
    fn verified(&self, mut candidates: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
        let n = self.adj.len();
        let identity: Vec<u32> = (0..n as u32).collect();
        let Ok(same) = self.mol.renumbered(&identity).map(|m| m.core_json()) else {
            return Vec::new();
        };
        candidates.retain(|g| {
            self.work
                .set(self.work.get() + (n + self.mol.bonds.len()) as u64);
            let moved: Vec<u32> = g.iter().map(|&x| x as u32).collect();
            self.mol
                .renumbered(&moved)
                .is_ok_and(|m| m.core_json() == same)
        });
        candidates
    }

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
        let n = cls.len();
        let mut count = {
            let mut d = cls.clone();
            d.sortm();
            d.dedup();
            d.len()
        };
        // Neighbour keys live in one flat buffer (one slice per atom) and the ranking sorts atom
        // indices; a round allocates only the index lists of the ranking sort (the short neighbour slots sort on the stack).
        let mut start = vec![0usize; n + 1];
        for i in 0..n {
            start[i + 1] = start[i] + self.adj[i].len();
        }
        let mut near = vec![(0usize, 0u8); start[n]];
        let mut order: Vec<usize> = (0..n).collect();
        loop {
            for i in 0..n {
                let slot = &mut near[start[i]..start[i + 1]];
                for (s, &(j, o)) in slot.iter_mut().zip(&self.adj[i]) {
                    *s = (cls[j], o);
                }
                slot.sortm();
            }
            let key = |i: usize| (cls[i], &near[start[i]..start[i + 1]]);
            order.sortm_by(|&x, &y| key(x).cmp(&key(y)));
            let mut next = vec![0usize; n];
            let mut now = 0;
            for w in 0..n {
                if w > 0 && key(order[w - 1]) != key(order[w]) {
                    now += 1;
                }
                next[order[w]] = now;
            }
            let now = if n == 0 { 0 } else { now + 1 };
            self.work.set(self.work.get() + (n + start[n]) as u64);
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
        let mut orbits = Orbits::new(n);
        for v in members {
            if self.visited >= LEAF_LIMIT {
                return;
            }
            orbits.absorb(&self.autos, prefix, &self.work);
            if done.iter().any(|&w| orbits.same(w, v, &self.work)) {
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
        self.work
            .set(self.work.get() + (perm.len() + self.mol.bonds.len()) as u64);
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
            self.autos.push(perm.iter().map(|&p| by_label[p]).collect());
            return;
        }
        if self.best.as_ref().is_none_or(|(b, _)| cert < *b) {
            self.best = Some((cert.clone(), new_of_old.clone()));
        }
        self.leaves.insert(cert, new_of_old);
    }
}

/// What the search of one unit gives: its certificate, its canonical order, its automorphisms and its distinct leaves.
type Piece = (String, Vec<u32>, Vec<Vec<usize>>, u64);

/// Parts searched alone: the atoms of each, the written core JSON of each, and the search of every distinct writing.
type Searched = (Vec<Vec<usize>>, Vec<String>, HashMap<String, Piece>);

/// The order of the whole record from its units alone, or the automorphisms they seed the whole search with.
enum Units {
    Order(Vec<u32>),
    Autos(Vec<Vec<usize>>),
}

struct Search<'a> {
    mol: &'a Mol,
    adj: Vec<Vec<(usize, u8)>>,
    /// Certificate (core JSON) of every leaf seen, with its labelling.
    leaves: HashMap<String, Vec<u32>>,
    best: Option<(String, Vec<u32>)>,
    autos: Vec<Vec<usize>>,
    visited: usize,
    work: Cell<u64>,
}

/// Orbits, at one node, of the automorphisms found so far that fix every atom of the node's prefix: the same relation the closure
/// of those automorphisms gives, kept in a union-find that takes each automorphism once instead of re-filtering all of them per sibling.
struct Orbits {
    parent: Vec<usize>,
    /// Automorphisms already looked at (the list only grows while the node's siblings are explored).
    seen: usize,
}

impl Orbits {
    fn new(n: usize) -> Orbits {
        Orbits {
            parent: (0..n).collect(),
            seen: 0,
        }
    }

    /// Joins the cycles of every new automorphism that fixes the prefix.
    fn absorb(&mut self, autos: &[Vec<usize>], prefix: &[usize], work: &Cell<u64>) {
        for g in &autos[self.seen..] {
            work.set(work.get() + prefix.len() as u64);
            if !prefix.iter().all(|&p| g[p] == p) {
                continue;
            }
            for (x, &y) in g.iter().enumerate() {
                let (a, b) = (self.root(x, work), self.root(y, work));
                if a != b {
                    self.parent[a.max(b)] = a.min(b);
                }
            }
        }
        self.seen = autos.len();
    }

    fn same(&mut self, w: usize, v: usize, work: &Cell<u64>) -> bool {
        self.root(w, work) == self.root(v, work)
    }

    /// Root of `x`, halving the path on the way.
    fn root(&mut self, mut x: usize, work: &Cell<u64>) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
            work.set(work.get() + 1);
        }
        work.set(work.get() + 1);
        x
    }
}

/// The swap of two components with the same canonical record: atoms with the same canonical label trade places. Each side is the
/// component's atoms and its canonical order (new index of each of them).
fn swap(
    n: usize,
    (atoms_a, order_a): (&[usize], &[u32]),
    (atoms_b, order_b): (&[usize], &[u32]),
) -> Vec<usize> {
    let mut at = vec![0usize; order_a.len()];
    for (x, &l) in order_a.iter().enumerate() {
        at[l as usize] = atoms_a[x];
    }
    let mut whole: Vec<usize> = (0..n).collect();
    for (x, &l) in order_b.iter().enumerate() {
        let (a, b) = (atoms_b[x], at[l as usize]);
        whole[a] = b;
        whole[b] = a;
    }
    whole
}

/// Dense rank of every key among the sorted distinct keys.
fn rank<K: Ord + Clone>(keys: &[K]) -> Vec<usize> {
    let mut sorted: Vec<&K> = keys.iter().collect();
    sorted.sortm();
    sorted.dedup();
    keys.iter()
        .map(|k| sorted.binary_search(&k).unwrap_or(0))
        .collect()
}

#[cfg(test)]
#[path = "canon_tests.rs"]
mod tests;
