//! Benchmark statistics (`tools/bench/statistica.py`): exact McNemar, Holm step-down correction,
//! bootstrap confidence intervals (mean, median), paired difference and non-inferiority/superiority.

use std::cmp::Ordering;

use crate::sortc::{slice_by, slice_ord};

pub const N_BOOT_DEFAULT: usize = 2000;
pub const SEME_DEFAULT: u64 = 20261012;
pub const ALFA_DEFAULT: f64 = 0.05;

/// Exact two-sided McNemar test on paired 0/1 outcomes.
///
/// Discordant pairs:
/// - `n10 = count(a == 1 && b == 0)` (solo a)
/// - `n01 = count(a == 0 && b == 1)` (solo b)
///
/// Returns `(n10, n01, p_value)`.
/// Under the null, discordant pairs follow Binomial(n10 + n01, 0.5).
pub fn mcnemar_esatto(a: &[i32], b: &[i32]) -> (usize, usize, f64) {
    let mut n10 = 0usize;
    let mut n01 = 0usize;
    for (&x, &y) in a.iter().zip(b.iter()) {
        if x == 1 && y == 0 {
            n10 += 1;
        } else if x == 0 && y == 1 {
            n01 += 1;
        }
    }
    let p = mcnemar_p_value(n10, n01);
    (n10, n01, p)
}

/// Numerically stable exact McNemar p-value for `n = n10 + n01` up to > 1000 without overflow.
pub fn mcnemar_p_value(n10: usize, n01: usize) -> f64 {
    let n = n10 + n01;
    if n == 0 {
        return 1.0;
    }
    let k = n10.min(n01);
    if k * 2 >= n {
        return 1.0;
    }
    if n <= 1022 {
        let mut p = 0.5_f64.powi(n as i32);
        let mut sum = p;
        for i in 1..=k {
            p = p * (n - i + 1) as f64 / i as f64;
            sum += p;
        }
        (2.0 * sum).min(1.0)
    } else {
        let mut log_p = 0.0_f64;
        for j in 1..=k {
            log_p += ((n - j + 1) as f64 / j as f64).ln();
        }
        log_p -= n as f64 * std::f64::consts::LN_2;
        if log_p < -740.0 {
            return 0.0;
        }
        let mut cur = log_p.exp();
        let mut sum = cur;
        for i in (1..=k).rev() {
            cur = cur * i as f64 / (n - i + 1) as f64;
            sum += cur;
            if cur == 0.0 && sum > 0.0 {
                break;
            }
        }
        (2.0 * sum).min(1.0)
    }
}

/// Holm step-down procedure: adjusted p-values (monotone, capped at 1.0)
/// and rejected hypotheses at significance level `alfa`, in the original input order.
pub fn holm(p: &[f64], alfa: f64) -> (Vec<f64>, Vec<bool>) {
    let m = p.len();
    if m == 0 {
        return (Vec::new(), Vec::new());
    }
    let mut ordine: Vec<usize> = (0..m).collect();
    slice_by(&mut ordine, |&i, &j| {
        let pi = p.get(i).copied().unwrap_or(f64::NAN);
        let pj = p.get(j).copied().unwrap_or(f64::NAN);
        match pi.partial_cmp(&pj) {
            Some(ord) => ord.then_with(|| i.cmp(&j)),
            None => i.cmp(&j),
        }
    });
    let mut adj = vec![0.0_f64; m];
    let mut corrente = 0.0_f64;
    for (r, &i) in ordine.iter().enumerate() {
        let pi = p.get(i).copied().unwrap_or(0.0);
        let unadj = (m - r) as f64 * pi;
        corrente = corrente.max(unadj.min(1.0));
        if let Some(slot) = adj.get_mut(i) {
            *slot = corrente;
        }
    }
    let rifiutata: Vec<bool> = adj.iter().map(|&x| x < alfa).collect();
    (adj, rifiutata)
}

/// Non-inferiority of `a - b`:
/// - `direzione_alto` (higher is better): `ic_basso > margine`
/// - otherwise (lower is better): `ic_alto < margine`
pub fn non_inferiore(ic_basso: f64, ic_alto: f64, direzione_alto: bool, margine: f64) -> bool {
    if direzione_alto {
        ic_basso > margine
    } else {
        ic_alto < margine
    }
}

