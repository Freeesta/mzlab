//! Deterministic synthetic spectra for the tests (no compound names anywhere).
#![allow(dead_code)]
use mzlab_lib::spec::{Meta, Spectrum};

pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    pub fn unit(&mut self) -> f64 {
        (self.next() % 1_000_000) as f64 / 1_000_000.0
    }
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.unit()
    }
}

/// A spectrum with well separated peaks (> 0.1 Da apart), `n` peaks below the precursor − 2.
pub fn synth(r: &mut Rng, i: usize, n: usize) -> Spectrum {
    let prec = r.range(150.0, 900.0);
    let top = prec - 3.0;
    let mut mz: Vec<f64> = Vec::new();
    let mut m = 50.0 + r.range(0.0, 20.0);
    while mz.len() < n && m < top {
        mz.push((m * 10000.0).round() / 10000.0);
        m += r.range(0.2, ((top - 50.0) / n as f64 * 1.6).max(0.3));
    }
    let it: Vec<f64> = mz
        .iter()
        .map(|_| 1.0 + r.unit() * r.unit() * 10_000.0)
        .collect();
    let pol = match i % 7 {
        0 => -1,
        1 => 0,
        _ => 1,
    };
    Spectrum {
        prec: (prec * 10000.0).round() / 10000.0,
        pol,
        mz,
        it,
        meta: Meta {
            name: format!("Synthetic {i}"),
            adduct: if pol >= 0 {
                "[M+H]+".into()
            } else {
                "[M-H]-".into()
            },
            ce: "30".into(),
            instrument: "Orbitrap".into(),
            formula: "C1H1".into(),
            inchikey: format!("AAAAAAAAAAAAAA-{i:010}-N"),
            smiles: "C".into(),
            accession: format!("SYN{i:06}"),
            authors: "A. Author".into(),
            license: if i % 3 == 0 {
                "CC BY".into()
            } else {
                "CC0".into()
            },
        },
    }
}

pub fn synth_set(seed: u64, count: usize, peaks: usize) -> Vec<Spectrum> {
    let mut r = Rng(seed);
    (0..count).map(|i| synth(&mut r, i, peaks)).collect()
}
