//! Growing a layout outward from placed atoms, breadth first: the ring systems that touch an atom are placed whole, then its other
//! neighbours go into the largest empty angle (a chain continues as a 120-degree zigzag, a linear atom straight on, three or more
//! neighbours are spread evenly).

use std::collections::VecDeque;

use super::place::{largest_gap, place_system, placed_angles};
use super::trig::*;
use super::G;

/// The neighbour directions (angles) for `k` new bonds on `a`, which has the placed bonds `have` and is `linear` or not.
fn directions(g: &G, a: usize, have: &[f64], k: usize) -> Vec<f64> {
    match (have.len(), k) {
        (0, 1) => vec![0.0],
        (0, _) => (0..k).map(|j| TAU * j as f64 / k as f64).collect(),
        (1, 1) => {
            let t = if g.linear[a] { PI } else { 2.0 * PI / 3.0 };
            vec![have[0] + t * zigzag(g, a)]
        }
        _ => {
            let (s, sz) = largest_gap(have);
            (1..=k)
                .map(|j| s + sz * j as f64 / (k + 1) as f64)
                .collect()
        }
    }
}

/// +1 or -1: the side that keeps the chain `g`-`p`-`a`-new in a zigzag (the new atom opposite the grandparent), +1 when unknown.
fn zigzag(g: &G, a: usize) -> f64 {
    let pos = &g.pos;
    let Some(pa) = pos[a] else { return 1.0 };
    let Some(p) = g.adj[a].iter().copied().find(|&n| pos[n].is_some()) else {
        return 1.0;
    };
    let Some(pp) = pos[p] else { return 1.0 };
    let Some(gp) = g.adj[p]
        .iter()
        .copied()
        .find(|&n| n != a && pos[n].is_some())
    else {
        return 1.0;
    };
    let side = cross(sub(pa, pp), sub(pos[gp].unwrap_or(pp), pp));
    // the new atom lies at the angle from `a` towards `p` plus or minus 120 degrees: it is on the left of p->a when that is minus
    if side > 1e-9 {
        1.0
    } else if side < -1e-9 {
        -1.0
    } else {
        1.0
    }
}

/// Places every neighbour of `a` that is still unplaced; returns the atoms newly placed.
fn grow_atom(g: &mut G, a: usize) -> Vec<usize> {
    let before: Vec<bool> = g.pos.iter().map(|p| p.is_some()).collect();
    let mut sys: Vec<usize> = g.rings_of[a].iter().map(|&r| g.ring_sys[r]).collect();
    sys.sort_unstable();
    sys.dedup();
    for s in sys {
        let open = g.systems[s]
            .iter()
            .any(|&r| g.rings[r].iter().any(|&x| g.pos[x].is_none()));
        if open {
            place_system(g, s);
        }
    }
    let todo: Vec<usize> = g.adj[a]
        .iter()
        .copied()
        .filter(|&n| g.pos[n].is_none())
        .collect();
    if !todo.is_empty() {
        let pa = g.pos[a].unwrap_or([0.0, 0.0]);
        let have = placed_angles(g, &g.pos, a);
        for (n, ang) in todo.iter().zip(directions(g, a, &have, todo.len())) {
            g.pos[*n] = Some(add(pa, unit(ang)));
        }
    }
    (0..g.n)
        .filter(|&i| !before[i] && g.pos[i].is_some())
        .collect()
}

/// Grows from the atoms in `queue` until nothing is left to place.
pub fn expand(g: &mut G, queue: &mut VecDeque<usize>) {
    while let Some(a) = queue.pop_front() {
        let fresh = grow_atom(g, a);
        queue.extend(fresh);
    }
}
