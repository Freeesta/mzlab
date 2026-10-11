//! Deterministic decoys and q-values for the library search (target-decoy). The decoys are ordinary library entries marked by an accession prefix,
//! so the search engine is the same with or without them. The number is a LOWER BOUND of the true error rate (see `fdr_note`).

use crate::error::Result;
use crate::io::Source;
use crate::read::Library;
use crate::search::{identity, IdentityOpts, Query};
use crate::sortutil;
use crate::spec::Spectrum;

/// Accession prefix that marks a decoy entry in the metadata.
pub const DECOY_PREFIX: &str = "DECOY:";
/// Precursor shift of the second kind of decoy, in Da (always outside the search tolerances).
pub const SHIFT_MIN: f64 = 5.0;
pub const SHIFT_MAX: f64 = 50.0;
/// Global seed used by the callers unless they pick another one.
pub const DEFAULT_SEED: u64 = 0x6d7a_4c61_6246_4452;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoyKind {
    /// Fragment m/z replaced by pseudo-random values in the range of the spectrum, intensities kept.
    Fragments,
    /// Same peaks, precursor moved by a pseudo-random delta of 5-50 Da.
    Precursor,
}

impl DecoyKind {
    pub fn code(self) -> u8 {
        match self {
            DecoyKind::Fragments => 0,
            DecoyKind::Precursor => 1,
        }
    }
}

/// q-value of a library hit with the counts it comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LibFdr {
    pub q_value: f32,
    pub n_target_ge: u32,
    pub n_decoy_ge: u32,
    pub decoy_kind: DecoyKind,
}

/// i18n key of the text that must accompany every q-value of the library search («lower bound»: isomers and analogues are not random errors).
pub fn fdr_note() -> &'static str {
    "lib.fdr.note"
}

/// xorshift64* with a fixed multiplier: the whole generator, written by hand.
pub struct XorShift(u64);

