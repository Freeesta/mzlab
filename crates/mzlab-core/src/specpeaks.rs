//! Peaks of a unit-resolution mass spectrum: the same rules, in the same order of operations, as `mzlab/peaks.py`
//! (`profile_peaks`, `merge_unit`, `pad_zeros`). Parity: `tests/specpeaks_parity.rs`.

pub const XIC_BELOW: f64 = 0.2;
const SG5: [f64; 5] = [
    -3.0 / 35.0,
    12.0 / 35.0,
    17.0 / 35.0,
    12.0 / 35.0,
    -3.0 / 35.0,
];

/// Nominal mass of the unit window that holds m/z.
pub fn nominal(mz: f64) -> i64 {
    (mz + XIC_BELOW).floor() as i64
}

/// Index of the first element of each run of equal nominal masses, and the run (group) of each point.
fn groups(nom: &[i64]) -> (Vec<usize>, Vec<usize>) {
    let mut starts = vec![];
    let mut g = Vec::with_capacity(nom.len());
    for i in 0..nom.len() {
        if i == 0 || nom[i] != nom[i - 1] {
            starts.push(i);
        }
        g.push(starts.len() - 1);
    }
    (starts, g)
}

/// Peaks of a profile spectrum: (m/z, height), one per nominal mass (smoothed, highest real top, centroid above half height).
pub fn profile_peaks(mz: &[f64], y: &[f64], min_rel: f64) -> (Vec<f64>, Vec<f64>) {
    let n = mz.len();
    if n == 0 {
        return (vec![], vec![]);
    }
    if n < 5 {
        let keep: Vec<usize> = (0..n).filter(|&i| y[i] > 0.0).collect();
        return (
            keep.iter().map(|&i| mz[i]).collect(),
            keep.iter().map(|&i| y[i]).collect(),
        );
    }
    // np.convolve(y, kernel, "same") with a symmetric kernel: terms added by growing index of y
    let s: Vec<f64> = (0..n)
        .map(|i| {
            let mut a = 0.0;
            for m in i.saturating_sub(2)..(i + 3).min(n) {
                a += y[m] * SG5[i + 2 - m];
            }
            a.max(0.0)
        })
        .collect();
    let nom: Vec<i64> = mz.iter().map(|&m| nominal(m)).collect();
    let (starts, g) = groups(&nom);
    let ng = starts.len();
    let top: Vec<f64> = (0..n)
        .map(|i| {
            let left = if i > 0 { s[i - 1] } else { f64::NEG_INFINITY };
            let right = if i + 1 < n {
                s[i + 1]
            } else {
                f64::NEG_INFINITY
            };
            if s[i] >= left && s[i] > right && s[i] > 0.0 {
                s[i]
            } else {
                0.0
            }
        })
        .collect();
    let mut gmax = vec![0.0f64; ng];
    for i in 0..n {
        if top[i] > gmax[g[i]] {
            gmax[g[i]] = top[i];
        }
    }
    let mut jg: Vec<(usize, usize)> = vec![]; // (index of the highest top, group)
    let mut seen = vec![false; ng];
    for i in 0..n {
        if top[i] > 0.0 && top[i] == gmax[g[i]] && !seen[g[i]] {
            seen[g[i]] = true;
            jg.push((i, g[i]));
        }
    }
    let smax = s.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let thr = min_rel * smax.max(1e-12);
    jg.retain(|&(j, _)| s[j] >= thr);
    if jg.is_empty() {
        return (vec![], vec![]);
    }
    let mut half = vec![f64::INFINITY; ng];
    for &(j, gj) in &jg {
        half[gj] = 0.5 * s[j];
    }
    let m: Vec<bool> = (0..n).map(|i| s[i] >= half[g[i]]).collect();
    let mut rid = vec![0usize; n];
    for i in 0..n {
        rid[i] = if i == 0 {
            1
        } else {
            rid[i - 1] + (m[i] != m[i - 1]) as usize
        };
    }
    let mut rj = vec![usize::MAX; ng];
    for &(j, gj) in &jg {
        rj[gj] = rid[j];
    }
    let mut sw = vec![0.0f64; ng];
    let mut sm = vec![0.0f64; ng];
    for i in 0..n {
        if m[i] && rid[i] == rj[g[i]] {
            sw[g[i]] += s[i];
            sm[g[i]] += s[i] * mz[i];
        }
    }
    (
        jg.iter()
            .map(|&(_, gj)| sm[gj] / sw[gj].max(1e-12))
            .collect(),
        jg.iter().map(|&(j, _)| s[j]).collect(),
    )
}

/// Centroids of the same unit window become one peak (intensity = sum, m/z = weighted mean).
pub fn merge_unit(mz: &[f64], y: &[f64]) -> (Vec<f64>, Vec<f64>) {
    if mz.len() < 2 {
        return (mz.to_vec(), y.to_vec());
    }
    let mut o: Vec<usize> = (0..mz.len()).collect();
    o.sort_by(|a, b| mz[*a].total_cmp(&mz[*b]));
    let (mzs, ys): (Vec<f64>, Vec<f64>) = o.iter().map(|&i| (mz[i], y[i])).unzip();
    let nom: Vec<i64> = mzs.iter().map(|&m| nominal(m)).collect();
    let (starts, g) = groups(&nom);
    let mut sy = vec![0.0f64; starts.len()];
    let mut sx = vec![0.0f64; starts.len()];
    for i in 0..mzs.len() {
        sy[g[i]] += ys[i];
        sx[g[i]] += mzs[i] * ys[i];
    }
    let smz = sx.iter().zip(&sy).map(|(a, b)| a / b.max(1e-12)).collect();
    (smz, sy)
}

/// Profile points with the zeros that the converter left out put back next to each gap.
pub fn pad_zeros(mz: &[f64], y: &[f64], step: f64) -> (Vec<f64>, Vec<f64>) {
    if mz.len() < 2 || step == 0.0 {
        return (mz.to_vec(), y.to_vec());
    }
    let gap: Vec<usize> = (0..mz.len() - 1)
        .filter(|&i| mz[i + 1] - mz[i] > 1.5 * step)
        .collect();
    if gap.is_empty() {
        return (mz.to_vec(), y.to_vec());
    }
    let mut om = mz.to_vec();
    let mut oy = y.to_vec();
    om.extend(gap.iter().map(|&i| mz[i] + step));
    om.extend(gap.iter().map(|&i| mz[i + 1] - step));
    oy.resize(om.len(), 0.0);
    let mut o: Vec<usize> = (0..om.len()).collect();
    o.sort_by(|a, b| om[*a].total_cmp(&om[*b]));
    (
        o.iter().map(|&i| om[i]).collect(),
        o.iter().map(|&i| oy[i]).collect(),
    )
}
