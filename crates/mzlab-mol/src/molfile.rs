//! Molfile (V2000 and V3000, first record of an SDF) into `mol/1`. Atoms keep the order of the file; aromatic bonds (type 4) are kekulised;
//! hydrogens of B C N O P S F Cl Br I Si As Se come from the valence rules (an `MRV_IMPLICIT_H` data group says which aromatic atom has one), other elements get none; stereo is read from wedges (tetrahedral
//! centres), the "either" flags and the 2D/3D geometry of double bonds, and enhanced-stereo groups (V3000). Coordinates are kept as a layer.
//! A double bond with the "either" flag is `unknown`; without the flag its Z/E comes from the drawn coordinates (as RDKit does).
//! Not read: whether a drawn centre or double bond is really stereogenic (a later package decides), nor stereo from 3D coordinates.

use crate::codec::MolError;
use crate::kekule::{assign, Facts};
use crate::model::{Atom, Bond, Coords, GroupKind, Mol};
use crate::molfile_stereo::{double_bonds, groups, tetra};
use crate::molfile_v2000::read_v2000;
use crate::molfile_v3000::read_v3000;
use crate::valence::shortfall;

/// Elements that get implicit hydrogens from the valence rules.
const ORGANIC: &[&str] = &[
    "B", "C", "N", "O", "P", "S", "F", "Cl", "Br", "I", "Si", "As", "Se",
];

impl Mol {
    /// Reads the first record of a V2000 or V3000 molfile (an SDF is cut at `$$$$`).
    pub fn from_molfile(text: &str) -> Result<Mol, MolError> {
        let draft = read_draft(text)?;
        draft.refuse_queries()?;
        let (m, _) = draft.finish(false)?;
        m.validate()?;
        Ok(m)
    }
}

/// An atom as a reader of drawings holds it before its hydrogens are counted: symbol, charge, unpaired electrons.
pub struct DrawnAtom<'a> {
    pub el: &'a str,
    pub chg: i8,
    pub rad: u8,
    /// The total valence the file states for the atom (`Some(0)`: no bonds and no hydrogens).
    pub valence: Option<u8>,
}

/// The hydrogens of the atoms of a drawing, counted by the code of `Mol::from_molfile` (the same kekulisation, stated hydrogens,
/// radicals and refusals) from atoms and bonds already read: `bonds` are (atom, atom, order 1-3 or 0 for a bond written aromatic),
/// `stated` what `stated_hydrogens` gives. For a reader that keeps its own graph and must agree with this one on the aromatic atoms.
pub fn drawn_hydrogens(
    atoms: &[DrawnAtom],
    bonds: &[(usize, usize, u8)],
    stated: Vec<(usize, u32)>,
) -> Result<Vec<u8>, MolError> {
    let n = atoms.len();
    for &(a, b, _) in bonds {
        if a == b {
            return Err(fail(format!("a bond joins atom {} to itself", a + 1)));
        }
        if a >= n || b >= n {
            return Err(fail(format!(
                "a bond names atoms {} and {} outside 1..{n}",
                a + 1,
                b + 1
            )));
        }
    }
    let mut draft = Draft {
        atoms: atoms
            .iter()
            .map(|a| DraftAtom {
                el: a.el.to_string(),
                iso: 0,
                chg: a.chg,
                rad: a.rad,
                xyz: [0.0; 3],
                valence: a.valence,
                query: false,
            })
            .collect(),
        bonds: bonds
            .iter()
            .map(|&(a, b, order)| DraftBond {
                a,
                b,
                order,
                wedge: Wedge::None,
                arom: order == 0,
                query: 0,
                multi_wedge: false,
            })
            .collect(),
        collections: Vec::new(),
        stated_h: stated.into_iter().filter(|s| s.0 < n).collect(),
    };
    draft.kekulise(false)?;
    Ok(draft.final_atoms().into_iter().map(|a| a.h).collect())
}

/// The draft of the first record of a molfile, query atoms and bonds kept (see `Draft::refuse_queries`).
pub(crate) fn read_draft(text: &str) -> Result<Draft, MolError> {
    let text = text.replace("\r\n", "\n");
    let record = text.split("\n$$$$").next().unwrap_or("");
    let lines: Vec<&str> = record.lines().collect();
    let counts = lines
        .get(3)
        .ok_or_else(|| fail("the file has no counts line"))?;
    if counts.contains("V3000") {
        read_v3000(&lines)
    } else {
        read_v2000(&lines)
    }
}

