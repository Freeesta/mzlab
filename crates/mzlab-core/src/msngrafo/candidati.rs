//! Candidate formulas of a fragment: every sub-formula of the ion of the root within a window of m/z, with its DBE, its electron parity
//! and its expected M+1, and the known losses. A formula here is a vector of counts aligned with the elements of the root (`Elementi`).

use super::PERDITE;
use crate::elements::{fmt, mass_of, parse_formula, Formula, ELECTRON};

/// Atomic number (electrons of the neutral atom) and valence of the elements of `elements::MASS`.
const ATOMI: [(&str, u32, u32); 14] = [
    ("H", 1, 1),
    ("C", 6, 4),
    ("N", 7, 3),
    ("O", 8, 2),
    ("F", 9, 1),
    ("P", 15, 3),
    ("S", 16, 2),
    ("Cl", 17, 1),
    ("Br", 35, 1),
    ("I", 53, 1),
    ("Na", 11, 1),
    ("K", 19, 1),
    ("Si", 14, 4),
    ("Se", 34, 2),
];

/// The enumeration stops here: a window of a few ppm never holds so many sub-formulas of one ion.
const MAX_CANDIDATI: usize = 4096;

/// The elements of the ion of the root, heaviest first (the enumeration cuts on the mass).
pub(super) struct Elementi {
    pub sim: Vec<String>,
    pub massa: Vec<f64>,
    pub z: Vec<u32>,
    pub valenza: Vec<u32>,
    pub max: Vec<u32>,
    /// 1 or -1: the charge of every ion of the graph.
    pub pol: i8,
    /// The counts of the known losses made only of elements of the root, with their name.
    pub perdite: Vec<(&'static str, Vec<u32>)>,
}

impl Elementi {
    pub fn massa(&self, c: &[u32]) -> f64 {
        crate::pysum::pysum(c.iter().zip(&self.massa).map(|(n, m)| *n as f64 * m))
    }

    /// m/z of the singly charged ion of these counts.
    pub fn mz(&self, c: &[u32]) -> f64 {
        self.massa(c) - self.pol as f64 * ELECTRON
    }

    /// Rings plus double bonds of the ion, 1 + Σ n·(valence − 2)/2: x.5 for an even-electron ion.
    pub fn dbe(&self, c: &[u32]) -> f64 {
        1.0 + c
            .iter()
            .zip(&self.valenza)
            .map(|(n, v)| *n as f64 * (*v as f64 - 2.0) / 2.0)
            .sum::<f64>()
    }

    /// Even number of electrons in the ion (of the neutral with `carica` 0).
    pub fn pari(&self, c: &[u32], carica: i8) -> bool {
        let e: i64 = c.iter().zip(&self.z).map(|(n, z)| (*n * z) as i64).sum();
        (e - carica as i64) % 2 == 0
    }

    /// The formula of a loss: its usual name when it is a known loss (`NH3`, not `H3N`), otherwise Hill order.
    pub fn testo_perdita(&self, d: &[u32]) -> String {
        match self.perdite.iter().find(|(_, c)| c == d) {
            Some((n, _)) => n.to_string(),
            None => self.testo(d),
        }
    }

    /// The formula in Hill order.
    pub fn testo(&self, c: &[u32]) -> String {
        let f: Formula = self
            .sim
            .iter()
            .cloned()
            .zip(c.iter().map(|n| *n as u64))
            .collect();
        fmt(&f)
    }

