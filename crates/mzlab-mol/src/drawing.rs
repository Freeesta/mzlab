//! A molfile read as drawn, for the structure check of the Draw page: query atoms and bonds are kept instead of refused, a failed
//! kekulisation leaves the aromatic bonds single, and the wedge marks come back with the bond they sit on. Nothing here judges the drawing.

use crate::codec::MolError;
use crate::model::Mol;
use crate::molfile::{read_draft, Wedge};
use crate::sites::Site;

/// A wedge, hash or wavy mark: narrow end `from`, wide end `to` (0-based atoms).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WedgeMark {
    pub from: u32,
    pub to: u32,
    /// Written on a double, triple or aromatic bond.
    pub multiple: bool,
    /// A wavy ("either") mark on a single bond.
    pub wavy: bool,
}

/// What a drawing holds: the record (not validated: a query atom keeps its symbol, with no hydrogens) and the marks that the record drops.
#[derive(Clone, Debug)]
pub struct Drawing {
    pub mol: Mol,
    /// False when the bonds written aromatic could not be given alternating orders (they are left single).
    pub kekule: bool,
    /// Atoms on a bond written aromatic.
    pub arom_atoms: Vec<u32>,
    pub query_atoms: Vec<u32>,
    /// Query bonds as (atom, atom, written type).
    pub query_bonds: Vec<(u32, u32, i32)>,
    pub wedges: Vec<WedgeMark>,
    /// The stereo sites of the record, query atoms read as an element without valence rules.
    pub sites: Vec<Site>,
}

impl Mol {
    /// Reads the first record of a V2000 or V3000 molfile as drawn; fails only on a file that cannot be read at all.
    pub fn drawing_from_molfile(text: &str) -> Result<Drawing, MolError> {
        let draft = read_draft(text)?;
        let query_atoms: Vec<u32> = (0..draft.atoms.len() as u32)
            .filter(|&i| draft.atoms[i as usize].query)
            .collect();
        let mut arom_atoms = Vec::new();
        let mut query_bonds = Vec::new();
        let mut wedges = Vec::new();
        for b in &draft.bonds {
            let (a, z) = (b.a as u32, b.b as u32);
            if b.arom {
                arom_atoms.extend([a, z]);
            }
            if b.query != 0 {
                query_bonds.push((a.min(z), a.max(z), b.query));
            }
            if b.multi_wedge || (b.order == 1 && b.wedge != Wedge::None) {
                wedges.push(WedgeMark {
                    from: a,
                    to: z,
                    multiple: b.multi_wedge,
                    wavy: b.wedge == Wedge::Either && !b.multi_wedge,
                });
            }
        }
        arom_atoms.sort_unstable();
        arom_atoms.dedup();
        let (mol, kekule) = draft.finish(true)?;
        let mut stand_in = mol.clone();
        for &i in &query_atoms {
            stand_in.atoms[i as usize].el = "Xe".into();
        }
        stand_in.validate()?;
        Ok(Drawing {
            sites: stand_in.stereo_sites(),
            mol,
            kekule,
            arom_atoms,
            query_atoms,
            query_bonds,
            wedges,
        })
    }
}

/// Allowed total valences (bond orders, hydrogens and unpaired electrons) of `el` with charge `chg`, ascending; empty for an element without
/// rules (hydrogen and the main-group elements of the SMILES organic subset and their heavier relatives have rules).
pub fn allowed_valences(el: &str, chg: i32) -> Vec<u32> {
    if el == "H" {
        return match chg {
            0 => vec![1],
            1 | -1 => vec![0],
            _ => Vec::new(),
        };
    }
    crate::valence::allowed(el, chg)
}

/// Whether `el` has valence rules at all.
pub fn has_valence_rules(el: &str) -> bool {
    el == "H" || !crate::valence::allowed(el, 0).is_empty()
}
