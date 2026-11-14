//! The tables of mzFinder's `iimn`: adducts from the protonated or deprotonated ion, isotope steps and neutral losses, built once.
//!
//! The identity of `legacy` (`legacy::edge`) and the relations of the families (`family::rel`) read them; they depend only on the elements.

use mzlab_core::elements::{mass, mass_of, parse_formula, ELECTRON};

/// `iimn.ISOTOPES`.
pub const ISOTOPES: [(&str, f64); 3] = [
    ("13C", mzlab_core::elements::C13_STEP),
    ("34S/13C2", 1.99580),
    ("13C2", 2.00671),
];
/// `iimn.LOSSES`: `ionfamily.NEUTRAL_LOSSES`, then the `EXTRA_LOSSES` not already there.
const LOSS_NAMES: [&str; 23] = [
    "H2O", "NH3", "CO", "CO2", "CH2O", "CH4O", "C2H2O", "HCOOH", "C2H4", "C3H6", "HCN", "HF",
    "HCl", "HBr", "SO2", "SO3", "H2S", "CH3", "NO2", "C6H6", "C4H8", "C4H9N", "C4H8O",
];

fn m(e: &str) -> f64 {
    mass_of(e).expect("fixed element")
}

fn neutral(f: &str) -> f64 {
    mass(&parse_formula(f).expect("fixed formula"))
}

/// `E.PROTON`.
pub fn proton() -> f64 {
    m("H") - ELECTRON
}

/// The tables of `iimn`, built once.
pub struct Iimn {
    pub proton: f64,
    /// `ADDUCT_FROM_H` (index 0) and `ADDUCT_FROM_DEPROT` (index 1).
    pub adducts: [Vec<(&'static str, f64)>; 2],
    pub losses: Vec<(&'static str, f64)>,
    pub nh3: f64,
}

impl Iimn {
    pub fn new() -> Iimn {
        let p = proton();
        Iimn {
            proton: p,
            adducts: [
                vec![
                    ("[M+Na]+", m("Na") - ELECTRON - p),
                    ("[M+K]+", m("K") - ELECTRON - p),
                    ("[M+NH4]+", m("N") + 4.0 * m("H") - ELECTRON - p),
                    ("[M+H+ACN]+", neutral("C2H3N")),
                ],
                vec![
                    ("[M+FA-H]-", neutral("CH2O2")),
                    ("[M+Cl]-", m("Cl") + ELECTRON + p),
                    ("[M+Na-2H]-", m("Na") - ELECTRON - p),
                ],
            ],
            losses: LOSS_NAMES.iter().map(|&n| (n, neutral(n))).collect(),
            nh3: neutral("NH3"),
        }
    }

    pub fn adducts_of(&self, pol: i32) -> &[(&'static str, f64)] {
        &self.adducts[if pol > 0 { 0 } else { 1 }]
    }
}

impl Default for Iimn {
    fn default() -> Self {
        Iimn::new()
    }
}
