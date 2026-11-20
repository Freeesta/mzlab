//! Geometry of what the tools build: where a new bond sprouts from an atom, and the atoms and bonds of a ring template placed on
//! the canvas, on an atom or fused on a bond. Lengths are in bond lengths (1.0 = one standard bond).

use crate::doc::Document;
use crate::m;
use crate::ops::{Op, OpError};
use std::f64::consts::{PI, TAU};

/// Atoms closer than this are the same atom when a template lands on existing ones.
const MERGE: f64 = 0.25;

/// The direction (radians, y up) a new bond takes from an atom: 30° on a bare atom, a zigzag after one bond, otherwise the middle of
/// the widest free angle.
pub fn sprout_angle(doc: &Document, id: u32) -> f64 {
    let Some(a) = doc.atom(id) else {
        return PI / 6.0;
    };
    let mut ang: Vec<f64> = doc
        .neighbours(id)
        .iter()
        .filter_map(|&n| doc.atom(n))
        .map(|n| m::atan2(n.y - a.y, n.x - a.x).rem_euclid(TAU))
        .collect();
    ang.sort_by(f64::total_cmp);
    match ang.len() {
        0 => PI / 6.0,
        1 => (ang[0] + TAU / 3.0).rem_euclid(TAU),
        _ => {
            let (mut best, mut gap) = (0.0, -1.0);
            for i in 0..ang.len() {
                let next = if i + 1 < ang.len() {
                    ang[i + 1]
                } else {
                    ang[0] + TAU
                };
                if next - ang[i] > gap {
                    gap = next - ang[i];
                    best = ang[i] + gap / 2.0;
                }
            }
            best.rem_euclid(TAU)
        }
    }
}

/// Corner positions of the ring, first to last, and whether the first corners are existing atoms (their ids).
struct Frame {
    pts: Vec<(f64, f64)>,
    fixed: Vec<u32>,
}

fn polygon(cx: f64, cy: f64, r: f64, start: f64, n: usize, turn: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let t = start + turn * TAU * i as f64 / n as f64;
            (cx + r * m::cos(t), cy + r * m::sin(t))
        })
        .collect()
}

fn frame(
    doc: &Document,
    n: usize,
    atom: Option<u32>,
    bond: Option<u32>,
    x: f64,
    y: f64,
) -> Result<Frame, OpError> {
    let half = PI / n as f64;
    if let Some(id) = bond {
        let b = doc.bond(id).ok_or(OpError {
            key: "error.draw2.noBond",
            id,
        })?;
        let (p, q) = (doc.atom(b.a), doc.atom(b.b));
        let (Some(p), Some(q)) = (p, q) else {
            return Err(OpError {
                key: "error.draw2.noBond",
                id,
            });
        };
        let (dx, dy) = (q.x - p.x, q.y - p.y);
        let len = m::hypot(dx, dy).max(1e-6);
        // the side of the bond with fewer substituents takes the ring
        let side: f64 = [b.a, b.b]
            .iter()
            .flat_map(|&e| doc.neighbours(e))
            .filter(|&m| m != b.a && m != b.b)
            .filter_map(|m| doc.atom(m))
            .map(|m| (dx * (m.y - p.y) - dy * (m.x - p.x)).signum())
            .sum();
        let s = if side > 0.0 { -1.0 } else { 1.0 };
        let apothem = len / (2.0 * m::tan(half));
        let (cx, cy) = (
            (p.x + q.x) / 2.0 - s * dy / len * apothem,
            (p.y + q.y) / 2.0 + s * dx / len * apothem,
        );
        let start = m::atan2(p.y - cy, p.x - cx);
        // corners turn so that q is the corner after p
        let pts = polygon(cx, cy, len / (2.0 * m::sin(half)), start, n, s);
        return Ok(Frame {
            pts,
            fixed: vec![b.a, b.b],
        });
    }
    let r = 1.0 / (2.0 * m::sin(half));
    if let Some(id) = atom {
        let a = doc.atom(id).ok_or(OpError {
            key: "error.draw2.noAtom",
            id,
        })?;
        let dir = sprout_angle(doc, id);
        let (cx, cy) = (a.x + r * m::cos(dir), a.y + r * m::sin(dir));
        return Ok(Frame {
            pts: polygon(cx, cy, r, dir + PI, n, 1.0),
            fixed: vec![id],
        });
    }
    Ok(Frame {
        pts: polygon(x, y, r, PI / 2.0, n, 1.0),
        fixed: Vec::new(),
    })
}

/// The primitive operations of a `ring` operation. A corner that falls on an existing atom reuses it; a side that exists is kept.
pub(crate) fn ring(doc: &Document, op: &Op) -> Result<Vec<Op>, OpError> {
    let Op::Ring {
        n,
        arom,
        atom,
        bond,
        x,
        y,
    } = op
    else {
        return Ok(Vec::new());
    };
    if !(3..=8).contains(n) {
        return Err(OpError {
            key: "error.draw2.ring",
            id: *n as u32,
        });
    }
    let n = *n as usize;
    let f = frame(doc, n, *atom, *bond, *x, *y)?;
    let mut ops = Vec::new();
    let mut next = doc.next;
    let mut ids = Vec::with_capacity(n);
    for (i, &(px, py)) in f.pts.iter().enumerate() {
        let near = doc
            .atoms
            .iter()
            .find(|a| m::hypot(a.x - px, a.y - py) < MERGE)
            .map(|a| a.id);
        let id = f.fixed.get(i).copied().or(near).unwrap_or_else(|| {
            ops.push(Op::AtomAdd {
                id: Some(next),
                el: "C".into(),
                x: px,
                y: py,
            });
            next += 1;
            next - 1
        });
        ids.push(id);
    }
    // alternate double bonds only where both ends are still free of one, starting so that an existing double bond keeps its place
    let has_double = |id: u32| {
        doc.bonds
            .iter()
            .any(|b| (b.a == id || b.b == id) && b.o > 1)
    };
    let mut busy: Vec<u32> = ids.iter().copied().filter(|&id| has_double(id)).collect();
    for i in 0..n {
        let (a, b) = (ids[i], ids[(i + 1) % n]);
        if a == b || doc.bond_between(a, b).is_some() {
            continue;
        }
        let double = *arom && !busy.contains(&a) && !busy.contains(&b);
        if double {
            busy.extend([a, b]);
        }
        ops.push(Op::BondAdd {
            id: Some(next),
            a,
            b,
            o: if double { 2 } else { 1 },
        });
        next += 1;
    }
    Ok(ops)
}
