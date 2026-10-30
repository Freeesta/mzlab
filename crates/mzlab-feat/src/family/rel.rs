//! What the difference of m/z between two co-eluting ions can be: the tables of mzFinder (`relations`, `dimer_of`) read in
//! the direction of `edge_evidence.relations_of`: the lighter ion X, the heavier Y.
//!
//! Y is an adduct of X, Y is the dimer of X, or X is an in-source fragment of Y (Y minus a neutral loss). Several readings can fit one step
//! (+17.0265 is `[M+NH4]+` and also the loss of NH3; +46.0055 is formate and also the loss of HCOOH): all are returned.

use crate::legacy::edge::{Iimn, ISOTOPES};

impl Tables {
    pub fn new(pol: i8) -> Self {
        let t = Iimn::new();
        let positive = pol >= 0;
        Tables {
            pol: if positive { 1 } else { -1 },
            adducts: t
                .adducts_of(if positive { 1 } else { -1 })
                .iter()
                .map(|&(n, d)| (n.to_string(), d))
                .collect(),
            losses: t.losses.iter().map(|&(n, d)| (n.to_string(), d)).collect(),
            proton: t.proton,
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

#[cfg(test)]
#[path = "rel_tests.rs"]
mod tests;