impl Draft {
    /// The error of a strict read: a symbol that is not an element, a bond type outside 1-4.
    fn refuse_queries(&self) -> Result<(), MolError> {
        if let Some(a) = self.atoms.iter().find(|a| a.query) {
            return Err(fail(format!("atom symbol {:?} is not an element", a.el)));
        }
        if let Some(b) = self.bonds.iter().find(|b| b.query != 0) {
            return Err(fail(format!(
                "bond type {} (a query or unsupported bond)",
                b.query
            )));
        }
        Ok(())
    }

    /// The record and the aromatic atoms given a hydrogen the file does not state (None: the aromatic bonds could not be kekulised); with
    /// `lenient` a failed kekulisation leaves them single instead of an error.
    pub(crate) fn finish(mut self, lenient: bool) -> Result<(Mol, Option<Vec<usize>>), MolError> {
        let kekule = self.kekulise(lenient)?;
        let atoms = self.final_atoms();
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
        Ok((m, kekule))
    }

    /// Gives the aromatic bonds their Kekulé orders and answers the aromatic atoms given a hydrogen the file does not state (None, with
    /// `lenient`: they could not be kekulised and are left single).
    fn kekulise(&mut self, lenient: bool) -> Result<Option<Vec<usize>>, MolError> {
        let ends: Vec<(usize, usize)> = self.bonds.iter().map(|b| (b.a, b.b)).collect();
        let mut orders: Vec<u8> = self.bonds.iter().map(|b| b.order).collect();
        let mut facts: Vec<Facts> = self
            .atoms
            .iter()
            .map(|a| Facts {
                el: &a.el,
                chg: i32::from(a.chg),
                h: 0,
            })
            .collect();
        // an atom named by two groups keeps the larger count
        for &(i, h) in &self.stated_h {
            facts[i].h = facts[i].h.max(h);
        }
        let kekule = assign(&ends, &mut orders, &facts);
        if kekule.is_none() && !lenient {
            return Err(fail("the aromatic system cannot be kekulised"));
        }
        if kekule.is_none() {
            orders = self.bonds.iter().map(|b| b.order.max(1)).collect();
        }
        for (b, &o) in self.bonds.iter_mut().zip(&orders) {
            b.order = o;
        }
        Ok(kekule)
    }

    /// The atoms with the hydrogens their (kekulised) bonds leave.
    fn final_atoms(&self) -> Vec<Atom> {
        let mut sum = vec![0u32; self.atoms.len()];
        for b in &self.bonds {
            sum[b.a] += u32::from(b.order);
            sum[b.b] += u32::from(b.order);
        }
        self.atoms
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
            .collect()
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
    /// The symbol is not an element (a query atom, an R-group, a list): `el` keeps it as written.
    pub query: bool,
}

pub(crate) struct DraftBond {
    pub a: usize,
    pub b: usize,
    /// 1-3, or 0 for an aromatic bond to be kekulised.
    pub order: u8,
    pub wedge: Wedge,
    /// Written as an aromatic bond (its order is replaced by the Kekulé one).
    pub arom: bool,
    /// The bond type as written when it is a query or unsupported one (read as single), else 0.
    pub query: i32,
    /// A wedge or hash written on a double, triple or aromatic bond (it says nothing; kept for the structure check).
    pub multi_wedge: bool,
}

/// What the two readers hand over: the atoms and bonds as written, enhanced-stereo collections as 0-based atom lists.
pub(crate) struct Draft {
    pub atoms: Vec<DraftAtom>,
    pub bonds: Vec<DraftBond>,
    pub collections: Vec<(GroupKind, Vec<usize>)>,
    /// Hydrogens an `MRV_IMPLICIT_H` data group states for an atom (how Ketcher, Marvin and RDKit say which aromatic atom carries the H).
    pub stated_h: Vec<(usize, u32)>,
}

/// More hydrogens than any atom carries: a larger count in an `MRV_IMPLICIT_H` group is a damaged file.
pub(crate) const MAX_STATED_H: u32 = 4;

/// The hydrogen count of the data `IMPL_Hn` of an `MRV_IMPLICIT_H` group; None for any other field, an error for a count that is not
/// 0..=`MAX_STATED_H`.
pub(crate) fn implicit_h(field: &str, data: &str) -> Result<Option<u32>, MolError> {
    let count = (field == "MRV_IMPLICIT_H")
        .then(|| data.trim_matches('"').strip_prefix("IMPL_H"))
        .flatten();
    let Some(count) = count else { return Ok(None) };
    match count.parse::<u32>() {
        Ok(h) if h <= MAX_STATED_H => Ok(Some(h)),
        _ => Err(fail(format!(
            "the hydrogen count {data:?} of a data group is not 0 to {MAX_STATED_H}"
        ))),
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