impl XorShift {
    pub fn new(seed: u64) -> XorShift {
        XorShift(if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

/// Seed of the decoy of one entry: FNV-1a of its identifiers (accession, name, InChIKey, precursor bits) mixed with the global seed.
/// It never depends on the position in the file or on a clock.
pub fn decoy_seed(s: &Spectrum, global: u64) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ global;
    for f in [&s.meta.accession, &s.meta.name, &s.meta.inchikey] {
        fnv(&mut h, f.as_bytes());
        fnv(&mut h, &[0xff]);
    }
    fnv(&mut h, &s.prec.to_bits().to_le_bytes());
    fnv(&mut h, &(s.mz.len() as u64).to_le_bytes());
    h
}

pub fn is_decoy(accession: &str) -> bool {
    accession.starts_with(DECOY_PREFIX)
}

/// One decoy of `s`. The metadata keep the polarity and the licence but carry the prefix in the accession and no chemistry.
pub fn make_decoy(s: &Spectrum, kind: DecoyKind, seed: u64) -> Spectrum {
    let mut r = XorShift::new(seed);
    let mut d = s.clone();
    match kind {
        DecoyKind::Fragments => {
            let lo = s.mz.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = s.mz.iter().cloned().fold(0.0, f64::max);
            // stay below the precursor region that the cleaning removes
            let hi = if s.prec - 2.0 > lo {
                hi.min(s.prec - 2.0)
            } else {
                hi
            };
            d.mz = s.mz.iter().map(|_| lo + (hi - lo) * r.unit()).collect();
            d.mz.iter_mut().for_each(|m| *m = (*m * 1e4).round() / 1e4);
        }
        DecoyKind::Precursor => {
            let delta = SHIFT_MIN + (SHIFT_MAX - SHIFT_MIN) * r.unit();
            d.prec = s.prec + if r.next_u64() & 1 == 0 { delta } else { -delta };
        }
    }
    d.meta.accession = format!("{DECOY_PREFIX}{}", s.meta.accession);
    d.meta.name = String::from("decoy");
    d.meta.formula.clear();
    d.meta.inchikey.clear();
    d.meta.smiles.clear();
    d
}

/// The decoys of a whole library: one per target, same size, same order.
pub fn make_decoys(targets: &[Spectrum], kind: DecoyKind, global_seed: u64) -> Vec<Spectrum> {
    targets
        .iter()
        .map(|s| make_decoy(s, kind, decoy_seed(s, global_seed)))
        .collect()
}

/// Targets followed by their decoys: what the builder indexes.
pub fn with_decoys(targets: &[Spectrum], kind: DecoyKind, global_seed: u64) -> Vec<Spectrum> {
    let mut v = targets.to_vec();
    v.extend(make_decoys(targets, kind, global_seed));
    v
}

/// Best target score and best decoy score of one query (0 where nothing was found).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Best {
    pub target: f32,
    pub decoy: f32,
}

/// Runs the identity search and splits the candidates into targets and decoys by the accession prefix (the score is the entropy similarity).
pub fn best_scores<S: Source>(lib: &Library<S>, q: &Query, o: &IdentityOpts) -> Result<Best> {
    let mut b = Best::default();
    for h in identity(lib, q, o)? {
        let d = is_decoy(&lib.meta(h.index)?.accession);
        let slot = if d { &mut b.decoy } else { &mut b.target };
        *slot = slot.max(h.entropy as f32);
    }
    Ok(b)
}

fn count_ge(sorted_desc: &[f32], s: f32) -> usize {
    sorted_desc.partition_point(|&x| x >= s)
}

/// Standard target-decoy q-values: FDR(s) = #decoy ≥ s / #target ≥ s, q = the minimum of the FDR over all scores ≤ s (monotone).
/// One value per target, in the order given; equal scores get equal values, so the result does not depend on the order of the input.
/// A score that is not finite gets q = 1.
pub fn lib_fdr(scores_target: &[f32], scores_decoy: &[f32], kind: DecoyKind) -> Vec<LibFdr> {
    let finite = |v: &[f32]| -> Vec<f32> { v.iter().cloned().filter(|x| x.is_finite()).collect() };
    let mut t = finite(scores_target);
    let mut d = finite(scores_decoy);
    sortutil::sort_by(&mut t, |a, b| b.total_cmp(a));
    sortutil::sort_by(&mut d, |a, b| b.total_cmp(a));
    // distinct target scores from the lowest: running minimum of the FDR
    let mut q_of: Vec<(f32, LibFdr)> = Vec::new();
    let mut run = 1.0f32;
    let mut i = t.len();
    while i > 0 {
        let s = t[i - 1];
        let nt = count_ge(&t, s);
        let nd = count_ge(&d, s);
        run = run.min((nd as f32 / nt as f32).min(1.0));
        q_of.push((
            s,
            LibFdr {
                q_value: run,
                n_target_ge: nt as u32,
                n_decoy_ge: nd as u32,
                decoy_kind: kind,
            },
        ));
        i = t.partition_point(|&x| x > s);
    }
    let none = LibFdr {
        q_value: 1.0,
        n_target_ge: 0,
        n_decoy_ge: 0,
        decoy_kind: kind,
    };
    scores_target
        .iter()
        .map(|&s| {
            if !s.is_finite() {
                return none;
            }
            let k = q_of.partition_point(|(x, _)| *x < s);
            q_of[k].1
        })
        .collect()
}

/// The q-values alone (see `lib_fdr`).
pub fn qvalues(scores_target: &[f32], scores_decoy: &[f32]) -> Vec<f32> {
    lib_fdr(scores_target, scores_decoy, DecoyKind::Fragments)
        .iter()
        .map(|f| f.q_value)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qvalues_small_case_by_hand() {
        // targets 0.9 0.8 0.7 0.6 0.5, decoys 0.75 0.55: FDR 0, 0, 1/3, 1/4, 2/5 → q 0, 0, 0.25, 0.25, 0.4
        let q = qvalues(&[0.9, 0.8, 0.7, 0.6, 0.5], &[0.75, 0.55]);
        assert_eq!(q, vec![0.0, 0.0, 0.25, 0.25, 0.4]);
    }

    #[test]
    fn qvalues_do_not_depend_on_the_order() {
        let t = [0.5f32, 0.9, 0.7, 0.7, 0.8];
        let d = [0.7f32, 0.3, 0.75];
        let a = qvalues(&t, &d);
        let tr: Vec<f32> = t.iter().rev().cloned().collect();
        let dr: Vec<f32> = d.iter().rev().cloned().collect();
        let mut b = qvalues(&tr, &dr);
        b.reverse();
        assert_eq!(a, b);
        assert_eq!(a[2], a[3]);
    }

    #[test]
    fn decoys_are_deterministic_and_keep_the_shape() {
        let s = Spectrum {
            prec: 300.0,
            pol: 1,
            mz: vec![60.0, 90.0, 120.0, 200.0],
            it: vec![1.0, 2.0, 3.0, 4.0],
            ..Default::default()
        };
        let a = make_decoy(&s, DecoyKind::Fragments, decoy_seed(&s, 1));
        assert_eq!(a, make_decoy(&s, DecoyKind::Fragments, decoy_seed(&s, 1)));
        assert_ne!(a.mz, s.mz);
        assert_eq!(a.it, s.it);
        assert!(a.mz.iter().all(|&m| (60.0..=200.0).contains(&m)));
        let p = make_decoy(&s, DecoyKind::Precursor, decoy_seed(&s, 1));
        assert!((p.prec - s.prec).abs() >= SHIFT_MIN && (p.prec - s.prec).abs() <= SHIFT_MAX);
        assert_eq!(p.mz, s.mz);
        assert_ne!(decoy_seed(&s, 1), decoy_seed(&s, 2));
    }
}
