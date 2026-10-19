//! Molfile (V2000 and V3000, first record of an SDF) into `mol/1`. Atoms keep the order of the file; aromatic bonds (type 4) are kekulised;
//! hydrogens of B C N O P S F Cl Br I come from the valence rules, other elements get none; stereo is read from wedges (tetrahedral
//! centres), the "either" flags and the 2D/3D geometry of double bonds, and enhanced-stereo groups (V3000). Coordinates are kept as a layer.
//! Not read: whether a drawn centre or double bond is really stereogenic (a later package decides), nor stereo from 3D coordinates.

use crate::codec::MolError;
use crate::kekule::{assign, Facts};
use crate::model::{Atom, Bond, Coords, GroupKind, Mol};
use crate::molfile_stereo::{double_bonds, groups, tetra};
use crate::molfile_v2000::read_v2000;
use crate::molfile_v3000::read_v3000;
use crate::valence::shortfall;

/// Elements that get implicit hydrogens from the valence rules.
const ORGANIC: &[&str] = &["B", "C", "N", "O", "P", "S", "F", "Cl", "Br", "I"];

impl Mol {
    /// Reads the first record of a V2000 or V3000 molfile (an SDF is cut at `$$$$`).
    pub fn from_molfile(text: &str) -> Result<Mol, MolError> {
        let text = text.replace("\r\n", "\n");
        let record = text.split("\n$$$$").next().unwrap_or("");
        let lines: Vec<&str> = record.lines().collect();
        let counts = lines
            .get(3)
            .ok_or_else(|| fail("the file has no counts line"))?;
        let draft = if counts.contains("V3000") {
            read_v3000(&lines)?
        } else {
            read_v2000(&lines)?
        };
        let m = draft.finish()?;
        m.validate()?;
        Ok(m)
    }
}

/// A wedge or wavy mark on a bond, read from its first atom to its second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wedge {
    None,
    Up,
    Down,
    Either,
}

pub(crate) struct DraftAtom {
    pub el: String,
    pub iso: u16,
    pub chg: i8,
    /// Unpaired electrons (`M  RAD`, V3000 `RAD=`).
    pub rad: u8,
    pub xyz: [f64; 3],
    /// Total valence the file states (`Some(0)` is "no bonds, no hydrogens"), None when it leaves it to the defaults.
    pub valence: Option<u8>,
}

pub(crate) struct DraftBond {
    pub a: usize,
    pub b: usize,
    /// 1-3, or 0 for an aromatic bond to be kekulised.
    pub order: u8,
    pub wedge: Wedge,
    /// Written as an aromatic bond (its order is replaced by the Kekulé one).
    pub arom: bool,
}

/// What the two readers hand over: the atoms and bonds as written, enhanced-stereo collections as 0-based atom lists.
pub(crate) struct Draft {
    pub atoms: Vec<DraftAtom>,
    pub bonds: Vec<DraftBond>,
    pub collections: Vec<(GroupKind, Vec<usize>)>,
}

impl Draft {
    fn finish(mut self) -> Result<Mol, MolError> {
        let n = self.atoms.len();
        let ends: Vec<(usize, usize)> = self.bonds.iter().map(|b| (b.a, b.b)).collect();
        let mut orders: Vec<u8> = self.bonds.iter().map(|b| b.order).collect();
        let facts: Vec<Facts> = self
            .atoms
            .iter()
            .map(|a| Facts {
                el: &a.el,
                chg: i32::from(a.chg),
                h: 0,
            })
            .collect();
        if !assign(&ends, &mut orders, &facts) {
            return Err(fail("the aromatic system cannot be kekulised"));
        }
        for (b, &o) in self.bonds.iter_mut().zip(&orders) {
            b.order = o;
        }
        let mut sum = vec![0u32; n];
        for b in &self.bonds {
            sum[b.a] += u32::from(b.order);
            sum[b.b] += u32::from(b.order);
        }
        let atoms: Vec<Atom> = self
            .atoms
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let (h, rad) = hydrogens(a, sum[i]);
                Atom {
                    el: a.el.clone(),
                    iso: a.iso,
                    chg: a.chg,
                    rad,
                    h: h as u8,
                }
            })
            .collect();
        let mut bonds: Vec<Bond> = self
            .bonds
            .iter()
            .map(|b| Bond {
                a: b.a.min(b.b) as u32,
                b: b.a.max(b.b) as u32,
                o: b.order,
            })
            .collect();
        bonds.sort_by_key(|b| (b.a, b.b));
        let mut stereo = tetra(&self, &atoms);
        stereo.extend(double_bonds(&self));
        let groups = groups(&self, &stereo)?;
        let mut m = Mol::new(atoms, bonds, stereo, groups);
        m.coords = self.coords();
        Ok(m)
    }

    /// The coordinate layer, unless every atom sits at the origin (a file without coordinates).
    fn coords(&self) -> Option<Coords> {
        if self.atoms.iter().all(|a| a.xyz == [0.0; 3]) {
            return None;
        }
        let flat = self.atoms.iter().all(|a| a.xyz[2] == 0.0);
        let xy = self
            .atoms
            .iter()
            .map(|a| a.xyz[..if flat { 2 } else { 3 }].to_vec())
            .collect();
        Some(Coords {
            dim: if flat { 2 } else { 3 },
            xy,
            from: "molfile".into(),
        })
    }
}

/// Hydrogens and unpaired electrons of an atom whose bonds add up to `sum`: a stated valence fixes the total (the rest is hydrogens,
/// or radical electrons when the valence is below the lowest allowed); otherwise hydrogens fill the lowest allowed valence.
fn hydrogens(a: &DraftAtom, sum: u32) -> (u32, u8) {
    if !ORGANIC.contains(&a.el.as_str()) {
        return (0, a.rad);
    }
    let chg = i32::from(a.chg);
    match a.valence {
        Some(v) => {
            let v = u32::from(v);
            let rad = if a.rad > 0 {
                a.rad
            } else {
                shortfall(&a.el, chg, v) as u8
            };
            (v.saturating_sub(sum), rad)
        }
        None => (
            shortfall(&a.el, chg, sum).saturating_sub(u32::from(a.rad)),
            a.rad,
        ),
    }
}

pub(crate) fn fail(msg: impl Into<String>) -> MolError {
    MolError::Molfile(msg.into())
}
