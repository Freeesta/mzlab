//! The figure of `mol draw`: positions in pixels, atom labels placed greedily, bonds clipped to the labels (plain, double, triple, wedged,
//! hashed, wavy), halos of the highlights, CIP marks, and the frame around everything. Arithmetic is `+ - * /` and `sqrt` only.

use super::draw::Drawn;
use super::draw_cip::cip_marks;
use super::draw_label::{exit, label_of, norm, overlap};
use super::draw_svg::{Figure, Label, Options, Prim, INK};
use mzlab_mol::trig::{dist, dot, sub, V};
use mzlab_mol::Mol;

/// Most pixels on a side; a larger drawing is scaled down and the scene says so.
const MAX_SIDE: f64 = 4096.0;

/// The structure in pixels: positions, neighbours, bond length `bl` and label size `f`.
pub(super) struct Geo<'a> {
    pub m: &'a Mol,
    pub pos: Vec<V>,
    pub nb: Vec<Vec<usize>>,
    pub bl: f64,
    pub f: f64,
}

/// A box `[left, top, right, bottom]`.
type Bx = [f64; 4];

/// The picture of a drawn structure; the flag says the scale was reduced to fit `MAX_SIDE`.
pub fn figure(d: &Drawn, o: &Options) -> (Figure, bool) {
    let (fig, side) = build(d, o, o.scale);
    if side <= MAX_SIDE {
        return (fig, false);
    }
    (build(d, o, o.scale * (MAX_SIDE - 8.0) / side).0, true)
}

fn geo(d: &Drawn, s: f64) -> Geo<'_> {
    let m = &d.mol;
    let n = m.atoms.len();
    let at = |i: usize| -> V {
        let g = |k: usize| {
            m.coords
                .as_ref()
                .and_then(|c| c.xy.get(i))
                .and_then(|r| r.get(k))
                .copied()
                .unwrap_or(0.0)
        };
        [g(0) * s, -g(1) * s]
    };
    let mut nb: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b, ..) in &m.written_bonds() {
        nb[a as usize].push(b as usize);
        nb[b as usize].push(a as usize);
    }
    Geo {
        m,
        pos: (0..n).map(at).collect(),
        nb,
        bl: 1.5 * s,
        f: 0.42 * 1.5 * s,
    }
}

/// Atom labels, placed greedily (the side of the hydrogens switched when it would touch an earlier label): the labels, their text runs and
/// the box of each labelled atom.
fn atom_labels(g: &Geo) -> (Vec<Label>, Vec<Prim>, Vec<Option<Bx>>) {
    let n = g.m.atoms.len();
    let mut labels: Vec<Label> = Vec::new();
    let mut runs: Vec<Prim> = Vec::new();
    let mut boxes: Vec<Option<Bx>> = vec![None; n];
    for (i, slot) in boxes.iter_mut().enumerate() {
        let a = &g.m.atoms[i];
        if a.el == "C" && a.chg == 0 && a.iso == 0 && !g.nb[i].is_empty() {
            continue;
        }
        let sx: f64 = g.nb[i].iter().map(|&j| g.pos[j][0] - g.pos[i][0]).sum();
        let first = sx > 0.1 * g.bl;
        let mut pick = label_of(g.m, i, first, g.pos[i], g.f);
        if a.h > 0 && labels.iter().any(|l| overlap(&l.bx, &pick.1, 0.0)) {
            let alt = label_of(g.m, i, !first, g.pos[i], g.f);
            if !labels.iter().any(|l| overlap(&l.bx, &alt.1, 0.0)) {
                pick = alt;
            }
        }
        let text: String = pick.0.iter().map(|p| p.0.as_str()).collect();
        for (t, p, sz) in pick.0 {
            runs.push(Prim::Run(p, sz, t, INK));
        }
        *slot = Some(pick.1);
        labels.push(Label {
            kind: "atom",
            atom: i as u32,
            other: None,
            text,
            bx: pick.1,
        });
    }
    (labels, runs, boxes)
}

