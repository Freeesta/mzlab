//! Stereo of a molfile draft: tetrahedral centres from wedges (2D only), "either" marks as `unknown` elements, double bonds from the
//! geometry of the drawing, and the enhanced-stereo collections. Nothing here decides whether a drawn element is really stereogenic.

use crate::codec::MolError;
use crate::model::{Atom, Cfg, Parity, Status, Stereo, StereoGroup};
use crate::molfile::{fail, Draft, Wedge};
use std::collections::VecDeque;

/// Neighbours of every atom with the index of the joining bond.
fn adjacency(d: &Draft) -> Vec<Vec<(usize, usize)>> {
    let mut adj = vec![Vec::new(); d.atoms.len()];
    for (i, b) in d.bonds.iter().enumerate() {
        adj[b.a].push((b.b, i));
        adj[b.b].push((b.a, i));
    }
    adj
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn drawn(d: &Draft) -> bool {
    d.atoms.iter().any(|a| a.xyz != [0.0; 3])
}

fn flat(d: &Draft) -> bool {
    d.atoms.iter().all(|a| a.xyz[2] == 0.0)
}

/// Tetrahedral centres: a wedge starting at the atom defines it (2D drawings only), a wavy bond starting at it makes it `unknown`.
/// The atom needs three neighbours and one hydrogen or lone pair, or four neighbours.
pub(crate) fn tetra(d: &Draft, atoms: &[Atom]) -> Vec<Stereo> {
    let adj = adjacency(d);
    let mut out = Vec::new();
    for (c, a) in atoms.iter().enumerate() {
        let k = adj[c].len();
        if !((k == 3 && a.h <= 1) || (k == 4 && a.h == 0)) {
            continue;
        }
        let starts: Vec<(usize, Wedge)> = adj[c]
            .iter()
            .filter(|&&(_, bi)| d.bonds[bi].a == c)
            .map(|&(n, bi)| (n, d.bonds[bi].wedge))
            .filter(|&(_, w)| w != Wedge::None)
            .collect();
        if starts.is_empty() {
            continue;
        }
        let known = flat(d) && drawn(d) && !starts.iter().any(|&(_, w)| w == Wedge::Either);
        let par = if known {
            parity(d, c, &adj[c], &starts)
        } else {
            None
        };
        let (refs, par, status) = match par {
            Some(p) => {
                let mut refs: Vec<u32> = adj[c].iter().map(|&(n, _)| n as u32).collect();
                refs.sort_unstable();
                (Some(refs), Some(p), Status::Defined)
            }
            None if starts.iter().any(|&(_, w)| w == Wedge::Either) => {
                (None, None, Status::Unknown)
            }
            None => continue,
        };
        let implicit_h = (status == Status::Defined).then_some(k == 3 && a.h == 1);
        out.push(Stereo::Tetra {
            c: c as u32,
            refs,
            implicit_h,
            par,
            status,
        });
    }
    out
}

/// Parity of the neighbours in ascending order (implicit hydrogen or lone pair last) from the wedges: neighbours below the plane get a
/// negative height, above a positive one. None when the drawing does not decide (flat arrangement).
fn parity(
    d: &Draft,
    c: usize,
    nbrs: &[(usize, usize)],
    wedges: &[(usize, Wedge)],
) -> Option<Parity> {
    let mut ns: Vec<usize> = nbrs.iter().map(|&(n, _)| n).collect();
    ns.sort_unstable();
    let mut v: Vec<[f64; 3]> = ns
        .iter()
        .map(|&n| {
            let mut r = sub(d.atoms[n].xyz, d.atoms[c].xyz);
            let len = (r[0] * r[0] + r[1] * r[1]).sqrt();
            r[2] = match wedges.iter().find(|&&(w, _)| w == n).map(|&(_, w)| w) {
                Some(Wedge::Up) => len,
                Some(Wedge::Down) => -len,
                _ => 0.0,
            };
            r
        })
        .collect();
    if v.len() == 3 {
        v.push([
            -(v[0][0] + v[1][0] + v[2][0]),
            -(v[0][1] + v[1][1] + v[2][1]),
            -(v[0][2] + v[1][2] + v[2][2]),
        ]);
    }
    let vol = dot(sub(v[0], v[3]), cross(sub(v[1], v[3]), sub(v[2], v[3])));
    let scale: f64 = v.iter().map(|x| dot(*x, *x)).sum::<f64>().max(1e-12);
    if vol.abs() < 1e-6 * scale.powf(1.5) {
        return None;
    }
    Some(if vol > 0.0 { Parity::Ccw } else { Parity::Cw })
}

/// Whether the double bond `a`-`b` lies in a ring of at most seven atoms (where cis/trans is not stereo).
fn in_small_ring(adj: &[Vec<(usize, usize)>], a: usize, b: usize, skip: usize) -> bool {
    let mut dist = vec![usize::MAX; adj.len()];
    dist[a] = 0;
    let mut queue = VecDeque::from([a]);
    while let Some(x) = queue.pop_front() {
        for &(y, bi) in &adj[x] {
            if bi == skip || dist[y] != usize::MAX {
                continue;
            }
            dist[y] = dist[x] + 1;
            if y == b {
                return dist[y] < 7;
            }
            queue.push_back(y);
        }
    }
    false
}

/// Double bonds with a substituent on both ends: `unknown` when the file marks the bond "either", else cis/trans from the drawing.
/// Bonds that were aromatic, in rings of up to seven atoms, or with a collinear substituent are left out.
pub(crate) fn double_bonds(d: &Draft) -> Vec<Stereo> {
    let adj = adjacency(d);
    let mut ends: Vec<(usize, usize, usize)> = d
        .bonds
        .iter()
        .enumerate()
        .filter(|(_, b)| b.order == 2 && !b.arom)
        .map(|(i, b)| (b.a.min(b.b), b.a.max(b.b), i))
        .collect();
    ends.sort_unstable();
    let mut out = Vec::new();
    for (a, b, bi) in ends {
        let low =
            |x: usize, skip: usize| adj[x].iter().map(|&(n, _)| n).filter(|&n| n != skip).min();
        let (Some(ra), Some(rb)) = (low(a, b), low(b, a)) else {
            continue;
        };
        if in_small_ring(&adj, a, b, bi) {
            continue;
        }
        if d.bonds[bi].wedge == Wedge::Either {
            out.push(Stereo::Db {
                a: a as u32,
                b: b as u32,
                refs: None,
                cfg: None,
                status: Status::Unknown,
            });
        } else if let Some(cfg) = geometry(d, (a, b), (ra, rb)) {
            out.push(Stereo::Db {
                a: a as u32,
                b: b as u32,
                refs: Some([ra as u32, rb as u32]),
                cfg: Some(cfg),
                status: Status::Defined,
            });
        }
    }
    out
}

/// cis when the two substituents lie on the same side of the double bond, from their parts perpendicular to its axis.
fn geometry(d: &Draft, (a, b): (usize, usize), (ra, rb): (usize, usize)) -> Option<Cfg> {
    if !drawn(d) {
        return None;
    }
    let p = |i: usize| d.atoms[i].xyz;
    let axis = sub(p(b), p(a));
    let len = dot(axis, axis).sqrt();
    if len < 1e-9 {
        return None;
    }
    let perp = |from: usize, to: usize| {
        let v = sub(p(to), p(from));
        let along = dot(v, axis) / (len * len);
        [
            v[0] - along * axis[0],
            v[1] - along * axis[1],
            v[2] - along * axis[2],
        ]
    };
    let (w1, w2) = (perp(a, ra), perp(b, rb));
    let (n1, n2) = (dot(w1, w1).sqrt(), dot(w2, w2).sqrt());
    if n1 < 0.05 * len || n2 < 0.05 * len {
        return None;
    }
    Some(if dot(w1, w2) > 0.0 {
        Cfg::Cis
    } else {
        Cfg::Trans
    })
}

/// Enhanced-stereo groups: the collections of the file, restricted to atoms that are defined tetrahedral centres here.
pub(crate) fn groups(d: &Draft, stereo: &[Stereo]) -> Result<Vec<StereoGroup>, MolError> {
    let defined = |a: usize| {
        stereo.iter().any(
            |s| matches!(s, Stereo::Tetra { c, status: Status::Defined, .. } if *c as usize == a),
        )
    };
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for (kind, list) in &d.collections {
        let mut atoms: Vec<u32> = Vec::new();
        for &a in list {
            if a >= d.atoms.len() {
                return Err(fail(format!(
                    "collection names atom {a}, outside the molecule"
                )));
            }
            if defined(a) {
                if seen.contains(&a) {
                    return Err(fail(format!("atom {a} is in two stereo collections")));
                }
                seen.push(a);
                atoms.push(a as u32);
            }
        }
        atoms.sort_unstable();
        if !atoms.is_empty() {
            out.push(StereoGroup { kind: *kind, atoms });
        }
    }
    out.sort_by(|x, y| x.atoms.cmp(&y.atoms));
    Ok(out)
}
