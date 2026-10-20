//! The graph of mzFinder as a view of the structure record `mol/1` (`mzlab-mol`): the same `Mol` the SMILES and molfile readers build, made from the
//! record's atoms, hydrogens and Kekulé bonds. Aromaticity is perceived from the record (`Mol::aromatic_bonds` of `mzlab-mol`) when asked for.

use mzlab_core::elements;
use mzlab_core::error::{Error, Result};

use super::grafo::{Atom, Bond, Mol, MAX_ATOMS};
use super::smiles::normal_valence;

impl Mol {
    /// The graph of a record, atoms in the record's order and bonds sorted by atom pair. With `aromatic` the bonds and atoms of the aromatic rings
    /// perceived from the Kekulé form get the flag and order 1.5 that a lower-case SMILES gives; without it every bond keeps its Kekulé order, as an
    /// upper-case SMILES does. Isotopes and radical marks are not part of this graph (its masses are the natural ones). Disconnected pieces are accepted.
    pub fn from_record(rec: &mzlab_mol::Mol, aromatic: bool) -> Result<Mol> {
        if rec.atoms.len() > MAX_ATOMS {
            return Err(Error::new("error.grafo.tooBig").with("max", MAX_ATOMS));
        }
        for a in &rec.atoms {
            if elements::mass_of(&a.el).is_none() || normal_valence(&a.el).is_none() {
                return Err(Error::new("error.grafo.element").with("element", &a.el));
            }
        }
        let (flag_atoms, flag_bonds) = if aromatic {
            (rec.aromatic_atoms(), rec.aromatic_bonds())
        } else {
            (vec![false; rec.atoms.len()], vec![false; rec.bonds.len()])
        };
        let atoms: Vec<Atom> = rec
            .atoms
            .iter()
            .zip(&flag_atoms)
            .map(|(a, &arom)| Atom {
                el: a.el.clone(),
                arom,
                h: u32::from(a.h),
                charge: i32::from(a.chg),
            })
            .collect();
        let bonds: Vec<Bond> = rec
            .bonds
            .iter()
            .zip(&flag_bonds)
            .map(|(b, &arom)| Bond {
                a: b.a as usize,
                b: b.b as usize,
                order: if arom { 1.5 } else { f32::from(b.o) },
            })
            .collect();
        let mut adj = vec![Vec::new(); atoms.len()];
        for (k, b) in bonds.iter().enumerate() {
            adj[b.a].push((b.b, k));
            adj[b.b].push((b.a, k));
        }
        Ok(Mol { atoms, bonds, adj })
    }
}
