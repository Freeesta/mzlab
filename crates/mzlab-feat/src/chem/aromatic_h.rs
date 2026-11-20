//! The hydrogens of the atoms of a ring written aromatic in a molfile (type 4 bonds), for `graph::Mol::from_molfile`: they are the ones
//! `mzlab-mol` gives (the reader of `check/1`), so the label of a selection or of a cut never differs from what the check sees.

use mzlab_core::error::{Error, Result};
use mzlab_mol::DrawnAtom;

use super::smiles::{self, RawAtom, RawBond};

/// Sets the hydrogens of the aromatic atoms of `raw` and answers, atom by atom, whether `mzlab-mol` counted them (`drawn_hydrogens`:
/// kekulised, the H of an `MRV_IMPLICIT_H` data group on its atom, radicals and stated valences counted; the caller must not subtract
/// the radical of such an atom again). `props` are the lines after the bond block, `rad` the `M  RAD` codes, `valence` the valence
/// column. One rule for a drawing and for a selected part of one, decided for each aromatic system (the atoms joined by aromatic
/// bonds) alone: a closed one that can be kekulised is counted by `mzlab-mol`; an open piece of a ring (what a cut or a selection
/// leaves: an aromatic atom with one aromatic bond) or a ring that cannot be kekulised is read by valence, a stated hydrogen on the
/// atom whose valence it fills. Never refused for that: `check/1` gives the `kekule` warning of the same drawing. Only a hydrogen
/// count outside 0 to 4 (a damaged file) is refused, with its reason.
pub(super) fn apply(
    raw: &mut [RawAtom],
    written: &[RawBond],
    props: &[&str],
    rad: &[(usize, i32)],
    valence: &[Option<u8>],
) -> Result<Vec<bool>> {
    let refuse =
        |e: mzlab_mol::MolError| Error::new("error.grafo.molfile").with("message", e.to_string());
    let stated = mzlab_mol::stated_hydrogens(props, raw.len()).map_err(refuse)?;
    let mut counted = vec![false; raw.len()];
    if !raw.iter().any(|r| r.arom) {
        return Ok(counted);
    }
    let (system, whole) = systems(raw.len(), written);
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
    // the hydrogens `mzlab-mol` counts when only the systems `keep` are aromatic (the bonds of the others single)
    let count = |keep: &dyn Fn(usize) -> bool| {
        let bonds: Vec<(usize, usize, u8)> = written
            .iter()
            .map(|&(a, b, order, _)| {
                (
                    a,
                    b,
                    if order != 1.5 {
                        order as u8
                    } else if keep(system[a]) {
                        0
                    } else {
                        1
                    },
                )
            })
            .collect();
        mzlab_mol::drawn_hydrogens(&atoms, &bonds, stated.clone())
    };
    // every closed system at once (what `mzlab-mol` does with the whole file); when that fails, each one from its own run: a
    // system that fails, or the search of the unstated hydrogens of many systems together, costs the others nothing
    let runs: Vec<(Option<usize>, Vec<u8>)> = match count(&|s| whole[s]) {
        Ok(h) => vec![(None, h)],
        Err(_) => (0..whole.len())
            .filter(|&s| whole[s])
            .filter_map(|s| count(&|k| k == s).ok().map(|h| (Some(s), h)))
            .collect(),
    };
    for (only, h) in runs {
        for (i, r) in raw.iter_mut().enumerate() {
            if r.arom && whole[system[i]] && only.is_none_or(|s| s == system[i]) {
                r.h = Some(u32::from(h[i]));
                counted[i] = true;
            }
        }
    }
    by_valence(raw, written, stated, &counted);
    Ok(counted)
}

/// The aromatic system of every atom (its number; atoms outside one have their own) and whether each system is closed: every atom
/// of it has two aromatic bonds or more.
fn systems(n: usize, written: &[RawBond]) -> (Vec<usize>, Vec<bool>) {
    let mut system: Vec<usize> = (0..n).collect();
    let mut ring_bonds = vec![0u8; n];
    for &(a, b, _, _) in written.iter().filter(|w| w.2 == 1.5) {
        ring_bonds[a] = ring_bonds[a].saturating_add(1);
        ring_bonds[b] = ring_bonds[b].saturating_add(1);
        let (from, to) = (system[a], system[b]);
        system
            .iter_mut()
            .filter(|s| **s == from)
            .for_each(|s| *s = to);
    }
    // only the number a system goes by can be whole
    let mut whole: Vec<bool> = (0..n).map(|i| system[i] == i).collect();
    for (i, bonds) in ring_bonds.iter().enumerate() {
        whole[system[i]] &= *bonds >= 2;
    }
    (system, whole)
}

/// A stated hydrogen on an aromatic atom whose bonds and stated hydrogens fill its valence (pyrrole-type N): the atom gives no
/// electron to the ring and its hydrogens are those the valence leaves. Any other statement changes nothing; `counted` atoms are left.
fn by_valence(
    raw: &mut [RawAtom],
    written: &[RawBond],
    stated: Vec<(usize, u32)>,
    counted: &[bool],
) {
    let mut used = vec![0f32; raw.len()];
    for &(a, b, order, _) in written {
        let v = if order == 1.5 { 1.0 } else { order };
        used[a] += v;
        used[b] += v;
    }
    for (i, h) in stated {
        let Some(r) = raw.get_mut(i) else { continue };
        let valence = smiles::valence_for(&r.el, r.charge, true, used[i]);
        if let Some(v) =
            valence.filter(|v| r.arom && !counted[i] && h > 0 && used[i] + h as f32 >= *v)
        {
            r.h = Some((v - used[i]).round().max(0.0) as u32);
        }
    }
}
