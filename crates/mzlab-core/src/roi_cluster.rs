//! ROI correlation and average-linkage agglomerative clustering.
//!
//! Mirrors `_pairwise_windowed_corr` and `_average_linkage` from `mzlab/ionfamily.py`.

/// Compute the pairwise Pearson correlation matrix over shared peak regions.
///
/// Traces are normalized to their peak maximum:
/// `A[i][k] = traces[i][k] / max(max_k traces[i][k], 1e-300)`.
/// For each pair (i, j), Pearson correlation is calculated over indices where
/// `A[i][k] >= thr || A[j][k] >= thr`. If the count of such indices is less
/// than `min_overlap`, or if variance is non-positive or non-finite, NaN is returned.
pub fn pairwise_windowed_corr(
    traces: &[Vec<f64>],
    thr: f64,
    min_overlap: usize,
) -> Vec<Vec<f64>> {
    let m = traces.len();
    if m == 0 {
        return Vec::new();
    }

    let mut a_mat: Vec<Vec<f64>> = Vec::with_capacity(m);
    for row in traces {
        let mut max_val = f64::NEG_INFINITY;
        for &v in row {
            if v.is_nan() {
                max_val = f64::NAN;
                break;
            }
            if v > max_val {
                max_val = v;
            }
        }
        let denom = if max_val.is_nan() {
            f64::NAN
        } else if max_val > 1e-300 {
            max_val
        } else {
            1e-300
        };
        let a_row: Vec<f64> = row.iter().map(|&v| v / denom).collect();
        a_mat.push(a_row);
    }

    let mut r_mat = vec![vec![f64::NAN; m]; m];
    for i in 0..m {
        for j in i..m {
            let row_i = match a_mat.get(i) {
                Some(r) => r,
                None => continue,
            };
            let row_j = match a_mat.get(j) {
                Some(r) => r,
                None => continue,
            };
            let r = calc_pair_corr(row_i, row_j, thr, min_overlap);
            if let Some(r_row) = r_mat.get_mut(i) {
                if let Some(cell) = r_row.get_mut(j) {
                    *cell = r;
                }
            }
            if let Some(r_row) = r_mat.get_mut(j) {
                if let Some(cell) = r_row.get_mut(i) {
                    *cell = r;
                }
            }
        }
    }
    r_mat
}

fn calc_pair_corr(x: &[f64], y: &[f64], thr: f64, min_overlap: usize) -> f64 {
    let len = x.len().min(y.len());
    let mut cnt = 0usize;
    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    let mut sxy = 0.0;

    for k in 0..len {
        let xi = match x.get(k) {
            Some(&v) => v,
            None => continue,
        };
        let yj = match y.get(k) {
            Some(&v) => v,
            None => continue,
        };
        if xi >= thr || yj >= thr {
            cnt += 1;
            sx += xi;
            sy += yj;
            sxx += xi * xi;
            syy += yj * yj;
            sxy += xi * yj;
        }
    }

    if cnt < min_overlap {
        return f64::NAN;
    }
    let cnt_f = cnt as f64;
    let cov = sxy - sx * sy / cnt_f;
    let vx = sxx - sx * sx / cnt_f;
    let vy = syy - sy * sy / cnt_f;
    let denom_prod = vx * vy;
    if denom_prod <= 0.0 || vx <= 0.0 || vy <= 0.0 || !denom_prod.is_finite() {
        return f64::NAN;
    }
    let r = cov / denom_prod.sqrt();
    if r.is_finite() {
        r
    } else {
        f64::NAN
    }
}

fn find_min_active_pair(d_mat: &[Vec<f64>], active: &[usize]) -> Option<(usize, usize, f64)> {
    let mut best_i = 0;
    let mut best_j = 0;
    let mut best_d = f64::INFINITY;
    let mut found = false;

    for (i, &a) in active.iter().enumerate() {
        let row_a = match d_mat.get(a) {
            Some(r) => r,
            None => continue,
        };
        for (j, &b) in active.iter().enumerate() {
            let d = match row_a.get(b) {
                Some(&v) => v,
                None => f64::INFINITY,
            };
            if d.is_nan() {
                return Some((i, j, d));
            }
            if d < best_d {
                best_d = d;
                best_i = i;
                best_j = j;
                found = true;
            }
        }
    }

    if found {
        Some((best_i, best_j, best_d))
    } else {
        None
    }
}

