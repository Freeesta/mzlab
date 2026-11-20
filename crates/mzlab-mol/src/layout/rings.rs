//! Ring perception for the layout: a smallest set of smallest rings (one shortest cycle through each ring bond, both tie orders, then
//! GF(2) independence) as ordered atom cycles, and the ring systems (rings that share two or more atoms). Deterministic: neighbours are
//! always visited in ascending atom order.

use std::collections::VecDeque;

/// The shortest path from `u` to `v` that does not use the bond `u`-`v`, neighbours tried ascending or descending.
fn shortest_cycle(adj: &[Vec<usize>], u: usize, v: usize, desc: bool) -> Option<Vec<usize>> {
    let mut prev = vec![usize::MAX; adj.len()];
    prev[u] = u;
    let mut q = VecDeque::from([u]);
    while let Some(x) = q.pop_front() {
        let nb: Vec<usize> = if desc {
            adj[x].iter().rev().copied().collect()
        } else {
            adj[x].clone()
        };
        for y in nb {
            if (x == u && y == v) || prev[y] != usize::MAX {
                continue;
            }
            prev[y] = x;
            if y == v {
                let mut path = vec![v];
                let mut c = v;
                while c != u {
                    c = prev[c];
                    path.push(c);
                }
                path.reverse();
                return Some(path);
            }
            q.push_back(y);
        }
    }
    None
}

/// The bond (edge) id of every adjacency entry: ascending pairs numbered in order.
fn edge_ids(adj: &[Vec<usize>]) -> (Vec<Vec<usize>>, usize) {
    let mut ids: Vec<Vec<usize>> = adj.iter().map(|a| vec![0; a.len()]).collect();
    let mut n = 0;
    for a in 0..adj.len() {
        for (k, &b) in adj[a].iter().enumerate() {
            if a < b {
                ids[a][k] = n;
                let j = adj[b].iter().position(|&x| x == a).unwrap_or(0);
                ids[b][j] = n;
                n += 1;
            }
        }
    }
    (ids, n)
}

fn components(adj: &[Vec<usize>]) -> usize {
    let mut seen = vec![false; adj.len()];
    let mut c = 0;
    for s in 0..adj.len() {
        if seen[s] {
            continue;
        }
        c += 1;
        let mut st = vec![s];
        seen[s] = true;
        while let Some(x) = st.pop() {
            for &y in &adj[x] {
                if !seen[y] {
                    seen[y] = true;
                    st.push(y);
                }
            }
        }
    }
    c
}

/// A smallest set of smallest rings, each as the atoms in cycle order, shortest first.
pub fn sssr(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let (ids, ne) = edge_ids(adj);
    let rank = (ne + components(adj)).saturating_sub(adj.len());
    if rank == 0 {
        return Vec::new();
    }
    let mut cands: Vec<Vec<usize>> = Vec::new();
    for u in 0..adj.len() {
        for &v in adj[u].iter().filter(|&&v| v > u) {
            for desc in [false, true] {
                if let Some(p) = shortest_cycle(adj, u, v, desc) {
                    let mut key = p.clone();
                    key.sort_unstable();
                    if !cands.iter().any(|c| {
                        let mut k = c.clone();
                        k.sort_unstable();
                        k == key
                    }) {
                        cands.push(p);
                    }
                }
            }
        }
    }
    cands.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    let words = ne.div_ceil(64);
    let mut basis: Vec<Vec<u64>> = Vec::new();
    let mut out = Vec::new();
    for c in cands {
        let mut bits = vec![0u64; words];
        for i in 0..c.len() {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            let k = adj[a].iter().position(|&x| x == b).unwrap_or(0);
            let e = ids[a][k];
            bits[e / 64] ^= 1 << (e % 64);
        }
        if reduce(&mut basis, bits) {
            out.push(c);
            if out.len() == rank {
                break;
            }
        }
    }
    out
}

/// Adds `bits` to the GF(2) basis when independent of it.
fn reduce(basis: &mut Vec<Vec<u64>>, mut bits: Vec<u64>) -> bool {
    for b in basis.iter() {
        let p = b.iter().position(|&w| w != 0).unwrap_or(0);
        let low = b[p].trailing_zeros();
        if bits[p] >> low & 1 == 1 {
            for (x, y) in bits.iter_mut().zip(b) {
                *x ^= y;
            }
        }
    }
    if bits.iter().all(|&w| w == 0) {
        return false;
    }
    basis.push(bits);
    true
}

/// Ring systems: groups of ring indices connected by sharing two or more atoms, ordered by their first ring.
pub fn systems(rings: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut sys: Vec<usize> = (0..rings.len()).collect();
    fn find(s: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while s[r] != r {
            r = s[r];
        }
        s[x] = r;
        r
    }
    for i in 0..rings.len() {
        for j in 0..i {
            let shared = rings[i].iter().filter(|a| rings[j].contains(a)).count();
            if shared >= 2 {
                let (a, b) = (find(&mut sys, i), find(&mut sys, j));
                sys[a.max(b)] = a.min(b);
            }
        }
    }
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut root_of: Vec<usize> = Vec::new();
    for i in 0..rings.len() {
        let r = find(&mut sys, i);
        match root_of.iter().position(|&x| x == r) {
            Some(k) => out[k].push(i),
            None => {
                root_of.push(r);
                out.push(vec![i]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adj_of(n: usize, bonds: &[(usize, usize)]) -> Vec<Vec<usize>> {
        let mut a = vec![Vec::new(); n];
        for &(x, y) in bonds {
            a[x].push(y);
            a[y].push(x);
        }
        a.iter_mut().for_each(|v| v.sort_unstable());
        a
    }

    #[test]
    fn cubane_has_five_four_rings_and_naphthalene_two_six_rings() {
        let cube = [
            (0, 1),
            (1, 2),
            (2, 3),
            (0, 3),
            (4, 5),
            (5, 6),
            (6, 7),
            (4, 7),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        let r = sssr(&adj_of(8, &cube));
        assert_eq!(r.len(), 5);
        assert!(r.iter().all(|x| x.len() == 4));
        let nap = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 4),
            (4, 5),
            (5, 0),
            (4, 6),
            (6, 7),
            (7, 8),
            (8, 9),
            (9, 5),
        ];
        let r = sssr(&adj_of(10, &nap));
        assert_eq!(r.len(), 2);
        assert_eq!(systems(&r).len(), 1);
    }
}