/// The halos under the structure: a thick line per highlighted bond, a disc per highlighted atom.
fn halos(g: &Geo, o: &Options) -> Vec<Prim> {
    let mut prims = Vec::new();
    for h in &o.hl {
        for &(a, b) in &h.bonds {
            prims.push(Prim::Line(
                g.pos[a as usize],
                g.pos[b as usize],
                0.4 * g.bl,
                h.color,
            ));
        }
        for &a in &h.atoms {
            prims.push(Prim::Disc(g.pos[a as usize], 0.28 * g.bl, h.color));
        }
    }
    prims
}

/// One bond clipped to its labels: atoms `ia`, `ib`, their points `p`, `q`, the clipped ends `ps`, `qs`, the unit direction `d`, its normal.
struct Bond {
    ia: usize,
    ib: usize,
    ps: V,
    qs: V,
    d: V,
    nrm: V,
    total: f64,
}

/// The bond `a`-`b` clipped to the label boxes; none when the labels leave no room for a line.
fn clip(g: &Geo, boxes: &[Option<Bx>], a: u32, b: u32) -> Option<Bond> {
    let (ia, ib) = (a as usize, b as usize);
    let (p, q) = (g.pos[ia], g.pos[ib]);
    let d = norm(sub(q, p));
    let pad = 0.2 * g.f;
    let cut = |i: usize, dir: V, from: V| boxes[i].map_or(0.0, |bx| exit(&bx, from, dir, pad));
    let (ta, tb) = (cut(ia, d, p), cut(ib, [-d[0], -d[1]], q));
    let total = dist(p, q);
    if ta + tb >= total - 0.1 * g.bl {
        return None;
    }
    Some(Bond {
        ia,
        ib,
        ps: [p[0] + d[0] * ta, p[1] + d[1] * ta],
        qs: [q[0] - d[0] * tb, q[1] - d[1] * tb],
        d,
        nrm: [-d[1], d[0]],
        total,
    })
}

/// `v` moved by `k` along the normal.
fn off(v: V, nrm: V, k: f64) -> V {
    [v[0] + nrm[0] * k, v[1] + nrm[1] * k]
}

/// The point at fraction `u` of the clipped bond.
fn along(b: &Bond, u: f64) -> V {
    [
        b.ps[0] + (b.qs[0] - b.ps[0]) * u,
        b.ps[1] + (b.qs[1] - b.ps[1]) * u,
    ]
}

/// The hashed wedge: short strokes growing towards the far end.
fn hashed(prims: &mut Vec<Prim>, b: &Bond, bl: f64, lw: f64) {
    let k = ((dist(b.ps, b.qs) / (0.11 * bl)) as usize).clamp(4, 12);
    for t in 1..=k {
        let u = t as f64 / (k + 1) as f64;
        let c = along(b, u);
        prims.push(Prim::Line(
            off(c, b.nrm, -0.1 * bl * u),
            off(c, b.nrm, 0.1 * bl * u),
            0.8 * lw,
            INK,
        ));
    }
}

/// The wavy bond (stereo "either"): eight segments zigzagging about the axis.
fn wavy(prims: &mut Vec<Prim>, b: &Bond, bl: f64, lw: f64) {
    let k = 8;
    let mut prev = b.ps;
    for t in 1..=k {
        let c = along(b, t as f64 / k as f64);
        let side = if t % 2 == 1 { 0.07 } else { -0.07 };
        let c = if t == k { c } else { off(c, b.nrm, side * bl) };
        prims.push(Prim::Line(prev, c, lw, INK));
        prev = c;
    }
}

