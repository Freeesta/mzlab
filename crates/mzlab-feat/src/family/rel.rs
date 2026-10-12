//! What the difference of m/z between two co-eluting ions can be: the tables of `TP_Mine/py/tpmine/hr/iimn.py` (`relations`, `dimer_of`) read in
//! the direction of `edge_evidence.relations_of`: the lighter ion X, the heavier Y.
//!
//! Y is an adduct of X, Y is the dimer of X, or X is an in-source fragment of Y (Y minus a neutral loss). Several readings can fit one step
//! (+17.0265 is `[M+NH4]+` and also the loss of NH3; +46.0055 is formate and also the loss of HCOOH): all are returned.

use mzlab_core::elements::{mass, mass_of, parse_formula, ELECTRON};

impl Tables {
    pub fn new(pol: i8) -> Self {
        let m = |e: &str| mass_of(e).expect("fixed element");
        let proton = m("H") - ELECTRON;
        let adducts = if pol >= 0 {
            vec![
                ("[M+Na]+".to_string(), m("Na") - ELECTRON - proton),
                ("[M+K]+".to_string(), m("K") - ELECTRON - proton),
                (
                    NH4_NAME.to_string(),
                    m("N") + 4.0 * m("H") - ELECTRON - proton,
                ),
                ("[M+H+ACN]+".to_string(), neutral("C2H3N")),
            ]
        } else {
            vec![
                (FORMATE_NAMES[0].to_string(), neutral("CH2O2")),
                ("[M+Cl]-".to_string(), m("Cl") + ELECTRON + proton),
                ("[M+Na-2H]-".to_string(), m("Na") - ELECTRON - proton),
            ]
        };
        let losses = LOSSES.iter().map(|n| (n.to_string(), neutral(n))).collect();
        Tables {
            pol: if pol >= 0 { 1 } else { -1 },
            adducts,
            losses,
            proton,
        }
    }

    /// Positive steps (Y minus X) that an adduct or a loss can give, with their names, in the order of the tables.
    pub fn steps(&self) -> impl Iterator<Item = (&str, RKind, f64)> + '_ {
        let a = self
            .adducts
            .iter()
            .map(|(n, d)| (n.as_str(), RKind::Adduct, *d));
        let l = self
            .losses
            .iter()
            .map(|(n, d)| (n.as_str(), RKind::Loss, *d));
        a.chain(l)
    }

    /// Is Y the dimer `[2M+H]+` (`[2M-H]-`) of X? The tolerance is relative to Y, as in `iimn.dimer_of`.
    pub fn is_dimer(&self, mz_x: f64, mz_y: f64, ppm: f64, abs_da: f64) -> bool {
        let tol = (mz_y * ppm * 1e-6).max(abs_da);
        let s = self.pol as f64;
        (mz_y - (2.0 * (mz_x - s * self.proton) + s * self.proton)).abs() <= tol
    }

    /// m/z of X for which Y is the dimer of X.
    pub fn dimer_base(&self, mz_y: f64) -> f64 {
        let s = self.pol as f64;
        (mz_y - s * self.proton) / 2.0 + s * self.proton
    }

    /// The readings of the step `mz_up - mz_lo` (`edge_evidence.relations_of`): adduct and isotope of X, loss of Y, dimer.
    pub fn readings(&self, mz_lo: f64, mz_up: f64, ppm: f64, abs_da: f64) -> Vec<Reading> {
        let delta = mz_up - mz_lo;
        let tol = (mz_lo * ppm * 1e-6).max(abs_da);
        let mut out = Vec::new();
        let mut push = |kind, name: &str, step: f64| {
            out.push(Reading {
                kind,
                name: name.to_string(),
                step,
                ppm: (delta - step) / mz_lo * 1e6,
            })
        };
        for (n, d) in &self.adducts {
            if (delta - d).abs() <= tol {
                push(RKind::Adduct, n, *d);
            }
        }
        for (n, d) in ISOTOPES {
            if (delta - d).abs() <= tol {
                push(RKind::Isotope, n, d);
            }
        }
        for (n, d) in &self.losses {
            if (delta - d).abs() <= tol {
                push(RKind::Loss, n, *d);
            }
        }
        if self.is_dimer(mz_lo, mz_up, ppm, abs_da) {
            let name = if self.pol > 0 { "[2M+H]+" } else { "[2M-H]-" };
            out.push(Reading {
                kind: RKind::Dimer,
                name: name.to_string(),
                step: delta,
                ppm: 0.0,
            });
        }
        out
    }

    /// `!readings(..).is_empty()` without building the readings.
    pub fn has_reading(&self, mz_lo: f64, mz_up: f64, ppm: f64, abs_da: f64) -> bool {
        let delta = mz_up - mz_lo;
        let tol = (mz_lo * ppm * 1e-6).max(abs_da);
        let near = |d: &f64| (delta - d).abs() <= tol;
        self.adducts.iter().any(|(_, d)| near(d))
            || ISOTOPES.iter().any(|(_, d)| near(d))
            || self.losses.iter().any(|(_, d)| near(d))
            || self.is_dimer(mz_lo, mz_up, ppm, abs_da)
    }

    /// Mass of the neutral NH3, the typical loss of the ammonium adduct.
    pub fn nh3(&self) -> f64 {
        self.losses[1].1
    }

    /// Is the reading one of the two ambiguous families: NH4 / loss of NH3, formate / loss of HCOOH?
    pub fn is_nh3(name: &str) -> bool {
        name == NH3_NAME || name == NH4_NAME
    }

    pub fn is_formate(name: &str) -> bool {
        FORMATE_NAMES.contains(&name)
    }
}

/// Isotope steps of `iimn.ISOTOPES` (only read by `readings`; the isotopologues of the file come from `isotopes`).
const ISOTOPES: [(&str, f64); 3] = [("13C", 1.003355), ("34S/13C2", 1.99580), ("13C2", 2.00671)];
/// `ionfamily.NEUTRAL_LOSSES`, then the alkene losses of `iimn.EXTRA_LOSSES` that are not already there, in the order of `iimn.LOSSES`.
const LOSSES: [&str; 23] = [
    "H2O", "NH3", "CO", "CO2", "CH2O", "CH4O", "C2H2O", "HCOOH", "C2H4", "C3H6", "HCN", "HF",
    "HCl", "HBr", "SO2", "SO3", "H2S", "CH3", "NO2", "C6H6", "C4H8", "C4H9N", "C4H8O",
];

pub const NH3_NAME: &str = "NH3";
pub const NH4_NAME: &str = "[M+NH4]+";
pub const FORMATE_NAMES: [&str; 2] = ["[M+FA-H]-", "HCOOH"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RKind {
    Adduct,
    Isotope,
    Loss,
    Dimer,
}

/// One reading of a step. `step` is the expected signed difference (Y minus X); `ppm` the error of the measured one.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    pub kind: RKind,
    pub name: String,
    pub step: f64,
    pub ppm: f64,
}

/// Tables of one polarity.
pub struct Tables {
    /// +1 or -1: sign of the proton in the base ion.
    pub pol: i8,
    adducts: Vec<(String, f64)>,
    losses: Vec<(String, f64)>,
    pub proton: f64,
}

fn neutral(f: &str) -> f64 {
    mass(&parse_formula(f).expect("fixed formula"))
}

#[cfg(test)]
#[path = "rel_tests.rs"]
mod tests;
