//! CIP marks of the picture: R/S/r/s beside the defined centres and E/Z beside the defined double bonds, each in the free angle that touches
//! no label, bond or atom (the first angle when none is free).

use super::cip::{cip_labels, CipLabels};
use super::draw_build::Geo;
use super::draw_font::{ADVANCE, CAP};
use super::draw_label::{exit, gaps, norm, overlap, seg_hits};
use super::draw_svg::{Label, Prim, INK};
use mzlab_mol::trig::{dot, len, sub, V};
use mzlab_mol::{Mol, Site, Stereo};

/// Where a mark goes: the point it hangs from, the directions to try, the start distance and the atoms it must not cover.
struct Spot {
    from: V,
    dirs: Vec<V>,
    t0: f64,
    own: (usize, Option<usize>),
}

fn defined_sites(m: &Mol) -> Vec<Site> {
    let mut sites = Vec::new();
    for st in &m.stereo {
        match st {
            Stereo::Tetra {
                c,
                status: mzlab_mol::Status::Defined,
                ..
            } => sites.push(Site::Tetra(*c)),
            Stereo::Db {
                a,
                b,
                status: mzlab_mol::Status::Defined,
                ..
            } => sites.push(Site::Db(*a, *b)),
            _ => {}
        }
    }
    sites
}

/// Whether the box `bx` is free of labels, bonds and other atoms.
fn is_free(
    g: &Geo,
    segs: &[(V, V)],
    labels: &[Label],
    bx: &[f64; 4],
    own: (usize, Option<usize>),
) -> bool {
    !labels.iter().any(|l| overlap(&l.bx, bx, 0.0))
        && !segs.iter().any(|&(p, q)| seg_hits(p, q, bx))
        && !g.pos.iter().enumerate().any(|(i, p)| {
            i != own.0
                && Some(i) != own.1
                && p[0] > bx[0]
                && p[0] < bx[2]
                && p[1] > bx[1]
                && p[1] < bx[3]
        })
}

/// Writes `text` at the first free direction of `spot` and records its label.
fn place(
    g: &Geo,
    segs: &[(V, V)],
    out: (&mut Vec<Label>, &mut Vec<Prim>),
    who: (u32, Option<u32>),
    text: String,
    spot: Spot,
) {
    let (labels, prims) = out;
    let size = 0.8 * g.f;
    let (w, h) = (ADVANCE * size * text.chars().count() as f64, CAP * size);
    let mut first = None;
    for d in spot.dirs {
        let r = spot.t0 + 0.5 * (w * d[0].abs() + h * d[1].abs()) + 0.06 * g.bl;
        let c = [spot.from[0] + d[0] * r, spot.from[1] + d[1] * r];
        let bx = [
            c[0] - w / 2.0,
            c[1] - h / 2.0,
            c[0] + w / 2.0,
            c[1] + h / 2.0,
        ];
        first.get_or_insert((c, bx));
        if is_free(g, segs, labels, &bx, spot.own) {
            first = Some((c, bx));
            break;
        }
    }
    if let Some((c, bx)) = first {
        prims.push(Prim::Run([bx[0], c[1] + h / 2.0], size, text.clone(), INK));
        labels.push(Label {
            kind: "cip",
            atom: who.0,
            other: who.1,
            text,
            bx,
        });
    }
}

/// The spot of the mark of centre `a`: the gaps between its bonds, outside its own atom label.
fn centre_spot(g: &Geo, labels: &[Label], a: u32) -> Spot {
    let i = a as usize;
    let ds: Vec<V> = g.nb[i]
        .iter()
        .map(|&j| norm(sub(g.pos[j], g.pos[i])))
        .collect();
    let t0 = labels
        .iter()
        .find(|l| l.kind == "atom" && l.atom == a)
        .map_or(0.1 * g.bl, |l| {
            exit(&l.bx, g.pos[i], [1.0, 0.0], 0.0)
                .min(0.5 * len([l.bx[2] - l.bx[0], l.bx[3] - l.bx[1]]))
        });
    Spot {
        from: g.pos[i],
        dirs: gaps(&ds),
        t0,
        own: (i, None),
    }
}

/// The spot of the mark of double bond `a`-`b`: beside its middle, first on the side of fewer substituents.
fn bond_spot(g: &Geo, a: u32, b: u32) -> Spot {
    let (i, j) = (a as usize, b as usize);
    let pos = &g.pos;
    let mid = [(pos[i][0] + pos[j][0]) / 2.0, (pos[i][1] + pos[j][1]) / 2.0];
    let d = norm(sub(pos[j], pos[i]));
    let nrm = [-d[1], d[0]];
    let side: f64 = [i, j]
        .iter()
        .flat_map(|&k| {
            g.nb[k]
                .iter()
                .filter(|&&x| x != i && x != j)
                .map(move |&x| (k, x))
        })
        .map(|(k, x)| {
            if dot(nrm, sub(pos[x], pos[k])) > 0.0 {
                1.0
            } else {
                -1.0
            }
        })
        .sum();
    let s = if side > 0.0 { -1.0 } else { 1.0 };
    Spot {
        from: mid,
        dirs: vec![[nrm[0] * s, nrm[1] * s], [-nrm[0] * s, -nrm[1] * s]],
        t0: 0.22 * g.bl,
        own: (i, Some(j)),
    }
}

/// R/S/r/s beside the defined centres and E/Z beside the defined double bonds, in the free angle that touches no label or bond.
pub(super) fn cip_marks(g: &Geo, labels: &mut Vec<Label>, prims: &mut Vec<Prim>) {
    let CipLabels { atoms, bonds, .. } = cip_labels(g.m, &defined_sites(g.m));
    let segs: Vec<(V, V)> = (0..g.m.atoms.len())
        .flat_map(|i| {
            g.nb[i]
                .iter()
                .filter(move |&&j| j > i)
                .map(move |&j| (i, j))
        })
        .map(|(i, j)| (g.pos[i], g.pos[j]))
        .collect();
    for (a, v) in atoms {
        let spot = centre_spot(g, labels, a);
        place(
            g,
            &segs,
            (&mut *labels, &mut *prims),
            (a, None),
            format!("({v})"),
            spot,
        );
    }
    for (a, b, v) in bonds {
        let spot = bond_spot(g, a, b);
        place(
            g,
            &segs,
            (&mut *labels, &mut *prims),
            (a, Some(b)),
            format!("({v})"),
            spot,
        );
    }
}
