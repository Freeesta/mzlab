//! The fragments a candidate position predicts: every cleavage of `Mol::cleavages()` (rings stay closed), the two sides with their formula, the side that
//! holds the site carrying the change.

use mzlab_core::elements::{self, Formula};

use super::confronto::Pol;
use super::siti::{delta_of, Site};
use crate::chem::frammenti::apply_delta;
use crate::chem::grafo::{add_h, hydrolysable, Mol};

/// One predicted fragment: the bond cut, the atoms of the side, its formula (with the change when `modified`) and the m/z of its ion.
#[derive(Debug, Clone, PartialEq)]
pub struct Frag {
    pub bond: usize,
    pub atoms: Vec<usize>,
    pub formula: String,
    pub modified: bool,
    pub mz: f64,
}

/// Fragments of `site`, as [F+H]+ (positive) or [F-H]- (negative). The convention of `Mol::cleavages()` holds: each side takes an H, and at an ether, ester,
/// amide or sulfonamide bond a side may take OH instead. A demethylation drops the bonds of the lost methyl (they do not exist in the product).
pub fn expected_fragments(mol: &Mol, site: &Site, pol: Pol) -> Vec<Frag> {
    let on_ring = mol.ring_bonds();
    let delta = delta_of(site.kind);
    let adduct = pol.adduct();
    let mut out = Vec::new();
    for (k, bd) in mol.bonds.iter().enumerate() {
        if on_ring[k] || bd.order != 1.0 {
            continue;
        }
        if site.kind == "demethyl" && (bd.a == site.atom || bd.b == site.atom) {
            continue;
        }
        let variants: &[(u64, u64)] = if hydrolysable(mol, k) == "cut" {
            &[(1, 0)]
        } else {
            &[(1, 0), (1, 1)]
        };
        let (sa, sb) = mol.sides(k);
        for mask in [sa, sb] {
            for (h, o) in variants {
                let base: Formula = add_h(mol.formula_of(&mask), *h, *o);
                let modified = mask[site.atom];
                let Ok(fm) = (if modified {
                    apply_delta(&base, &delta)
                } else {
                    Ok(base)
                }) else {
                    continue;
                };
                if fm.is_empty() {
                    continue;
                }
                out.push(Frag {
                    bond: k,
                    atoms: (0..mask.len()).filter(|i| mask[*i]).collect(),
                    formula: elements::fmt(&fm),
                    modified,
                    mz: elements::ion_mz(elements::mass(&fm), adduct).unwrap_or(0.0),
                });
            }
        }
    }
    out
}
