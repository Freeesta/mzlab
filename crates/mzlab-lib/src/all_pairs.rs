//! All-pairs modified cosine similarity matrix with prefilter.
//!
//! Provides dense fragment/loss binning (`bin_matrices`), upper-bound prefilter
//! matrix products (`upper_bound`), candidate pair extraction (`pairs_above`),
//! and greedy plain cosine scoring (`plain_cosine`).

#[derive(Debug, Clone, PartialEq)]
pub struct BinnedMatrices {
    pub f: Vec<Vec<f32>>,
    pub l: Vec<Vec<f32>>,
    pub fs: Vec<Vec<f32>>,
    pub ls: Vec<Vec<f32>>,
    pub n_bins: usize,
}

/// Computes circular modular smear of a binned matrix row by row.
fn smear_matrix(x: &[Vec<f32>], nb: usize) -> Vec<Vec<f32>> {
    let mut xs = Vec::with_capacity(x.len());
    for row in x {
        let mut smeared = vec![0.0f32; nb];
        if nb == 0 {
            xs.push(smeared);
            continue;
        }
        if nb == 1 {
            if let Some(&val) = row.first() {
                smeared[0] = val;
            }
            xs.push(smeared);
            continue;
        }
        for b in 0..nb {
            let prev = if b == 0 { nb - 1 } else { b - 1 };
            let next = if b + 1 == nb { 0 } else { b + 1 };
            let curr = match row.get(b) {
                Some(&v) => v,
                None => 0.0,
            };
            let p_val = match row.get(prev) {
                Some(&v) => v,
                None => 0.0,
            };
            let n_val = match row.get(next) {
                Some(&v) => v,
                None => 0.0,
            };
            if let Some(slot) = smeared.get_mut(b) {
                *slot = curr.max(p_val).max(n_val);
            }
        }
        xs.push(smeared);
    }
    xs
}

/// Bins fragment masses (F) and neutral losses (L) into `binw`-wide bins up to `mmax`.
///
/// Returns `BinnedMatrices` containing `F`, `L`, and their circular smeared counterparts `Fs`, `Ls`.
pub fn bin_matrices(
    m: &[Vec<f64>],
    w: &[Vec<f32>],
    p: &[f64],
    binw: f64,
    mmax: f64,
) -> BinnedMatrices {
    if binw <= 0.0 || !binw.is_finite() || mmax < 0.0 || !mmax.is_finite() {
        let n = m.len();
        return BinnedMatrices {
            f: vec![vec![0.0f32; 2]; n],
            l: vec![vec![0.0f32; 2]; n],
            fs: vec![vec![0.0f32; 2]; n],
            ls: vec![vec![0.0f32; 2]; n],
            n_bins: 2,
        };
    }
    let nb = ((mmax / binw) as usize).saturating_add(2);
    let n = m.len();
    let mut f = vec![vec![0.0f32; nb]; n];
    let mut l = vec![vec![0.0f32; nb]; n];

    for (i, m_row) in m.iter().enumerate() {
        let prec = match p.get(i) {
            Some(&val) => val,
            None => 0.0,
        };
        let w_row = w.get(i);
        for (j, &mz) in m_row.iter().enumerate() {
            if mz <= 0.0 || !mz.is_finite() {
                continue;
            }
            let weight = match w_row.and_then(|row| row.get(j)) {
                Some(&wt) => wt,
                None => 0.0f32,
            };
            if weight <= 0.0 || !weight.is_finite() {
                continue;
            }
            let fb_raw = mz / binw + 0.5;
            let fb = if fb_raw <= 0.0 {
                0
            } else {
                (fb_raw as usize).min(nb.saturating_sub(1))
            };
            let lb_raw = (prec - mz) / binw + 0.5;
            let lb = if lb_raw <= 0.0 {
                0
            } else {
                (lb_raw as usize).min(nb.saturating_sub(1))
            };
            if let Some(row) = f.get_mut(i) {
                if let Some(val) = row.get_mut(fb) {
                    *val = val.max(weight);
                }
            }
            if let Some(row) = l.get_mut(i) {
                if let Some(val) = row.get_mut(lb) {
                    *val = val.max(weight);
                }
            }
        }
    }

    let fs = smear_matrix(&f, nb);
    let ls = smear_matrix(&l, nb);
    BinnedMatrices {
        f,
        l,
        fs,
        ls,
        n_bins: nb,
    }
}

