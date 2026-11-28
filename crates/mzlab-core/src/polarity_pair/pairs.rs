//! The matching: every positive row under each positive adduct against every negative row under each negative adduct. Two ions are a pair when the
//! neutral masses they imply differ by at most `mass_tol` and the RTs by at most `rt_tol`. Nothing is merged: every pair within both tolerances is kept,
//! and `mutual` marks the pairs that are the nearest of both rows (the distance is the larger-tolerance-scaled mass and RT error together).

use super::params::{Params, Unit};
use super::table::Row;

#[derive(Debug, Clone)]
pub struct Pair {
    pub pos: usize,
    pub neg: usize,
    pub pos_adduct: usize,
    pub neg_adduct: usize,
    /// Neutral masses implied by the positive and the negative ion.
    pub m_pos: f64,
    pub m_neg: f64,
    pub drt: f64,
    /// Both tolerances in one number: 1 = on the edge of either; the ranking of the candidates of a row.
    pub dist: f64,
    pub mutual: bool,
    /// How many pairs the positive row and the negative row are in (competing partners).
    pub pos_n: usize,
    pub neg_n: usize,
}

impl Pair {
    pub fn neutral(&self) -> f64 {
        (self.m_pos + self.m_neg) / 2.0
    }
    /// Negative minus positive neutral mass, Da.
    pub fn err_da(&self) -> f64 {
        self.m_neg - self.m_pos
    }
    pub fn err_ppm(&self) -> f64 {
        self.err_da() / self.neutral() * 1e6
    }
}

pub struct Matched {
    pub pairs: Vec<Pair>,
    pub pos_free: Vec<usize>,
    pub neg_free: Vec<usize>,
}

fn window(m: f64, p: &Params) -> f64 {
    match p.unit {
        Unit::Ppm => m.abs() * p.mass_tol * 1e-6,
        Unit::Da => p.mass_tol,
    }
}

fn scaled(err: f64, mass: f64, drt: f64, p: &Params) -> f64 {
    let tol = window(mass, p);
    let (a, b) = (
        err / tol.max(f64::MIN_POSITIVE),
        drt / p.rt_tol.max(f64::MIN_POSITIVE),
    );
    (a * a + b * b).sqrt()
}

pub fn run(pos: &[Row], neg: &[Row], p: &Params) -> Matched {
    // the negative ions as neutral masses, sorted: each positive ion looks up its window
    let mut ions: Vec<(f64, usize, usize)> = Vec::new();
    for (i, r) in neg.iter().enumerate() {
        for (a, ad) in p.neg.iter().enumerate() {
            ions.push((ad.neutral(r.mz), i, a));
        }
    }
    crate::sortc::slice_by(&mut ions, |x, y| {
        x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2))
    });
    let mut pairs: Vec<Pair> = Vec::new();
    for (i, r) in pos.iter().enumerate() {
        for (a, ad) in p.pos.iter().enumerate() {
            let m = ad.neutral(r.mz);
            let w = window(m, p) * (1.0 + 1e-9) + 1e-12;
            let from = ions.partition_point(|x| x.0 < m - w);
            for &(mn, j, b) in ions[from..].iter().take_while(|x| x.0 <= m + w) {
                let drt = neg[j].rt - r.rt;
                if (mn - m).abs() > window(m, p) || drt.abs() > p.rt_tol {
                    continue;
                }
                let dist = scaled(mn - m, m, drt, p);
                pairs.push(Pair {
                    pos: i,
                    neg: j,
                    pos_adduct: a,
                    neg_adduct: b,
                    m_pos: m,
                    m_neg: mn,
                    drt,
                    dist,
                    mutual: false,
                    pos_n: 0,
                    neg_n: 0,
                });
            }
        }
    }
    crate::sortc::slice_by(&mut pairs, |x, y| {
        x.neutral()
            .total_cmp(&y.neutral())
            .then_with(|| pos[x.pos].id.cmp(&pos[y.pos].id))
            .then_with(|| neg[x.neg].id.cmp(&neg[y.neg].id))
            .then(x.pos_adduct.cmp(&y.pos_adduct))
            .then(x.neg_adduct.cmp(&y.neg_adduct))
    });
    mark(&mut pairs, pos.len(), neg.len());
    let mut pos_seen = vec![false; pos.len()];
    let mut neg_seen = vec![false; neg.len()];
    for x in &pairs {
        pos_seen[x.pos] = true;
        neg_seen[x.neg] = true;
    }
    let free = |seen: Vec<bool>| (0..seen.len()).filter(|&i| !seen[i]).collect();
    Matched {
        pairs,
        pos_free: free(pos_seen),
        neg_free: free(neg_seen),
    }
}

/// Counts the partners of every row and marks the pairs that are the nearest (`dist`, the first in order on a tie) of both of their rows.
fn mark(pairs: &mut [Pair], n_pos: usize, n_neg: usize) {
    let mut best_pos: Vec<Option<usize>> = vec![None; n_pos];
    let mut best_neg: Vec<Option<usize>> = vec![None; n_neg];
    let (mut cp, mut cn) = (vec![0usize; n_pos], vec![0usize; n_neg]);
    for (k, x) in pairs.iter().enumerate() {
        cp[x.pos] += 1;
        cn[x.neg] += 1;
        if best_pos[x.pos].is_none_or(|b| x.dist < pairs[b].dist) {
            best_pos[x.pos] = Some(k);
        }
        if best_neg[x.neg].is_none_or(|b| x.dist < pairs[b].dist) {
            best_neg[x.neg] = Some(k);
        }
    }
    let flags: Vec<bool> = pairs
        .iter()
        .enumerate()
        .map(|(k, x)| best_pos[x.pos] == Some(k) && best_neg[x.neg] == Some(k))
        .collect();
    for (k, x) in pairs.iter_mut().enumerate() {
        x.mutual = flags[k];
        x.pos_n = cp[x.pos];
        x.neg_n = cn[x.neg];
    }
}
