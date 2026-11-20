//! Ring placement. A ring with no placed atom is a regular polygon (a trans double bond in a macrocycle makes it a Z-kink closed by an
//! arc); with one placed atom it is a polygon hanging off that atom (substituent or spiro ring); with two or more, the unplaced atoms of
//! each run between placed ones go on an equal-chord circular arc (a fused ring is the regular polygon, a bridge a shallow arc).

use super::clean::penalty;
use super::trig::*;
use super::G;
use crate::sortm::SortM;

type Pos = Vec<Option<V>>;

/// Start and size of the largest empty angle among `angles` (counter-clockwise from the start); the whole circle if none.
pub fn largest_gap(angles: &[f64]) -> (f64, f64) {
    // a NaN has no place in an order (it would make the comparison inconsistent): it is left out, so the sort is total
    let mut a: Vec<f64> = angles.iter().copied().filter(|x| !x.is_nan()).collect();
    a.sortm_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    match a.len() {
        0 => (0.0, TAU),
        1 => (a[0], TAU),
        n => {
            let mut best = (a[n - 1], a[0] + TAU - a[n - 1]);
            for i in 0..n - 1 {
                let g = a[i + 1] - a[i];
                if g > best.1 + 1e-9 {
                    best = (a[i], g);
                }
            }
            best
        }
    }
}

/// Angles of the placed neighbours of `a` as seen from `a`.
pub fn placed_angles(g: &G, pos: &Pos, a: usize) -> Vec<f64> {
    let Some(p) = pos[a] else { return Vec::new() };
    g.adj[a]
        .iter()
        .filter_map(|&n| pos[n].map(|q| angle(sub(q, p))))
        .collect()
}

/// Circumradius of a regular polygon of `n` sides of length one.
fn radius(n: usize) -> f64 {
    1.0 / (2.0 * sincos(PI / n as f64).0)
}

/// `m` points from `a` to `b` on an equal-chord arc of segments of length one (longer if `a`-`b` is nearly as long as the path),
/// bulging to the left of `a`->`b` when `left`.
pub fn arc(a: V, b: V, m: usize, left: bool) -> Vec<V> {
    let segs = (m + 1) as f64;
    let d = dist(a, b);
    let l = if d > 0.92 * segs {
        d / (0.92 * segs)
    } else {
        1.0
    };
    let target = d / l;
    let (mut lo, mut hi) = (1e-9, PI - 1e-9);
    for _ in 0..60 {
        let t = (lo + hi) / 2.0;
        let gt = sincos(t).0 / sincos(t / segs).0;
        if gt > target {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) / 2.0;
    let r = l / (2.0 * sincos(t / segs).0);
    let mid = scale(add(a, b), 0.5);
    let ab = sub(b, a);
    let nrm = if d > 0.0 {
        scale([-ab[1], ab[0]], 1.0 / d)
    } else {
        [0.0, 1.0]
    };
    let nrm = if left { nrm } else { scale(nrm, -1.0) };
    let centre = sub(mid, scale(nrm, r * sincos(t).1));
    let ca = sub(a, centre);
    let step = 2.0 * t / segs;
    // the rotation direction that carries `a` to `b` after the whole arc (2t)
    let cb = sub(b, centre);
    let (plus, minus) = (rot(ca, sincos(2.0 * t)), rot(ca, sincos(-2.0 * t)));
    let sgn = if dist(plus, cb) <= dist(minus, cb) {
        1.0
    } else {
        -1.0
    };
    (1..=m)
        .map(|j| add(centre, rot(ca, sincos(sgn * step * j as f64))))
        .collect()
}

/// A free ring: regular polygon about the origin, first atom up; mirrored when `mirror`.
fn polygon_free(r: &[usize], pos: &mut Pos, mirror: bool) {
    let n = r.len();
    let (rad, s) = (radius(n), if mirror { -1.0 } else { 1.0 });
    for (j, &a) in r.iter().enumerate() {
        pos[a] = Some(scale(unit(PI / 2.0 + s * j as f64 * TAU / n as f64), rad));
    }
}

/// A ring with one placed atom `k` of the cycle: regular polygon whose centre lies away from that atom's other bonds.
fn polygon_hung(g: &G, r: &[usize], k: usize, pos: &mut Pos, mirror: bool) {
    let n = r.len();
    let p = pos[r[k]].unwrap_or([0.0, 0.0]);
    let (gs, gz) = largest_gap(&placed_angles(g, pos, r[k]));
    let away = unit(gs + gz / 2.0);
    let centre = add(p, scale(away, radius(n)));
    let base = angle(sub(p, centre));
    let s = if mirror { -1.0 } else { 1.0 };
    for j in 1..n {
        let a = r[(k + j) % n];
        pos[a] = Some(add(
            centre,
            scale(unit(base + s * j as f64 * TAU / n as f64), radius(n)),
        ));
    }
}

/// A ring needing one trans double bond `r[i]=r[i+1]` (ring of 8 or more): the Z of four atoms, the rest on a closing arc.
fn polygon_kink(r: &[usize], i: usize, pos: &mut Pos) {
    let n = r.len();
    let at = |k: usize| r[(i + n + k - 1) % n];
    let pts = [
        [-0.5, 0.866_025_403_784_438_6],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.5, -0.866_025_403_784_438_6],
    ];
    for (k, p) in pts.iter().enumerate() {
        pos[at(k)] = Some(*p);
    }
    let rest = arc(pts[3], pts[0], n - 4, true);
    for (k, p) in rest.into_iter().enumerate() {
        pos[at(4 + k)] = Some(p);
    }
}

