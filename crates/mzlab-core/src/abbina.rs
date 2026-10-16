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

/// Input parameters for deterministic bipartite m/z-RT matching.
#[derive(Clone, Copy)]
pub struct AbbinaParams<'a> {
    pub mz_a: &'a [f64],
    pub rt_a: &'a [f64],
    pub mz_b: &'a [f64],
    pub rt_b: &'a [f64],
    pub ppm: f64,
    pub rt_tol: f64,
}

fn collect_candidates(p: &AbbinaParams<'_>, na: usize, nb: usize) -> Vec<Candidate> {
    let mut order_b: Vec<usize> = (0..nb).collect();
    crate::sortc::by(&mut order_b, |&i, &j| match (p.mz_b.get(i), p.mz_b.get(j)) {
        (Some(a), Some(b)) => a.total_cmp(b),
        _ => std::cmp::Ordering::Equal,
    });

    let mut candidates: Vec<Candidate> = Vec::new();

    for ia in 0..na {
        let Some(&mza) = p.mz_a.get(ia) else {
            continue;
        };
        let Some(&rta) = p.rt_a.get(ia) else {
            continue;
        };
        if !mza.is_finite() || mza <= 0.0 || !rta.is_finite() {
            continue;
        }

        let lo = mza * (1.0 - p.ppm * 1e-6);
        let hi = mza * (1.0 + p.ppm * 1e-6);

        let lo_idx = order_b.partition_point(|&ib| p.mz_b.get(ib).is_some_and(|&mzb| mzb < lo));
        let hi_idx = order_b.partition_point(|&ib| p.mz_b.get(ib).is_some_and(|&mzb| mzb <= hi));

        if let Some(slice) = order_b.get(lo_idx..hi_idx) {
            for &ib in slice {
                let Some(&mzb) = p.mz_b.get(ib) else {
                    continue;
                };
                let Some(&rtb) = p.rt_b.get(ib) else {
                    continue;
                };
                if (rtb - rta).abs() <= p.rt_tol {
                    let d_mz = (mzb - mza) / mza * 1e6 / p.ppm;
                    let d_rt = (rtb - rta) / p.rt_tol;
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
/// Accepts parameters via [`AbbinaParams`]. Returns `(out_a, out_b)`.
pub fn abbina_params(p: &AbbinaParams<'_>) -> (Vec<usize>, Vec<usize>) {
    let na = p.mz_a.len().min(p.rt_a.len());
    let nb = p.mz_b.len().min(p.rt_b.len());
    if na == 0
        || nb == 0
        || !p.ppm.is_finite()
        || !p.rt_tol.is_finite()
        || p.ppm <= 0.0
        || p.rt_tol <= 0.0
    {
        return (Vec::new(), Vec::new());
    }

    let candidates = collect_candidates(p, na, nb);
    match_greedy(candidates, na, nb)
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
    abbina_params(&AbbinaParams {
        mz_a,
        rt_a,
        mz_b,
        rt_b,
        ppm,
        rt_tol,
    })
}