/// Superiority of `a` over `b`:
/// - `direzione_alto`: `ic_basso > 0.0 && rifiutata`
/// - otherwise: `ic_alto < 0.0 && rifiutata`
pub fn superiore(ic_basso: f64, ic_alto: f64, direzione_alto: bool, rifiutata: bool) -> bool {
    let lato = if direzione_alto {
        ic_basso > 0.0
    } else {
        ic_alto < 0.0
    };
    lato && rifiutata
}

/// Deterministic 64-bit PRNG (SplitMix64) for reproducible bootstrap draws.
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn gen_range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}

/// Generates `n_boot` resamples of `n` indices with replacement.
pub fn indici_bootstrap(n: usize, n_boot: usize, seed: u64) -> Vec<Vec<usize>> {
    if n == 0 || n_boot == 0 {
        return Vec::new();
    }
    let mut rng = SplitMix64::new(seed);
    let mut out = Vec::with_capacity(n_boot);
    for _ in 0..n_boot {
        let mut row = Vec::with_capacity(n);
        for _ in 0..n {
            row.push(rng.gen_range(n));
        }
        out.push(row);
    }
    out
}

/// Generates `n_boot` resamples of indices stratified by stratum labels.
pub fn indici_bootstrap_strati(strati: &[usize], n_boot: usize, seed: u64) -> Vec<Vec<usize>> {
    let n = strati.len();
    if n == 0 || n_boot == 0 {
        return Vec::new();
    }
    let mut distinct = strati.to_vec();
    slice_ord(&mut distinct);
    distinct.dedup();

    let mut stratum_indices: Vec<Vec<usize>> = Vec::with_capacity(distinct.len());
    for &s in &distinct {
        let pos: Vec<usize> = strati
            .iter()
            .enumerate()
            .filter_map(|(idx, &val)| if val == s { Some(idx) } else { None })
            .collect();
        stratum_indices.push(pos);
    }

    let mut rng = SplitMix64::new(seed);
    let mut out = vec![vec![0usize; n]; n_boot];
    let mut col = 0usize;
    for pos in stratum_indices {
        let len_pos = pos.len();
        if len_pos == 0 {
            continue;
        }
        for b in 0..n_boot {
            for j in 0..len_pos {
                let pick = rng.gen_range(len_pos);
                if let (Some(row), Some(&p)) = (out.get_mut(b), pos.get(pick)) {
                    if let Some(target) = row.get_mut(col + j) {
                        *target = p;
                    }
                }
            }
        }
        col += len_pos;
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatTipo {
    Media,
    Mediana,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntervalloBootstrap {
    pub n: usize,
    pub valore: Option<f64>,
    pub ic_basso: Option<f64>,
    pub ic_alto: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DifferenzaBootstrap {
    pub n: usize,
    pub valore: Option<f64>,
    pub ic_basso: Option<f64>,
    pub ic_alto: Option<f64>,
    pub p: Option<f64>,
}

pub fn mean(vals: &[f64]) -> Option<f64> {
    if vals.is_empty() {
        return None;
    }
    let sum: f64 = vals.iter().sum();
    Some(sum / vals.len() as f64)
}

pub fn nanmedian(vals: &[f64]) -> Option<f64> {
    let mut valid: Vec<f64> = vals.iter().copied().filter(|x| !x.is_nan()).collect();
    if valid.is_empty() {
        return None;
    }
    slice_by(&mut valid, |a, b| match a.partial_cmp(b) {
        Some(ord) => ord,
        None => Ordering::Equal,
    });
    let m = valid.len();
    if m % 2 == 1 {
        valid.get(m / 2).copied()
    } else {
        let v0 = valid.get(m / 2 - 1).copied().unwrap_or(0.0);
        let v1 = valid.get(m / 2).copied().unwrap_or(0.0);
        Some((v0 + v1) / 2.0)
    }
}

pub fn percentile(sorted_vals: &[f64], q: f64) -> Option<f64> {
    if sorted_vals.is_empty() {
        return None;
    }
    let n = sorted_vals.len();
    if n == 1 {
        return sorted_vals.first().copied();
    }
    let q_clamped = q.clamp(0.0, 100.0);
    let idx = (n - 1) as f64 * (q_clamped / 100.0);
    let i0 = idx.floor() as usize;
    let i1 = (i0 + 1).min(n - 1);
    let f = idx - idx.floor();
    let v0 = sorted_vals.get(i0).copied().unwrap_or(0.0);
    let v1 = sorted_vals.get(i1).copied().unwrap_or(0.0);
    Some((1.0 - f) * v0 + f * v1)
}

pub fn nanpercentile(vals: &[f64], q: f64) -> Option<f64> {
    let mut valid: Vec<f64> = vals.iter().copied().filter(|x| !x.is_nan()).collect();
    if valid.is_empty() {
        return None;
    }
    slice_by(&mut valid, |a, b| match a.partial_cmp(b) {
        Some(ord) => ord,
        None => Ordering::Equal,
    });
    percentile(&valid, q)
}

fn calc_stat(x: &[f64], tipo: StatTipo) -> Option<f64> {
    match tipo {
        StatTipo::Media => mean(x),
        StatTipo::Mediana => nanmedian(x),
    }
}

/// Point estimate and 95% bootstrap percentile interval.
pub fn intervallo(x: &[f64], tipo: StatTipo, n_boot: usize, seed: u64) -> IntervalloBootstrap {
    let n = x.len();
    let punto = calc_stat(x, tipo);
    if n == 0 || (tipo == StatTipo::Mediana && punto.is_none()) {
        return IntervalloBootstrap {
            n,
            valore: None,
            ic_basso: None,
            ic_alto: None,
        };
    }
    let indices = indici_bootstrap(n, n_boot, seed);
    let mut s = Vec::with_capacity(indices.len());
    for row in &indices {
        let sample: Vec<f64> = row.iter().filter_map(|&idx| x.get(idx).copied()).collect();
        s.push(calc_stat(&sample, tipo).unwrap_or(f64::NAN));
    }
    let ic_basso = nanpercentile(&s, 2.5);
    let ic_alto = nanpercentile(&s, 97.5);
    IntervalloBootstrap {
        n,
        valore: punto,
        ic_basso,
        ic_alto,
    }
}

/// Paired difference `a - b`: point estimate, 95% percentile interval, and two-sided bootstrap p-value.
pub fn differenza(
    a: &[f64],
    b: &[f64],
    tipo: StatTipo,
    n_boot: usize,
    seed: u64,
) -> Option<DifferenzaBootstrap> {
    if a.len() != b.len() {
        return None;
    }
    let n = a.len();
    let pt_a = calc_stat(a, tipo);
    let pt_b = calc_stat(b, tipo);
    if n == 0 || (tipo == StatTipo::Mediana && (pt_a.is_none() || pt_b.is_none())) {
        return Some(DifferenzaBootstrap {
            n,
            valore: None,
            ic_basso: None,
            ic_alto: None,
            p: None,
        });
    }
    let diff_punto = match (pt_a, pt_b) {
        (Some(va), Some(vb)) => Some(va - vb),
        _ => None,
    };
    let indices = indici_bootstrap(n, n_boot, seed);
    let mut d = Vec::with_capacity(indices.len());
    for row in &indices {
        let sa: Vec<f64> = row.iter().filter_map(|&i| a.get(i).copied()).collect();
        let sb: Vec<f64> = row.iter().filter_map(|&i| b.get(i).copied()).collect();
        let st_a = calc_stat(&sa, tipo).unwrap_or(f64::NAN);
        let st_b = calc_stat(&sb, tipo).unwrap_or(f64::NAN);
        let diff = st_a - st_b;
        if !diff.is_nan() {
            d.push(diff);
        }
    }
    if d.is_empty() {
        return Some(DifferenzaBootstrap {
            n,
            valore: diff_punto,
            ic_basso: None,
            ic_alto: None,
            p: None,
        });
    }
    let mut d_sorted = d.clone();
    slice_by(&mut d_sorted, |x, y| match x.partial_cmp(y) {
        Some(ord) => ord,
        None => Ordering::Equal,
    });
    let ic_basso = percentile(&d_sorted, 2.5);
    let ic_alto = percentile(&d_sorted, 97.5);
    let d_len = d.len() as f64;
    let le_zero = d.iter().filter(|&&v| v <= 0.0).count() as f64;
    let ge_zero = d.iter().filter(|&&v| v >= 0.0).count() as f64;
    let p_val =
        (2.0 * ((le_zero + 1.0) / (d_len + 1.0)).min((ge_zero + 1.0) / (d_len + 1.0))).min(1.0);
    Some(DifferenzaBootstrap {
        n,
        valore: diff_punto,
        ic_basso,
        ic_alto,
        p: Some(p_val),
    })
}
