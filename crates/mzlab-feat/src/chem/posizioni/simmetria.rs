//! Atom equivalence classes by iterative refinement of local invariants (Morgan/CANON style).

use std::collections::BTreeMap;

use crate::chem::grafo::Mol;

/// Class id of every atom: atoms with the same id have the same element, degree, hydrogens, aromaticity and charge, and (recursively) the same multiset of
/// (bond order, neighbour class). Ids are ranks of the sorted invariants, so they are stable for a given graph.
/// Known limit: on a few regular graphs the refinement keeps together atoms that are not truly equivalent (it never separates equivalent ones); the effect
/// is only that two sites count as one candidate, and no observation is ever discarded because of it.
pub fn symmetry_classes(mol: &Mol) -> Vec<u32> {
    let n = mol.atoms.len();
    let mut cls = rank(
        (0..n)
            .map(|i| {
                let a = &mol.atoms[i];
                (a.el.clone(), mol.adj[i].len(), a.h, a.arom, a.charge)
            })
            .collect(),
    );
    let mut count = distinct(&cls);
    loop {
        let keys: Vec<(u32, Vec<(u8, u32)>)> = (0..n)
            .map(|i| {
                let mut nb: Vec<(u8, u32)> = mol.adj[i]
                    .iter()
                    .map(|(j, k)| ((mol.bonds[*k].order * 2.0) as u8, cls[*j]))
                    .collect();
                nb.sort_unstable();
                (cls[i], nb)
            })
            .collect();
        let next = rank(keys);
        let c = distinct(&next);
        cls = next;
        if c == count {
            return cls;
        }
        count = c;
    }
}

fn distinct(v: &[u32]) -> usize {
    let mut s = v.to_vec();
    s.sort_unstable();
    s.dedup();
    s.len()
}

/// Rank of each key among the sorted distinct keys.
fn rank<K: Ord + Clone>(keys: Vec<K>) -> Vec<u32> {
    let mut order: BTreeMap<K, u32> = BTreeMap::new();
    for k in &keys {
        order.entry(k.clone()).or_insert(0);
    }
    for (r, v) in order.values_mut().enumerate() {
        *v = r as u32;
    }
    keys.iter().map(|k| order[k]).collect()
}