fn update_linkage_step(
    d_mat: &mut [Vec<f64>],
    members: &mut [Vec<usize>],
    active: &[usize],
    a: usize,
    b: usize,
    m: usize,
) {
    let na = members.get(a).map_or(0, |v| v.len());
    let nb = members.get(b).map_or(0, |v| v.len());
    let total_n = (na + nb) as f64;

    for &k in active {
        if k != a && k != b {
            let dak = d_mat
                .get(a)
                .and_then(|r| r.get(k))
                .copied()
                .unwrap_or(f64::INFINITY);
            let dbk = d_mat
                .get(b)
                .and_then(|r| r.get(k))
                .copied()
                .unwrap_or(f64::INFINITY);
            let new_dist = (dak * na as f64 + dbk * nb as f64) / total_n;
            if let Some(row_a) = d_mat.get_mut(a) {
                if let Some(cell) = row_a.get_mut(k) {
                    *cell = new_dist;
                }
            }
            if let Some(row_k) = d_mat.get_mut(k) {
                if let Some(cell) = row_k.get_mut(a) {
                    *cell = new_dist;
                }
            }
        }
    }

    if let Some(row_a) = d_mat.get_mut(a) {
        if let Some(cell) = row_a.get_mut(a) {
            *cell = f64::INFINITY;
        }
    }

    for k in 0..m {
        if let Some(row_b) = d_mat.get_mut(b) {
            if let Some(cell) = row_b.get_mut(k) {
                *cell = f64::INFINITY;
            }
        }
        if let Some(row_k) = d_mat.get_mut(k) {
            if let Some(cell) = row_k.get_mut(b) {
                *cell = f64::INFINITY;
            }
        }
    }

    let moved_members = if b < members.len() {
        std::mem::take(&mut members[b])
    } else {
        Vec::new()
    };
    if let Some(mem_a) = members.get_mut(a) {
        mem_a.extend(moved_members);
    }
}

/// Agglomerative clustering with average linkage on a distance matrix.
///
/// Halts when the closest pair distance exceeds `cut` or is non-finite.
/// Returns cluster labels `0..k-1` and chronological merge heights.
pub fn average_linkage(dist: &[Vec<f64>], cut: f64) -> (Vec<usize>, Vec<f64>) {
    let m = dist.len();
    if m == 0 {
        return (Vec::new(), Vec::new());
    }
    if m == 1 {
        return (vec![0], Vec::new());
    }

    let mut d_mat = dist.to_vec();
    for i in 0..m {
        if let Some(row) = d_mat.get_mut(i) {
            if let Some(cell) = row.get_mut(i) {
                *cell = f64::INFINITY;
            }
        }
    }

    let mut members: Vec<Vec<usize>> = (0..m).map(|i| vec![i]).collect();
    let mut active: Vec<usize> = (0..m).collect();
    let mut heights: Vec<f64> = Vec::new();

    while active.len() > 1 {
        let (best_i, best_j, best_d) = match find_min_active_pair(&d_mat, &active) {
            Some(res) => res,
            None => break,
        };
        if !best_d.is_finite() || best_d > cut {
            break;
        }

        let a = match active.get(best_i) {
            Some(&idx) => idx,
            None => break,
        };
        let b = match active.get(best_j) {
            Some(&idx) => idx,
            None => break,
        };

        update_linkage_step(&mut d_mat, &mut members, &active, a, b, m);
        if best_j < active.len() {
            active.remove(best_j);
        }
        heights.push(best_d);
    }

    let mut labels = vec![0usize; m];
    for (c, &a) in active.iter().enumerate() {
        if let Some(mem) = members.get(a) {
            for &idx in mem {
                if let Some(slot) = labels.get_mut(idx) {
                    *slot = c;
                }
            }
        }
    }
    (labels, heights)
}

/// Compute windowed correlations, transform to distance, and cluster ROIs with average linkage.
pub fn cluster_rois(
    traces: &[Vec<f64>],
    thr: f64,
    min_overlap: usize,
    cut: f64,
) -> (Vec<usize>, Vec<f64>) {
    let m = traces.len();
    if m == 0 {
        return (Vec::new(), Vec::new());
    }
    let r = pairwise_windowed_corr(traces, thr, min_overlap);
    let mut dist = vec![vec![1.0; m]; m];
    for (i, row_r) in r.iter().enumerate().take(m) {
        for (j, &val) in row_r.iter().enumerate().take(m) {
            if i == j {
                if let Some(d_row) = dist.get_mut(i) {
                    if let Some(cell) = d_row.get_mut(j) {
                        *cell = 0.0;
                    }
                }
            } else if val.is_finite() {
                if let Some(d_row) = dist.get_mut(i) {
                    if let Some(cell) = d_row.get_mut(j) {
                        *cell = (1.0 - val) / 2.0;
                    }
                }
            }
        }
    }
    average_linkage(&dist, cut)
}
