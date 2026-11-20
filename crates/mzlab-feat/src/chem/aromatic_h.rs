//! The hydrogens of the atoms of a ring written aromatic in a molfile (type 4 bonds), for `graph::Mol::from_molfile`: they are the ones
//! `mzlab-mol` gives (the reader of `check/1`), so the label of a selection or of a cut never differs from what the check sees.

use mzlab_core::error::{Error, Result};
use mzlab_mol::DrawnAtom;

use super::smiles::{self, RawAtom, RawBond};

/// Sets the hydrogens of the aromatic atoms of `raw` and answers whether `mzlab-mol` counted them (`drawn_hydrogens`: kekulised, the H
/// of an `MRV_IMPLICIT_H` data group on its atom, radicals and stated valences counted; the caller must not subtract the radicals
/// again). `props` are the lines after the bond block, `rad` the `M  RAD` codes, `valence` the valence column. One rule for a drawing
/// and for a selected part of one: closed aromatic rings that can be kekulised are counted by `mzlab-mol`; an open piece of a ring
/// (what a cut or a selection leaves: an aromatic atom with one aromatic bond) or a ring that cannot be kekulised is read by valence
/// (false), a stated hydrogen on the atom whose valence it fills. Never refused for that: `check/1` gives the `kekule` warning of the
/// same drawing. Only a hydrogen count outside 0 to 4 (a damaged file) is refused, with its reason.
pub(super) fn apply(
    raw: &mut [RawAtom],
    written: &[RawBond],
    props: &[&str],
    rad: &[(usize, i32)],
    valence: &[Option<u8>],
) -> Result<bool> {
    let refuse =
        |e: mzlab_mol::MolError| Error::new("error.grafo.molfile").with("message", e.to_string());
    let stated = mzlab_mol::stated_hydrogens(props, raw.len()).map_err(refuse)?;
    if !raw.iter().any(|r| r.arom) {
        return Ok(false);
    }
    let mut ring_bonds = vec![0u8; raw.len()];
    for &(a, b, order, _) in written {
        if order == 1.5 {
            ring_bonds[a] = ring_bonds[a].saturating_add(1);
            ring_bonds[b] = ring_bonds[b].saturating_add(1);
        }
    }
    let closed = raw.iter().zip(&ring_bonds).all(|(r, n)| !r.arom || *n >= 2);
    let mut atoms: Vec<DrawnAtom> = raw
        .iter()
        .zip(valence)
        .map(|(r, v)| DrawnAtom {
            el: &r.el,
            chg: r.charge as i8,
            rad: 0,
            valence: *v,
        })
        .collect();
    for &(i, code) in rad {
        atoms[i].rad = match code {
            1 | 3 => 2,
            2 => 1,
            _ => 0,
        };
    }
    let bonds: Vec<(usize, usize, u8)> = written
        .iter()
        .map(|&(a, b, order, _)| (a, b, if order == 1.5 { 0 } else { order as u8 }))
        .collect();
    match mzlab_mol::drawn_hydrogens(&atoms, &bonds, stated.clone()) {
        Ok(h) if closed => {
            for (r, h) in raw.iter_mut().zip(h).filter(|(r, _)| r.arom) {
                r.h = Some(u32::from(h));
            }
            Ok(true)
        }
        _ => {
            by_valence(raw, written, stated);
            Ok(false)
        }
    }
}

/// A stated hydrogen on an aromatic atom whose bonds and stated hydrogens fill its valence (pyrrole-type N): the atom gives no
/// electron to the ring and its hydrogens are those the valence leaves. Any other statement changes nothing.
fn by_valence(raw: &mut [RawAtom], written: &[RawBond], stated: Vec<(usize, u32)>) {
    let mut used = vec![0f32; raw.len()];
    for &(a, b, order, _) in written {
        let v = if order == 1.5 { 1.0 } else { order };
        used[a] += v;
        used[b] += v;
    }
    for (i, h) in stated {
        let Some(r) = raw.get_mut(i) else { continue };
        let valence = smiles::valence_for(&r.el, r.charge, true, used[i]);
        if let Some(v) = valence.filter(|v| r.arom && h > 0 && used[i] + h as f32 >= *v) {
            r.h = Some((v - used[i]).round().max(0.0) as u32);
        }
    }
}
