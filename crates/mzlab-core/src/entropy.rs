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
    // b sorted (the callers sort by m/z): only the run of b within tol of a[i] is looked at; same pairs in the same order as the full scan
    let sorted = b_mz.windows(2).all(|w| w[0] <= w[1]);
    for i in 0..a_mz.len() {
        let x = a_mz[i];
        let from = if sorted {
            b_mz.partition_point(|&y| x - y > tol)
        } else {
            0
        };
        for j in from..b_mz.len() {
            if (x - b_mz[j]).abs() <= tol {
                cand.push((i, j, a_it[i] * b_it[j]));
            } else if sorted && b_mz[j] > x {
                break;
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

#[cfg(test)]
mod tests {
    /// `matches` as the plain double loop over every (i, j).
    fn reference(a: &[f64], ai: &[f64], b: &[f64], bi: &[f64], tol: f64) -> Vec<(usize, usize)> {
        let mut c: Vec<(usize, usize, f64)> = Vec::new();
        for i in 0..a.len() {
            for j in 0..b.len() {
                if (a[i] - b[j]).abs() <= tol {
                    c.push((i, j, ai[i] * bi[j]));
                }
            }
        }
        crate::sortc::by(&mut c, |x, y| {
            y.2.partial_cmp(&x.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(x.0.cmp(&y.0))
                .then(x.1.cmp(&y.1))
        });
        let (mut ua, mut ub) = (vec![false; a.len()], vec![false; b.len()]);
        let mut out = Vec::new();
        for (i, j, _) in c {
            if !ua[i] && !ub[j] {
                ua[i] = true;
                ub[j] = true;
                out.push((i, j));
            }
        }
        out
    }

    #[test]
    fn the_sorted_search_finds_the_pairs_of_the_double_loop() {
        let mut seed = 7u64;
        let mut r = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 20_000) as f64 / 100.0
        };
        let it = |v: &[f64]| {
            (0..v.len())
                .map(|k| ((k * 7) % 5) as f64 + 1.0)
                .collect::<Vec<_>>()
        };
        for n in [0usize, 1, 5, 60, 300] {
            let mut a: Vec<f64> = (0..n).map(|_| 50.0 + r()).collect();
            let mut b: Vec<f64> = (0..n + 3).map(|_| 50.0 + r()).collect();
            b.extend_from_slice(&a[..n / 2]);
            a.sort_by(f64::total_cmp);
            let unsorted = b.clone();
            b.sort_by(f64::total_cmp);
            for tol in [0.0, 0.01, 0.5, 3.0] {
                for bb in [&b, &unsorted] {
                    let (ai, bi) = (it(&a), it(bb));
                    assert_eq!(
                        super::matches(&a, &ai, bb, &bi, tol),
                        reference(&a, &ai, bb, &bi, tol),
                        "n={n} tol={tol}"
                    );
                }
            }
        }
    }
}
