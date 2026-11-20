//! Overlap removal and double-bond geometry on a finished layout: a penalty for atoms too close, atoms on bonds and crossing bonds;
//! reflecting or turning the subtree behind an acyclic bond when that lowers it; reflecting a subtree to give a defined double bond its
//! written cis/trans. Everything is greedy and in ascending index order, so the result is deterministic.

use super::trig::*;
use super::G;

type Pos = Vec<Option<V>>;

/// Work the overlap cleanup may spend on one piece, in inner-loop steps of `penalty` (about 0.2 s native); a molecule that is not
/// cleaned up in that budget keeps the layout it has and `G::cut` says so.
pub const WORK_BUDGET: u64 = 40_000_000;

/// Overlap cost of the placed atoms, counting only terms that involve an atom flagged in `mv`. The cost of a call is the size of the
/// flagged part times the size of the molecule, not the molecule squared; the steps taken are added to `g.work`.
pub fn penalty(g: &G, pos: &Pos, mv: &[bool]) -> f64 {
    let nb = g.bonds.len();
    let movers: Vec<usize> = (0..g.n).filter(|&i| mv[i] && pos[i].is_some()).collect();
    let mover_bonds: Vec<usize> = (0..nb)
        .filter(|&k| mv[g.bonds[k].0] || mv[g.bonds[k].1])
        .collect();
    let mut in_mb = vec![false; nb];
    mover_bonds.iter().for_each(|&k| in_mb[k] = true);
    let mut p = 0.0;
    let mut steps = 0u64;
    for &i in &movers {
        let Some(a) = pos[i] else { continue };
        for j in 0..g.n {
            // a pair of two flagged atoms is counted once, from its higher index
            if j == i || (mv[j] && j > i) || g.adj[i].contains(&j) {
                continue;
            }
            if let Some(b) = pos[j] {
                let d = dist(a, b);
                if d < 0.85 {
                    p += (0.85 - d) * (0.85 - d) * 40.0;
                }
            }
        }
        p += atom_on_bonds(g, pos, i, a, 0..nb);
        steps += (g.n + nb) as u64;
    }
    for i in (0..g.n).filter(|&i| !mv[i]) {
        if let Some(a) = pos[i] {
            p += atom_on_bonds(g, pos, i, a, mover_bonds.iter().copied());
            steps += mover_bonds.len() as u64;
        }
    }
    for &k in &mover_bonds {
        let (a, b) = g.bonds[k];
        let (Some(pa), Some(pb)) = (pos[a], pos[b]) else {
            continue;
        };
        for (m, &(c, d)) in g.bonds.iter().enumerate() {
            // a pair of two bonds at a flagged atom is counted once, from its higher index
            if m == k || (in_mb[m] && m > k) || a == c || a == d || b == c || b == d {
                continue;
            }
            if let (Some(pc), Some(pd)) = (pos[c], pos[d])
                && crosses(pa, pb, pc, pd)
            {
                p += 25.0;
            }
        }
        steps += nb as u64;
    }
    g.work.set(g.work.get() + steps);
    p
}

/// The cost of the atom `i` (at `a`) lying on the bonds `of`, other than its own.
fn atom_on_bonds(g: &G, pos: &Pos, i: usize, a: V, of: impl Iterator<Item = usize>) -> f64 {
    let mut p = 0.0;
    for k in of {
        let (x, y) = g.bonds[k];
        if x == i || y == i {
            continue;
        }
        if let (Some(u), Some(w)) = (pos[x], pos[y]) {
            let d = seg_dist(a, u, w);
            if d < 0.55 {
                p += (0.55 - d) * (0.55 - d) * 80.0;
            }
        }
    }
    p
}

/// The atoms behind the bond `a`-`b`, on the side of `b`; `None` when the bond lies in a ring.
pub fn side_of(g: &G, a: usize, b: usize) -> Option<Vec<bool>> {
    let mut seen = vec![false; g.n];
    seen[a] = true;
    seen[b] = true;
    let mut st = vec![b];
    while let Some(x) = st.pop() {
        for &y in &g.adj[x] {
            if y == a {
                if x != b {
                    return None;
                }
                continue;
            }
            if !seen[y] {
                seen[y] = true;
                st.push(y);
            }
        }
    }
    seen[a] = false;
    Some(seen)
}

fn apply(pos: &mut Pos, mask: &[bool], f: impl Fn(V) -> V) {
    for (i, m) in mask.iter().enumerate() {
        if *m && let Some(p) = pos[i] {
            pos[i] = Some(f(p));
        }
    }
}

