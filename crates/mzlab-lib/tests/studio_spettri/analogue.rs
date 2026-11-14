//! E5 of the spectral study: analogues with two modifications on disjoint subtrees of the fragmentation tree (+O on one, -Cl+H or -CH2 on
//! the other; the root carries both). The modified compound is absent from the library; its unmodified parent is the truth of the analogue search.

use super::r#gen::*;
use super::scen::{pick, q, Ctx, Q};

fn ancestor(t: &Tree, x: usize, mut y: usize) -> bool {
    while let Some(p) = t.nodes[y].parent {
        if p == x {
            return true;
        }
        y = p;
    }
    false
}

/// Ions that carry a modification placed on ion `a`: `a`, its ancestors and its descendants.
fn carriers(t: &Tree, a: usize) -> Vec<bool> {
    (0..t.nodes.len())
        .map(|i| i == a || ancestor(t, i, a) || ancestor(t, a, i))
        .collect()
}

/// E5 analogue of `t`: +O on the subtree of one ion and -Cl+H (or -CH2) on a disjoint one; the root carries both. `None` when no pair works.
pub fn modify(r: &mut R, t: &Tree) -> Option<(Tree, Vec<f64>, &'static str)> {
    let n = t.nodes.len() as i32;
    let plus_o: F = [0, 0, 0, 1, 0, 0, 0];
    for _ in 0..40 {
        let (a, b) = (r.int(1, n - 1) as usize, r.int(1, n - 1) as usize);
        if a == b || ancestor(t, a, b) || ancestor(t, b, a) {
            continue;
        }
        let (sa, sb) = (carriers(t, a), carriers(t, b));
        let has_cl = (0..n as usize)
            .filter(|&i| sb[i])
            .all(|i| t.nodes[i].f[6] >= 1);
        let (d2, name): (F, &str) = if has_cl {
            ([0, 1, 0, 0, 0, 0, -1], "-Cl+H")
        } else {
            ([-1, -2, 0, 0, 0, 0, 0], "-CH2")
        };
        let mut m = t.clone();
        for i in 0..n as usize {
            if sa[i] {
                m.nodes[i].f = add(&m.nodes[i].f, &plus_o);
            }
            if sb[i] {
                m.nodes[i].f = add(&m.nodes[i].f, &d2);
            }
        }
        let ok = m
            .nodes
            .iter()
            .all(|x| valid_ion(&x.f) && ion_mz(&x.f) >= 50.0);
        let distinct = (0..m.nodes.len()).all(|i| (0..i).all(|j| m.nodes[i].f != m.nodes[j].f));
        if ok && distinct {
            return Some((m, vec![mass(&plus_o), mass(&d2)], name));
        }
    }
    None
}

pub fn e5(c: &mut Ctx, n_base: usize, n: usize) -> Vec<Q> {
    let mut out = Vec::new();
    for k in 0..n {
        let mut r = c.rng();
        for j in 0.. {
            let id = (pick(n_base, k, n) + j * 13) % n_base;
            if let Some((t, d, name)) = modify(&mut r, &c.lib[id].tree) {
                let mut x = q(
                    measure(&mut r, &t, 35.0),
                    Some(id),
                    format!("e5/{name}"),
                    t.nodes[0].f,
                );
                x.deltas = d;
                out.push(x);
                break;
            }
        }
    }
    out
}
