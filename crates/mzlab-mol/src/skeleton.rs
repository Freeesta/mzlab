//! Looser identities of a record, for telling a drawing from its near relatives in a catalogue: the skeleton (no stereo, no isotopes,
//! plain explicit H folded into its neighbour) and its two halves (isotopes dropped with stereo kept, stereo dropped with isotopes kept),
//! which say what a near match differs in. Each is the core hash of a canonical record, like `can()`, under its own prefix.

use crate::model::{Mol, Stereo};

/// Prefix of `Mol::skeleton`; it changes with `CAN_VERSION`, since the skeleton is the canonical hash of a reduced record.
pub const SKELETON_VERSION: &str = "skel1";
/// Prefix of `Mol::lookup_key`, the `can()` that does not see which Kekulé form was written.
pub const LOOKUP_VERSION: &str = "mol1a";

impl Mol {
    /// The same record with every isotope at natural abundance; plain H atoms that no stereo element names join their neighbour's `h`.
    pub fn without_isotopes(&self) -> Mol {
        let mut m = self.core_only();
        m.atoms.iter_mut().for_each(|a| a.iso = 0);
        m.with_implicit_hydrogens()
    }

    /// The same record with no stereo elements and no groups (isotopes kept); plain H atoms join their neighbour's `h`.
    pub fn without_stereo(&self) -> Mol {
        let mut m = self.core_only();
        m.stereo.clear();
        m.groups.clear();
        m.with_implicit_hydrogens()
    }

    /// The identity for lookups: like `can()`, but plain explicit H folded and every perceived aromatic bond hashed as one kind (order 4),
    /// so `OC(=O)c1ccccc1OC(C)=O` and `CC(=O)OC1=CC=CC=C1C(=O)O` agree although their Kekulé forms differ. A key, never a record to store.
    pub fn lookup_key(&self) -> String {
        format!("{LOOKUP_VERSION}:{}", self.aromatic_hash())
    }

    /// The constitution alone: `without_stereo` and `without_isotopes` together, hashed as `lookup_key` does, under `skel1:`.
    pub fn skeleton(&self) -> String {
        format!(
            "{SKELETON_VERSION}:{}",
            self.without_stereo().without_isotopes().aromatic_hash()
        )
    }

    fn aromatic_hash(&self) -> String {
        let mut m = self.with_implicit_hydrogens();
        let arom = m.aromatic_bonds();
        m.bonds
            .iter_mut()
            .zip(arom)
            .filter(|(_, a)| *a)
            .for_each(|(b, _)| b.o = 4);
        m.canonical().core_hash_hex()
    }

    fn core_only(&self) -> Mol {
        Mol::new(
            self.atoms.clone(),
            self.bonds.clone(),
            self.stereo.clone(),
            self.groups.clone(),
        )
    }

    /// Drops every H atom of natural abundance with no charge, radical or H of its own and one single bond to a heavier atom, unless a stereo
    /// element names it; its neighbour gains one `h`. The kept atoms keep their relative order, so bonds stay `a < b` and refs ascending.
    pub fn with_implicit_hydrogens(&self) -> Mol {
        let into = self.foldable_hydrogens();
        if into.iter().all(|x| x.is_none()) {
            return self.clone();
        }
        let n = self.atoms.len();
        let mut atoms = self.atoms.clone();
        into.iter().flatten().for_each(|&o| atoms[o].h += 1);
        let mut new_of = vec![u32::MAX; n];
        for (k, i) in (0..n).filter(|&i| into[i].is_none()).enumerate() {
            new_of[i] = k as u32;
        }
        let at = |i: u32| new_of[i as usize];
        let kept = |i: u32| into[i as usize].is_none();
        let atoms = atoms
            .into_iter()
            .enumerate()
            .filter(|(i, _)| into[*i].is_none())
            .map(|(_, a)| a)
            .collect();
        let bonds = self
            .bonds
            .iter()
            .filter(|b| kept(b.a) && kept(b.b))
            .map(|b| crate::model::Bond {
                a: at(b.a),
                b: at(b.b),
                o: b.o,
            })
            .collect();
        let stereo = self.stereo.iter().map(|s| remap_stereo(s, &at)).collect();
        let mut groups = self.groups.clone();
        groups
            .iter_mut()
            .for_each(|g| g.atoms.iter_mut().for_each(|a| *a = at(*a)));
        Mol::new(atoms, bonds, stereo, groups)
    }

    /// For every atom: the heavy neighbour it folds into when it is a plain H (no isotope, charge, radical or H count, one single bond to a
    /// non-H, not named by a stereo element), else `None`.
    fn foldable_hydrogens(&self) -> Vec<Option<usize>> {
        let n = self.atoms.len();
        let mut named = vec![false; n];
        for s in &self.stereo {
            let ids: Vec<u32> = match s {
                Stereo::Tetra { c, refs, .. } => std::iter::once(*c)
                    .chain(refs.iter().flatten().copied())
                    .collect(),
                Stereo::Db { a, b, refs, .. } => [*a, *b]
                    .into_iter()
                    .chain(refs.iter().flatten().copied())
                    .collect(),
            };
            ids.into_iter().for_each(|i| named[i as usize] = true);
        }
        let mut deg = vec![0usize; n];
        for b in &self.bonds {
            deg[b.a as usize] += 1;
            deg[b.b as usize] += 1;
        }
        let plain = |i: usize| {
            let a = &self.atoms[i];
            a.el == "H"
                && a.iso == 0
                && a.chg == 0
                && a.rad == 0
                && a.h == 0
                && deg[i] == 1
                && !named[i]
        };
        let mut into = vec![None; n];
        for b in self.bonds.iter().filter(|b| b.o == 1) {
            let (x, y) = (b.a as usize, b.b as usize);
            for (h, other) in [(x, y), (y, x)] {
                if plain(h) && self.atoms[other].el != "H" {
                    into[h] = Some(other);
                }
            }
        }
        into
    }
}

/// A stereo element with its atom ids renumbered.
fn remap_stereo(s: &Stereo, at: &impl Fn(u32) -> u32) -> Stereo {
    match s.clone() {
        Stereo::Tetra {
            c,
            refs,
            implicit_h,
            par,
            status,
        } => Stereo::Tetra {
            c: at(c),
            refs: refs.map(|r| r.into_iter().map(at).collect()),
            implicit_h,
            par,
            status,
        },
        Stereo::Db {
            a,
            b,
            refs,
            cfg,
            status,
        } => Stereo::Db {
            a: at(a),
            b: at(b),
            refs: refs.map(|r| r.map(at)),
            cfg,
            status,
        },
    }
}