fn ring_placed(r: &[usize], pos: &Pos) -> usize {
    r.iter().filter(|&&a| pos[a].is_some()).count()
}

/// Fills the unplaced runs of a ring that has two or more placed atoms.
fn fuse_ring(g: &G, r: &[usize], sys_atoms: &[usize], pos: &mut Pos) {
    let n = r.len();
    let pts: Vec<V> = sys_atoms.iter().filter_map(|&a| pos[a]).collect();
    let c = scale(
        pts.iter().fold([0.0, 0.0], |s, &p| add(s, p)),
        1.0 / pts.len() as f64,
    );
    let Some(start) = (0..n).find(|&i| pos[r[i]].is_some() && pos[r[(i + n - 1) % n]].is_none())
    else {
        return;
    };
    let mut i = 0;
    while i < n {
        let at = |k: usize| r[(start + k) % n];
        if pos[at(i)].is_some() {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < n && pos[at(j)].is_none() {
            j += 1;
        }
        let (a, b) = (
            pos[at(i - 1)].unwrap_or([0.0; 2]),
            pos[at(j % n)].unwrap_or([0.0; 2]),
        );
        let (ab, mid) = (sub(b, a), scale(add(a, b), 0.5));
        let left = cross(ab, sub(c, mid)) <= 0.0;
        let mut best: Option<(f64, Vec<V>)> = None;
        for side in [left, !left] {
            let cand = arc(a, b, j - i, side);
            let mut trial = pos.clone();
            let mut mv = vec![false; g.n];
            for (k, p) in cand.iter().enumerate() {
                trial[at(i + k)] = Some(*p);
                mv[at(i + k)] = true;
            }
            let pen = penalty(g, &trial, &mv);
            if best.as_ref().is_none_or(|b| pen < b.0 - 1e-9) {
                best = Some((pen, cand));
            }
            if pen < 1e-9 {
                break;
            }
        }
        if let Some((_, cand)) = best {
            for (k, p) in cand.into_iter().enumerate() {
                pos[at(i + k)] = Some(p);
            }
        }
        i = j;
    }
}

/// Places every unplaced atom of ring system `s` into `pos`, the first ring mirrored when `mirror`.
fn fuse_all(g: &G, s: usize, pos: &mut Pos, mirror: bool) {
    let rings = &g.systems[s];
    let mut atoms: Vec<usize> = rings
        .iter()
        .flat_map(|&r| g.rings[r].iter().copied())
        .collect();
    atoms.sortm();
    atoms.dedup();
    let mut first = true;
    loop {
        let mut best: Option<(usize, usize)> = None;
        for &ri in rings {
            let r = &g.rings[ri];
            let c = ring_placed(r, pos);
            if c < r.len() && best.is_none_or(|b| c > b.1) {
                best = Some((ri, c));
            }
        }
        let Some((ri, c)) = best else { return };
        let r = &g.rings[ri];
        match c {
            0 => match g.kink_at(ri) {
                Some(i) => polygon_kink(r, i, pos),
                None => polygon_free(r, pos, mirror),
            },
            1 => {
                let k = (0..r.len()).find(|&k| pos[r[k]].is_some()).unwrap_or(0);
                polygon_hung(g, r, k, pos, mirror && first);
            }
            _ => fuse_ring(g, r, &atoms, pos),
        }
        first = false;
    }
}

/// Places the unplaced atoms of ring system `s` (both mirror images of the first ring tried, the one with less overlap kept).
pub fn place_system(g: &mut G, s: usize) {
    let mut best: Option<(f64, Pos)> = None;
    let mut mv = vec![false; g.n];
    for &r in &g.systems[s] {
        for &a in &g.rings[r] {
            mv[a] = g.pos[a].is_none();
        }
    }
    for mirror in [false, true] {
        let mut trial = g.pos.clone();
        fuse_all(g, s, &mut trial, mirror);
        let pen = penalty(g, &trial, &mv);
        if best.as_ref().is_none_or(|b| pen < b.0 - 1e-9) {
            best = Some((pen, trial));
        }
        if pen < 1e-9 {
            break;
        }
    }
    if let Some((_, p)) = best {
        g.pos = p;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_largest_gap_ignores_a_nan_angle() {
        let clean = largest_gap(&[0.0, 1.0, 2.5]);
        assert_eq!(largest_gap(&[f64::NAN, 0.0, 2.5, f64::NAN, 1.0]), clean);
        assert_eq!(largest_gap(&[f64::NAN]), (0.0, TAU));
        assert_eq!(largest_gap(&[2.5, f64::NAN, 1.0, 0.0]), clean);
    }

    #[test]
    fn an_arc_closing_a_hexagon_gives_its_four_missing_corners() {
        let pts = arc([3.0, 0.0], [3.0, 1.0], 4, true);
        let mut prev = [3.0, 0.0];
        for p in pts.iter().chain(std::iter::once(&[3.0, 1.0])) {
            assert!((dist(prev, *p) - 1.0).abs() < 1e-9, "{pts:?}");
            prev = *p;
        }
    }
}
