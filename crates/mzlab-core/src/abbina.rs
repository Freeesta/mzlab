//! Deterministic one-to-one matching of two lists of (m/z, RT) points.
//!
//! Every pair within +-ppm and +-rt_tol is a candidate; pairs are sorted from
//! smallest normalized distance up (ties: lower index of the first list, then of
//! the second); each point is used at most once.

#[derive(Clone, Copy)]
struct Candidate {
    d: f64,
    ia: usize,
    ib: usize,
}

fn collect_candidates(
    mz_a: &[f64],
    rt_a: &[f64],
    mz_b: &[f64],
    rt_b: &[f64],
    ppm: f64,
    rt_tol: f64,
    na: usize,
    nb: usize,
) -> Vec<Candidate> {
    let mut order_b: Vec<usize> = (0..nb).collect();
    crate::sortc::by(&mut order_b, |&i, &j| match (mz_b.get(i), mz_b.get(j)) {
        (Some(a), Some(b)) => a.total_cmp(b),
        _ => std::cmp::Ordering::Equal,
    });

    let mut candidates: Vec<Candidate> = Vec::new();

    for ia in 0..na {
        let Some(&mza) = mz_a.get(ia) else {
            continue;
        };
        let Some(&rta) = rt_a.get(ia) else {
            continue;
        };
        if !mza.is_finite() || mza <= 0.0 || !rta.is_finite() {
            continue;
        }

        let lo = mza * (1.0 - ppm * 1e-6);
        let hi = mza * (1.0 + ppm * 1e-6);

        let lo_idx = order_b.partition_point(|&ib| mz_b.get(ib).is_some_and(|&mzb| mzb < lo));
        let hi_idx = order_b.partition_point(|&ib| mz_b.get(ib).is_some_and(|&mzb| mzb <= hi));

        if let Some(slice) = order_b.get(lo_idx..hi_idx) {
            for &ib in slice {
                let Some(&mzb) = mz_b.get(ib) else {
                    continue;
                };
                let Some(&rtb) = rt_b.get(ib) else {
                    continue;
                };
                if (rtb - rta).abs() <= rt_tol {
                    let d_mz = (mzb - mza) / mza * 1e6 / ppm;
                    let d_rt = (rtb - rta) / rt_tol;
                    let d = d_mz.hypot(d_rt);
                    candidates.push(Candidate { d, ia, ib });
                }
            }
        }
    }

    candidates
}

fn match_greedy(mut candidates: Vec<Candidate>, na: usize, nb: usize) -> (Vec<usize>, Vec<usize>) {
    crate::sortc::by(&mut candidates, |a, b| {
        a.d.total_cmp(&b.d)
            .then_with(|| a.ia.cmp(&b.ia))
            .then_with(|| a.ib.cmp(&b.ib))
    });

    let mut used_a = vec![false; na];
    let mut used_b = vec![false; nb];
    let mut accepted: Vec<(usize, usize)> = Vec::new();

    for c in candidates {
        let Some(ua) = used_a.get_mut(c.ia) else {
            continue;
        };
        if *ua {
            continue;
        }
        let Some(ub) = used_b.get_mut(c.ib) else {
            continue;
        };
        if *ub {
            continue;
        }
        *ua = true;
        *ub = true;
        accepted.push((c.ia, c.ib));
    }

    crate::sortc::by(&mut accepted, |a, b| a.0.cmp(&b.0));

    let mut out_a = Vec::with_capacity(accepted.len());
    let mut out_b = Vec::with_capacity(accepted.len());
    for (ia, ib) in accepted {
        out_a.push(ia);
        out_b.push(ib);
    }

    (out_a, out_b)
}

/// Matches two lists of (m/z, RT) points with a deterministic 1-to-1 bipartite assignment.
///
/// Returns `(out_a, out_b)` where `out_a` and `out_b` contain the indices of matched
/// points, sorted by `out_a` ascending.
pub fn abbina(
    mz_a: &[f64],
    rt_a: &[f64],
    mz_b: &[f64],
    rt_b: &[f64],
    ppm: f64,
    rt_tol: f64,
) -> (Vec<usize>, Vec<usize>) {
    let na = mz_a.len().min(rt_a.len());
    let nb = mz_b.len().min(rt_b.len());
    if na == 0 || nb == 0 || !ppm.is_finite() || !rt_tol.is_finite() || ppm <= 0.0 || rt_tol <= 0.0
    {
        return (Vec::new(), Vec::new());
    }

    let candidates = collect_candidates(mz_a, rt_a, mz_b, rt_b, ppm, rt_tol, na, nb);
    match_greedy(candidates, na, nb)
}