/// A double bond: centred when it ends in a terminal atom or both sides are alike, else the second line goes on the side of the substituents.
fn double(g: &Geo, boxes: &[Option<Bx>], prims: &mut Vec<Prim>, b: &Bond, lw: f64) {
    let gap = 0.16 * g.bl;
    let (ia, ib) = (b.ia, b.ib);
    let line = |x: V, y: V| Prim::Line(x, y, lw, INK);
    // +1 on the normal's side, -1 against it, 0 when alike
    let side: i32 = [ia, ib]
        .iter()
        .flat_map(|&i| {
            g.nb[i]
                .iter()
                .filter(|&&j| j != ia && j != ib)
                .map(move |&j| (i, j))
        })
        .map(|(i, j)| {
            if dot(b.nrm, sub(g.pos[j], g.pos[i])) > 0.0 {
                1
            } else {
                -1
            }
        })
        .sum();
    let end = g.nb[ia].len() == 1 || g.nb[ib].len() == 1;
    if end || side == 0 {
        prims.push(line(
            off(b.ps, b.nrm, gap / 2.0),
            off(b.qs, b.nrm, gap / 2.0),
        ));
        prims.push(line(
            off(b.ps, b.nrm, -gap / 2.0),
            off(b.qs, b.nrm, -gap / 2.0),
        ));
        return;
    }
    let k = f64::from(side.signum()) * gap;
    let inset = |i: usize| {
        if boxes[i].is_some() {
            0.0
        } else {
            (0.9 * gap).min(0.25 * b.total)
        }
    };
    let (ia2, ib2) = (inset(ia), inset(ib));
    let u = [b.ps[0] + b.d[0] * ia2, b.ps[1] + b.d[1] * ia2];
    let v = [b.qs[0] - b.d[0] * ib2, b.qs[1] - b.d[1] * ib2];
    prims.push(line(b.ps, b.qs));
    prims.push(line(off(u, b.nrm, k), off(v, b.nrm, k)));
}

/// The lines of every bond.
fn bonds(g: &Geo, boxes: &[Option<Bx>], prims: &mut Vec<Prim>) {
    let lw = (0.05 * g.bl).max(1.0);
    let gap = 0.16 * g.bl;
    for &(a, b, order, wedge) in &g.m.written_bonds() {
        let Some(bd) = clip(g, boxes, a, b) else {
            continue;
        };
        let line = |x: V, y: V| Prim::Line(x, y, lw, INK);
        match (order, wedge) {
            (1, 1) => prims.push(Prim::Poly(
                vec![
                    bd.ps,
                    off(bd.qs, bd.nrm, 0.1 * g.bl),
                    off(bd.qs, bd.nrm, -0.1 * g.bl),
                ],
                INK,
            )),
            (1, -1) => hashed(prims, &bd, g.bl, lw),
            (1, 2) => wavy(prims, &bd, g.bl, lw),
            (2, _) => double(g, boxes, prims, &bd, lw),
            (3, _) => {
                for k in [-gap, 0.0, gap] {
                    prims.push(line(off(bd.ps, bd.nrm, k), off(bd.qs, bd.nrm, k)));
                }
            }
            _ => prims.push(line(bd.ps, bd.qs)),
        }
    }
}

/// The frame: the box of every atom and label, with a margin of half a bond.
/// The figure and its longer side in pixels, as a float (the integer sizes saturate, so a huge drawing would be scaled by a wrong factor).
fn frame(g: &Geo, labels: Vec<Label>, prims: Vec<Prim>, s: f64) -> (Figure, f64) {
    let mut bb = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    let mut grow = |x: f64, y: f64| bb = [bb[0].min(x), bb[1].min(y), bb[2].max(x), bb[3].max(y)];
    for p in &g.pos {
        grow(p[0], p[1]);
    }
    for l in &labels {
        grow(l.bx[0], l.bx[1]);
        grow(l.bx[2], l.bx[3]);
    }
    if g.pos.is_empty() {
        bb = [0.0; 4];
    }
    let margin = 0.5 * g.bl;
    let (bw, bh) = (bb[2] - bb[0], bb[3] - bb[1]);
    let (w, h) = (
        (bw + 2.0 * margin).ceil().max(1.0),
        (bh + 2.0 * margin).ceil().max(1.0),
    );
    let fig = Figure {
        w: w as u32,
        h: h as u32,
        x0: bb[0] - (w - bw) / 2.0,
        y0: bb[1] - (h - bh) / 2.0,
        scale: s,
        prims,
        labels,
    };
    (fig, w.max(h))
}

fn build(d: &Drawn, o: &Options, s: f64) -> (Figure, f64) {
    let g = geo(d, s);
    let (mut labels, runs, boxes) = atom_labels(&g);
    let mut prims = halos(&g, o);
    bonds(&g, &boxes, &mut prims);
    prims.extend(runs);
    if o.cip {
        cip_marks(&g, &mut labels, &mut prims);
    }
    frame(&g, labels, prims, s)
}
