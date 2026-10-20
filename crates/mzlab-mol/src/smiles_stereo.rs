//! Stereo of a parsed SMILES: `@`/`@@` into tetrahedral parity and `/`, `\` into cis/trans, both as `defined` elements of `mol/1`.
//! Only what is written is read: whether an unmarked centre could be stereogenic is not decided here.

use crate::model::{Cfg, Parity, Status, Stereo};
use crate::smiles_parse::{Raw, Sym};

/// Tetrahedral centres marked with `@`/`@@`. The neighbours are the written ones with the implicit hydrogen (or lone pair) right after the
/// first, or first when the atom has no predecessor; the parity is turned into "ascending refs, implicit last" by counting inversions.
pub(crate) fn tetra(raw: &Raw, h: &[u8]) -> Vec<Stereo> {
    let mut out = Vec::new();
    for (c, a) in raw.atoms.iter().enumerate() {
        let Some(double) = a.chiral else { continue };
        let mut list: Vec<usize> = raw.slots[c].iter().map(|&b| raw.other(b, c)).collect();
        let explicit = list.len();
        let implicit = match (explicit, h[c]) {
            (3, 0 | 1) => true,
            (4, 0) => false,
            _ => continue,
        };
        if implicit {
            list.insert(usize::from(a.has_parent), usize::MAX);
        }
        let inversions = (0..list.len())
            .flat_map(|i| (i + 1..list.len()).map(move |j| (i, j)))
            .filter(|&(i, j)| list[i] > list[j])
            .count();
        let mut refs: Vec<u32> = list
            .iter()
            .filter(|&&x| x != usize::MAX)
            .map(|&x| x as u32)
            .collect();
        refs.sort_unstable();
        let cw = double ^ (inversions % 2 == 1);
        out.push(Stereo::Tetra {
            c: c as u32,
            refs: Some(refs),
            implicit_h: Some(explicit == 3 && h[c] == 1),
            par: Some(if cw { Parity::Cw } else { Parity::Ccw }),
            status: Status::Defined,
        });
    }
    out
}

/// Whether the neighbour at the other end of bond `b` sits above `x`, when the bond carries `/` or `\`.
fn up_from(raw: &Raw, b: usize, x: usize) -> Option<bool> {
    let e = &raw.bonds[b];
    match e.sym {
        Sym::Up | Sym::Down => Some((e.sym == Sym::Up) == (e.from == x)),
        _ => None,
    }
}

/// The lowest-index neighbour of `x` other than `skip` and whether it is above `x`; a neighbour without a mark takes the opposite of a marked one.
fn side(raw: &Raw, x: usize, skip: usize) -> Option<(usize, bool)> {
    let mut near: Vec<(usize, Option<bool>)> = raw.slots[x]
        .iter()
        .map(|&b| (raw.other(b, x), up_from(raw, b, x)))
        .filter(|&(o, _)| o != skip)
        .collect();
    near.sort_unstable();
    let r = near.first()?.0;
    let (o, up) = near.iter().find_map(|&(o, d)| d.map(|u| (o, u)))?;
    Some((r, up == (o == r)))
}

/// Double bonds written with `=` whose two ends both have a marked neighbour.
pub(crate) fn double(raw: &Raw, orders: &[u8]) -> Vec<Stereo> {
    let mut ends: Vec<(usize, usize)> = raw
        .bonds
        .iter()
        .enumerate()
        .filter(|&(i, e)| e.sym == Sym::Double && orders[i] == 2)
        .map(|(_, e)| (e.from.min(e.to), e.from.max(e.to)))
        .collect();
    ends.sort_unstable();
    ends.into_iter()
        .filter_map(|(a, b)| {
            let (ra, ua) = side(raw, a, b)?;
            let (rb, ub) = side(raw, b, a)?;
            Some(Stereo::Db {
                a: a as u32,
                b: b as u32,
                refs: Some([ra as u32, rb as u32]),
                cfg: Some(if ua == ub { Cfg::Cis } else { Cfg::Trans }),
                status: Status::Defined,
            })
        })
        .collect()
}