    /// The counts of a formula, if every element of it is in the root (zero counts allowed).
    pub fn conta(&self, f: &Formula) -> Option<Vec<u32>> {
        let mut c = vec![0u32; self.sim.len()];
        for (e, n) in f {
            match self.sim.iter().position(|s| s == e) {
                Some(i) => c[i] += *n as u32,
                None if *n == 0 => {}
                None => return None,
            }
        }
        Some(c)
    }
}

/// `a` is a sub-formula of `b`, element by element.
pub(super) fn sotto(a: &[u32], b: &[u32]) -> bool {
    a.iter().zip(b).all(|(x, y)| x <= y)
}

/// The enumeration of the sub-formulas, cut on the mass.
impl Elementi {
    /// Every sub-formula of the root whose ion has an m/z within [lo, hi] and a DBE ≥ −0.5, in the order of the enumeration.
    pub fn candidati(&self, lo: f64, hi: f64) -> Vec<Vec<u32>> {
        let sh = self.pol as f64 * ELECTRON;
        let (lo, hi) = (lo + sh, hi + sh);
        let n = self.sim.len();
        let mut resto = vec![0.0; n + 1];
        for i in (0..n).rev() {
            resto[i] = resto[i + 1] + self.max[i] as f64 * self.massa[i];
        }
        let mut out = vec![];
        let mut c = vec![0u32; n];
        self.scendi(0, 0.0, lo, hi, &resto, &mut c, &mut out);
        out.retain(|c| self.dbe(c) >= -0.5);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn scendi(
        &self,
        i: usize,
        m: f64,
        lo: f64,
        hi: f64,
        resto: &[f64],
        c: &mut Vec<u32>,
        out: &mut Vec<Vec<u32>>,
    ) {
        if out.len() >= MAX_CANDIDATI {
            return;
        }
        if i == c.len() {
            if m >= lo && m <= hi {
                out.push(c.clone());
            }
            return;
        }
        for k in 0..=self.max[i] {
            let m2 = m + k as f64 * self.massa[i];
            if m2 > hi {
                break;
            }
            if m2 + resto[i + 1] < lo {
                continue;
            }
            c[i] = k;
            self.scendi(i + 1, m2, lo, hi, resto, c, out);
        }
        c[i] = 0;
    }
}

pub(super) fn elementi(f: &Formula, pol: i8) -> Elementi {
    let mut v: Vec<(String, f64, u32, u32, u32)> = f
        .iter()
        .filter(|(_, n)| *n > 0)
        .map(|(e, n)| {
            let (_, z, val) = ATOMI
                .iter()
                .find(|a| a.0 == e)
                .copied()
                .unwrap_or(("", 0, 0));
            (e.clone(), mass_of(e).unwrap_or(0.0), z, val, *n as u32)
        })
        .collect();
    crate::sortc::slice_by(&mut v, |a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut e = Elementi {
        sim: v.iter().map(|x| x.0.clone()).collect(),
        massa: v.iter().map(|x| x.1).collect(),
        z: v.iter().map(|x| x.2).collect(),
        valenza: v.iter().map(|x| x.3).collect(),
        max: v.iter().map(|x| x.4).collect(),
        pol,
        perdite: vec![],
    };
    e.perdite = PERDITE
        .iter()
        .filter_map(|(n, _)| {
            parse_formula(n)
                .ok()
                .and_then(|f| e.conta(&f))
                .map(|c| (*n, c))
        })
        .collect();
    e
}

/// Abundance of the heavier isotope one mass unit up, % of the lightest (13C, 2H, 15N, 17O, 33S, 29Si, 77Se).
const M1: [(&str, f64); 7] = [
    ("C", 1.0816),
    ("H", 0.0115),
    ("N", 0.3653),
    ("O", 0.0381),
    ("S", 0.7893),
    ("Si", 5.0778),
    ("Se", 0.8),
];
/// An M+1 is coherent within this fraction of the expected one...
const M1_REL: f64 = 0.3;
/// ...or this many points of % (the M+1 of a small fragment is a few % and noisy).
const M1_ABS: f64 = 2.0;

/// The M+1 of the formula, % of the monoisotopic peak.
pub(super) fn m1_atteso(e: &Elementi, c: &[u32]) -> f64 {
    e.sim
        .iter()
        .zip(c)
        .map(|(s, n)| *n as f64 * M1.iter().find(|x| x.0 == s).map_or(0.0, |x| x.1))
        .sum()
}

/// The observed M+1 (% of the peak) agrees with the formula.
pub(super) fn m1_coerente(e: &Elementi, c: &[u32], osservato: f64) -> bool {
    let a = m1_atteso(e, c);
    (osservato - a).abs() <= (M1_REL * a).max(M1_ABS)
}

/// The loss from `p` to `c` (`c` under `p`) is a known one.
pub(super) fn perdita_nota(e: &Elementi, p: &[u32], c: &[u32]) -> bool {
    let d: Vec<u32> = p.iter().zip(c).map(|(a, b)| a - b).collect();
    e.perdite.iter().any(|(_, x)| *x == d)
}