/// Whether the mask may move (no frozen atom in it).
fn movable(g: &G, mask: &[bool]) -> bool {
    !mask.iter().zip(&g.frozen).any(|(m, f)| *m && *f)
}

/// The mover for the bond `x`-`y`: the smaller side that holds no frozen atom, as (pivot, mask).
fn mover(g: &G, x: usize, y: usize) -> Option<(usize, usize, Vec<bool>)> {
    let sy = side_of(g, x, y)?;
    let sx = side_of(g, y, x)?;
    let (cy, cx) = (
        sy.iter().filter(|m| **m).count(),
        sx.iter().filter(|m| **m).count(),
    );
    let order = if cy <= cx {
        [(x, y, sy), (y, x, sx)]
    } else {
        [(y, x, sx), (x, y, sy)]
    };
    order.into_iter().find(|(_, _, m)| movable(g, m))
}

fn is_defined_db(g: &G, x: usize, y: usize) -> bool {
    g.dbs
        .iter()
        .any(|d| (d.a == x && d.b == y) || (d.a == y && d.b == x))
}

/// Reflects and, if `turn`, also turns subtrees behind acyclic bonds while the penalty falls.
pub fn tidy(g: &mut G) {
    g.work.set(0);
    let all = vec![true; g.n];
    if penalty(g, &g.pos, &all) < 1e-9 {
        return;
    }
    for round in 0..2 {
        for _ in 0..6 {
            let mut better = false;
            for k in 0..g.bonds.len() {
                if g.work.get() > WORK_BUDGET {
                    g.cut = true;
                    return;
                }
                let (x, y) = g.bonds[k];
                if is_defined_db(g, x, y) {
                    continue;
                }
                let Some((a, b, mask)) = mover(g, x, y) else {
                    continue;
                };
                let (Some(pa), Some(pb)) = (g.pos[a], g.pos[b]) else {
                    continue;
                };
                let base = penalty(g, &g.pos, &mask);
                let mut best = (base, None);
                let mut tries: Vec<Pos> = Vec::new();
                let mut t = g.pos.clone();
                apply(&mut t, &mask, |p| reflect(p, pa, pb));
                tries.push(t);
                if round == 1 {
                    for deg in [-60.0f64, 60.0, -30.0, 30.0, 180.0] {
                        let sc = sincos(deg * PI / 180.0);
                        let mut t = g.pos.clone();
                        apply(&mut t, &mask, |p| add(pa, rot(sub(p, pa), sc)));
                        tries.push(t);
                    }
                }
                for t in tries {
                    let pen = penalty(g, &t, &mask);
                    if pen < best.0 - 1e-6 {
                        best = (pen, Some(t));
                    }
                }
                if let Some(t) = best.1 {
                    g.pos = t;
                    better = true;
                }
            }
            if !better {
                break;
            }
        }
        if penalty(g, &g.pos, &all) < 1e-9 {
            return;
        }
    }
}

/// Whether the drawn double bond `d` is cis (both refs on one side of its axis).
pub fn drawn_cis(pos: &Pos, d: &super::Db) -> Option<bool> {
    let (a, b) = (pos[d.a]?, pos[d.b]?);
    let ax = sub(b, a);
    let (s0, s1) = (cross(ax, sub(pos[d.r0]?, a)), cross(ax, sub(pos[d.r1]?, b)));
    Some(s0 * s1 > 0.0)
}

/// Reflects the subtree behind each defined acyclic double bond whose drawn cis/trans is wrong.
pub fn fix_ez(g: &mut G) {
    for k in 0..g.dbs.len() {
        let d = g.dbs[k].clone();
        if drawn_cis(&g.pos, &d) != Some(d.trans) {
            continue;
        }
        let Some((a, b, mask)) = mover(g, d.a, d.b) else {
            continue;
        };
        let (Some(pa), Some(pb)) = (g.pos[a], g.pos[b]) else {
            continue;
        };
        apply(&mut g.pos, &mask, |p| reflect(p, pa, pb));
    }
}

/// The defined double bonds (by index in `g.dbs`) drawn with the wrong cis/trans.
pub fn ez_wrong(g: &G, pos: &Pos) -> Vec<usize> {
    (0..g.dbs.len())
        .filter(|&k| drawn_cis(pos, &g.dbs[k]) == Some(g.dbs[k].trans))
        .collect()
}