/// Calculates prefilter upper bound matrix products `F @ Fs.T + L @ Ls.T`.
pub fn upper_bound(
    bm: &BinnedMatrices,
    rows: Option<&[usize]>,
    cols: Option<&[usize]>,
) -> Vec<Vec<f32>> {
    let r_all: Vec<usize>;
    let r_slice = match rows {
        Some(r) => r,
        None => {
            r_all = (0..bm.f.len()).collect();
            &r_all
        }
    };
    let c_all: Vec<usize>;
    let c_slice = match cols {
        Some(c) => c,
        None => {
            c_all = (0..bm.f.len()).collect();
            &c_all
        }
    };
    let mut u = vec![vec![0.0f32; c_slice.len()]; r_slice.len()];
    for (i, &r) in r_slice.iter().enumerate() {
        let f_r = bm.f.get(r);
        let l_r = bm.l.get(r);
        for (j, &c) in c_slice.iter().enumerate() {
            let fs_c = bm.fs.get(c);
            let ls_c = bm.ls.get(c);
            if let (Some(f_row), Some(l_row), Some(fs_row), Some(ls_row)) = (f_r, l_r, fs_c, ls_c) {
                let dot_f: f32 = f_row.iter().zip(fs_row.iter()).map(|(&a, &b)| a * b).sum();
                let dot_l: f32 = l_row.iter().zip(ls_row.iter()).map(|(&a, &b)| a * b).sum();
                if let Some(row_mut) = u.get_mut(i) {
                    if let Some(cell) = row_mut.get_mut(j) {
                        *cell = dot_f + dot_l;
                    }
                }
            }
        }
    }
    u
}

/// Returns index pairs `(ia, ib)` where `U[i][j] >= threshold`.
///
/// If `upper_triangle` is true, only pairs with `i < j` are kept.
pub fn pairs_above(
    u: &[Vec<f32>],
    threshold: f32,
    upper_triangle: bool,
) -> (Vec<usize>, Vec<usize>) {
    let mut ia = Vec::new();
    let mut ib = Vec::new();
    for (i, row) in u.iter().enumerate() {
        for (j, &val) in row.iter().enumerate() {
            if upper_triangle && j <= i {
                continue;
            }
            if val >= threshold {
                ia.push(i);
                ib.push(j);
            }
        }
    }
    (ia, ib)
}

#[derive(Debug, Clone)]
struct MatchCandidate {
    weight: f64,
    u: usize,
    v: usize,
}

fn score_single_pair(
    m_a: &[f64],
    w_a: &[f64],
    m_b: &[f64],
    w_b: &[f64],
    tol: f64,
) -> f64 {
    let mut candidates = Vec::new();
    for (u, &mz_a) in m_a.iter().enumerate() {
        let wt_a = match w_a.get(u) {
            Some(&val) => val,
            None => 0.0,
        };
        if wt_a <= 0.0 || !wt_a.is_finite() {
            continue;
        }
        for (v, &mz_b) in m_b.iter().enumerate() {
            let wt_b = match w_b.get(v) {
                Some(&val) => val,
                None => 0.0,
            };
            if wt_b <= 0.0 || !wt_b.is_finite() {
                continue;
            }
            if (mz_b - mz_a).abs() <= tol {
                candidates.push(MatchCandidate {
                    weight: wt_a * wt_b,
                    u,
                    v,
                });
            }
        }
    }

    candidates.sort_by(|x, y| {
        y.weight
            .total_cmp(&x.weight)
            .then_with(|| x.u.cmp(&y.u))
            .then_with(|| x.v.cmp(&y.v))
    });

    let mut matched_u = vec![false; m_a.len()];
    let mut matched_v = vec![false; m_b.len()];
    let mut score = 0.0f64;

    for cand in candidates {
        let u_free = match matched_u.get(cand.u) {
            Some(&done) => !done,
            None => false,
        };
        let v_free = match matched_v.get(cand.v) {
            Some(&done) => !done,
            None => false,
        };
        if u_free && v_free {
            if let Some(m) = matched_u.get_mut(cand.u) {
                *m = true;
            }
            if let Some(m) = matched_v.get_mut(cand.v) {
                *m = true;
            }
            score += cand.weight;
        }
    }
    score
}

/// Cosine similarity with no precursor shift (peaks at the same m/z only within `tol`),
/// matched 1-to-1 greedily by descending candidate weight.
pub fn plain_cosine(
    m: &[Vec<f64>],
    w: &[Vec<f64>],
    ia: &[usize],
    ib: &[usize],
    tol: f64,
) -> Vec<f64> {
    let n_pairs = ia.len().min(ib.len());
    let mut scores = Vec::with_capacity(n_pairs);
    for k in 0..n_pairs {
        let a_idx = match ia.get(k) {
            Some(&idx) => idx,
            None => continue,
        };
        let b_idx = match ib.get(k) {
            Some(&idx) => idx,
            None => continue,
        };
        let (m_a, w_a) = match (m.get(a_idx), w.get(a_idx)) {
            (Some(ma), Some(wa)) => (ma.as_slice(), wa.as_slice()),
            _ => {
                scores.push(0.0);
                continue;
            }
        };
        let (m_b, w_b) = match (m.get(b_idx), w.get(b_idx)) {
            (Some(mb), Some(wb)) => (mb.as_slice(), wb.as_slice()),
            _ => {
                scores.push(0.0);
                continue;
            }
        };
        scores.push(score_single_pair(m_a, w_a, m_b, w_b, tol));
    }
    scores
}
