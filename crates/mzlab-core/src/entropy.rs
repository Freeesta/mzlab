//! Spectral entropy similarity (Li & Fiehn, Nat. Methods 2021), computed in `f64`; reference: `tools/golden.py`.

fn entropy(p: &[f64]) -> f64 {
    -p.iter()
        .filter(|&&v| v > 0.0)
        .map(|&v| v * v.ln())
        .sum::<f64>()
}

/// Intensities normalised to sum 1; if the entropy is below 3 they are raised to 0.25 + 0.25 S and normalised again.
fn weighted(it: &[f64]) -> Vec<f64> {
    let s: f64 = it.iter().sum();
    let mut p: Vec<f64> = it.iter().map(|v| v / s).collect();
    let big_s = entropy(&p);
    if big_s < 3.0 {
        let w = 0.25 + 0.25 * big_s;
        p = p.iter().map(|v| v.powf(w)).collect();
        let t: f64 = p.iter().sum();
        p = p.iter().map(|v| v / t).collect();
    }
    p
}

/// One-to-one pairs within `tol` (Da), the most intense pairs first (ties: lower i, then lower j).
fn matches(
    a_mz: &[f64],
    a_it: &[f64],
    b_mz: &[f64],
    b_it: &[f64],
    tol: f64,
) -> Vec<(usize, usize)> {
    let mut cand: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..a_mz.len() {
        for j in 0..b_mz.len() {
            if (a_mz[i] - b_mz[j]).abs() <= tol {
                cand.push((i, j, a_it[i] * b_it[j]));
            }
        }
    }
    crate::sortc::by(&mut cand, |x, y| {
        y.2.partial_cmp(&x.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(x.0.cmp(&y.0))
            .then(x.1.cmp(&y.1))
    });
    let (mut ua, mut ub) = (vec![false; a_mz.len()], vec![false; b_mz.len()]);
    let mut out = Vec::new();
    for (i, j, _) in cand {
        if !ua[i] && !ub[j] {
            ua[i] = true;
            ub[j] = true;
            out.push((i, j));
        }
    }
    out
}

/// 1 - (2 S_AB - S_A - S_B) / ln 4, clipped to [0, 1].
pub fn entropy_similarity(a_mz: &[f64], a_it: &[f64], b_mz: &[f64], b_it: &[f64], tol: f64) -> f64 {
    let (pa, pb) = (weighted(a_it), weighted(b_it));
    let pairs = matches(a_mz, a_it, b_mz, b_it, tol);
    let (mut ia, mut ib) = (vec![false; pa.len()], vec![false; pb.len()]);
    let mut m: Vec<f64> = Vec::new();
    for &(i, j) in &pairs {
        m.push((pa[i] + pb[j]) / 2.0);
        ia[i] = true;
        ib[j] = true;
    }
    m.extend(
        pa.iter()
            .enumerate()
            .filter(|(i, _)| !ia[*i])
            .map(|(_, v)| v / 2.0),
    );
    m.extend(
        pb.iter()
            .enumerate()
            .filter(|(j, _)| !ib[*j])
            .map(|(_, v)| v / 2.0),
    );
    let s = 1.0 - (2.0 * entropy(&m) - entropy(&pa) - entropy(&pb)) / 4f64.ln();
    s.clamp(0.0, 1.0)
}
