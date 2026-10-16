//! A molecule with 3D coordinates, numbered like the rest of mzLab: the heavy atoms in the order of the SMILES (the order of
//! `mzlab_feat::chem::grafo::Mol`, i.e. of `posiz/1`), then the hydrogens of each heavy atom in turn.

use mzlab_core::error::{Error, Result};
use mzlab_feat::chem::grafo::Mol;

/// Elements the geometry covers (atomic numbers): H, C, N, O, F, P, S, Cl, Br, I.
const SUPPORTED: [u8; 10] = [1, 6, 7, 8, 9, 15, 16, 17, 35, 53];

pub fn atomic_number(el: &str) -> Option<u8> {
    Some(match el {
        "H" => 1,
        "C" => 6,
        "N" => 7,
        "O" => 8,
        "F" => 9,
        "P" => 15,
        "S" => 16,
        "Cl" => 17,
        "Br" => 35,
        "I" => 53,
        _ => return None,
    })
}

pub fn symbol(z: u8) -> &'static str {
    match z {
        1 => "H",
        6 => "C",
        7 => "N",
        8 => "O",
        9 => "F",
        15 => "P",
        16 => "S",
        17 => "Cl",
        35 => "Br",
        53 => "I",
        _ => "X",
    }
}

#[derive(Debug, Clone)]
pub struct Mol3d {
    /// Atomic numbers.
    pub el: Vec<u8>,
    /// Coordinates in Å (all zero until a geometry is embedded).
    pub xyz: Vec<[f64; 3]>,
    /// (a, b, order): order 1, 2, 3, or 4 for an aromatic bond.
    pub bonds: Vec<(usize, usize, u8)>,
    /// Index in `grafo::Mol` of each atom (`None` for a hydrogen).
    pub heavy_index: Vec<Option<usize>>,
    /// Formal charge of each atom.
    pub charge: Vec<i8>,
    /// Aromatic flag of each atom (as written in the SMILES).
    pub arom: Vec<bool>,
}

impl Mol3d {
    /// The atoms of `mol` plus their implicit hydrogens (counts from `grafo::Mol`), no coordinates yet.
    pub fn from_mol(mol: &Mol) -> Result<Mol3d> {
        let n = mol.atoms.len();
        let mut el = Vec::new();
        let mut charge = Vec::new();
        let mut arom = Vec::new();
        for a in &mol.atoms {
            let z = atomic_number(&a.el)
                .filter(|z| SUPPORTED.contains(z))
                .ok_or_else(|| Error::new("error.chem.element").with("element", &a.el))?;
            el.push(z);
            charge.push(a.charge as i8);
            arom.push(a.arom);
        }
        let mut heavy_index: Vec<Option<usize>> = (0..n).map(Some).collect();
        let mut bonds: Vec<(usize, usize, u8)> = mol
            .bonds
            .iter()
            .map(|b| (b.a, b.b, if b.order == 1.5 { 4 } else { b.order as u8 }))
            .collect();
        for (i, a) in mol.atoms.iter().enumerate() {
            for _ in 0..a.h {
                bonds.push((i, el.len(), 1));
                el.push(1);
                charge.push(0);
                arom.push(false);
                heavy_index.push(None);
            }
        }
        let xyz = vec![[0.0; 3]; el.len()];
        Ok(Mol3d {
            el,
            xyz,
            bonds,
            heavy_index,
            charge,
            arom,
        })
    }

    pub fn len(&self) -> usize {
        self.el.len()
    }

    pub fn is_empty(&self) -> bool {
        self.el.is_empty()
    }

    /// XYZ file text (atom count, comment line, one line per atom).
    pub fn to_xyz(&self, comment: &str) -> String {
        let mut s = format!("{}\n{}\n", self.len(), comment);
        for (z, p) in self.el.iter().zip(&self.xyz) {
            s.push_str(&format!(
                "{:<2} {:>12.6} {:>12.6} {:>12.6}\n",
                symbol(*z),
                p[0],
                p[1],
                p[2]
            ));
        }
        s
    }
}
