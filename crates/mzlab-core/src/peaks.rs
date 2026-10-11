//! Peak picking on one trace: a line-by-line port of `ionfamily.detect_peaks` of the Python engine
//! (AsLS/SNIP baseline, Savitzky-Golay, CWT with Ricker wavelets, ridge lines, limits at 5% of the apex).
//! The Python engine is the reference: any difference in the limits is a difference of logic, not of tolerance.

#[derive(Debug, Clone, PartialEq)]
pub struct Peak {
    pub apex_i: usize,
    /// Height of the parabola through the apex (on the smoothed trace).
    pub height: f64,
    /// Area of the baseline-corrected trace between the limits (trapezoid, RT in minutes).
    pub area: f64,
    pub lo: usize,
    pub hi: usize,
    pub n_scans: usize,
    pub snr: f64,
}

pub const MIN_SCANS_RELIABLE: usize = 6;

fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

/// Robust noise sigma from the first differences (MAD).
pub fn noise_level(y: &[f64]) -> f64 {
    if y.len() < 4 {
        return 0.0;
    }
    let d: Vec<f64> = y.windows(2).map(|w| w[1] - w[0]).collect();
    let m = median(&d);
    let dev: Vec<f64> = d.iter().map(|v| (v - m).abs()).collect();
    1.4826 * median(&dev) / 2f64.sqrt()
}

/// Solves A x = b (A square, row-major, n x n) by Gaussian elimination with partial pivoting. None if singular.
fn solve(mut a: Vec<f64>, mut b: Vec<f64>, n: usize) -> Option<Vec<f64>> {
    for c in 0..n {
        let mut p = c;
        for r in c + 1..n {
            if a[r * n + c].abs() > a[p * n + c].abs() {
                p = r;
            }
        }
        if a[p * n + c].abs() < 1e-300 {
            return None;
        }
        if p != c {
            for k in 0..n {
                a.swap(c * n + k, p * n + k);
            }
            b.swap(c, p);
        }
        let d = a[c * n + c];
        for r in c + 1..n {
            let f = a[r * n + c] / d;
            if f != 0.0 {
                for k in c..n {
                    a[r * n + k] -= f * a[c * n + k];
                }
                b[r] -= f * b[c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let mut s = b[r];
        for k in r + 1..n {
            s -= a[r * n + k] * x[k];
        }
        x[r] = s / a[r * n + r];
    }
    Some(x)
}

/// Savitzky-Golay convolution coefficients (odd window), derivative order `deriv`, unit spacing, ordered like `np.convolve` wants.
fn savgol_coeffs(window: usize, poly: usize, deriv: usize) -> Vec<f64> {
    let half = (window / 2) as i64;
    let m = poly + 1;
    // pinv(A)[deriv] = row `deriv` of (A^T A)^-1 A^T, A[i][j] = x_i^j
    let xs: Vec<f64> = (-half..=half).map(|v| v as f64).collect();
    let mut ata = vec![0.0; m * m];
    for j in 0..m {
        for k in 0..m {
            ata[j * m + k] = xs.iter().map(|x| x.powi((j + k) as i32)).sum();
        }
    }
    let mut e = vec![0.0; m];
    e[deriv] = 1.0;
    let w = solve(ata, e, m).unwrap_or_else(|| vec![0.0; m]);
    let fact: f64 = (1..=deriv).map(|v| v as f64).product();
    let mut c: Vec<f64> = xs
        .iter()
        .map(|x| fact * (0..m).map(|j| w[j] * x.powi(j as i32)).sum::<f64>())
        .collect();
    c.reverse();
    c
}

fn reflect(i: i64, n: i64) -> usize {
    // numpy "reflect" padding (the edge is not repeated); valid while |offset| < n
    let mut j = i;
    if j < 0 {
        j = -j;
    }
    if j > n - 1 {
        j = 2 * (n - 1) - j;
    }
    j as usize
}

pub fn savgol(y: &[f64], window: usize, poly: usize) -> Vec<f64> {
    let n = y.len();
    let mut window = window | 1;
    if n < 3 {
        return y.to_vec();
    }
    window = window.min(if n % 2 == 1 { n } else { n - 1 });
    let poly = poly.min(window - 1);
    let half = window / 2;
    let c = savgol_coeffs(window, poly, 0);
    let ni = n as i64;
    (0..n)
        .map(|i| {
            (0..window)
                .map(|k| {
                    let src = i as i64 + k as i64 - half as i64;
                    let v = if n > half {
                        y[reflect(src, ni)]
                    } else {
                        y[src.clamp(0, ni - 1) as usize]
                    };
                    v * c[window - 1 - k]
                })
                .sum()
        })
        .collect()
}

pub(crate) fn snip_baseline(y: &[f64], iters: usize) -> Vec<f64> {
    let n = y.len();
    let mut v: Vec<f64> = y
        .iter()
        .map(|&x| (((x.max(0.0) + 1.0).sqrt() + 1.0).ln() + 1.0).ln())
        .collect();
    let top = iters.min((n / 2).saturating_sub(1).max(1));
    for w in 1..=top {
        let a: Vec<f64> = (0..n)
            .map(|i| {
                let right = if i + w < n { v[i + w] } else { v[n - 1] };
                let left = if i >= w { v[i - w] } else { v[0] };
                (right + left) / 2.0
            })
            .collect();
        for i in 0..n {
            v[i] = v[i].min(a[i]);
        }
    }
    v.iter()
        .map(|&x| ((x.exp() - 1.0).exp() - 1.0).powi(2) - 1.0)
        .collect()
}

fn interp(x: f64, xp: &[f64], fp: &[f64]) -> f64 {
    if x <= xp[0] {
        return fp[0];
    }
    if x >= xp[xp.len() - 1] {
        return fp[fp.len() - 1];
    }
    let j = xp.partition_point(|&v| v <= x) - 1;
    fp[j] + (x - xp[j]) * (fp[j + 1] - fp[j]) / (xp[j + 1] - xp[j])
}

fn asls_baseline(y: &[f64], lam: f64, p: f64, iters: usize, max_n: usize) -> Vec<f64> {
    let n = y.len();
    if n < 5 {
        let m = y.iter().cloned().fold(f64::INFINITY, f64::min);
        return vec![if n == 0 { 0.0 } else { m }; n];
    }
    if n > 20000 {
        return snip_baseline(y, 20);
    }
    let k = n.div_ceil(max_n).max(1);
    if k > 1 {
        let m = n / k;
        let yd: Vec<f64> = (0..m)
            .map(|b| y[b * k..(b + 1) * k].iter().sum::<f64>() / k as f64)
            .collect();
        let xd: Vec<f64> = (0..m).map(|b| (b as f64 + 0.5) * k as f64 - 0.5).collect();
        let zd = asls_baseline(&yd, lam / (k as f64).powi(4), p, iters, usize::MAX / 4);
        return (0..n)
            .map(|i| interp(i as f64, &xd, &zd).min(y[i]))
            .collect();
    }
    // P = lam * D2^T D2 (D2 = second differences)
    let mut pm = vec![0.0; n * n];
    for r in 0..n - 2 {
        let c = [(r, 1.0), (r + 1, -2.0), (r + 2, 1.0)];
        for &(i, vi) in &c {
            for &(j, vj) in &c {
                pm[i * n + j] += lam * vi * vj;
            }
        }
    }
    let mut w = vec![1.0; n];
    let mut z = y.to_vec();
    for _ in 0..iters {
        let mut a = pm.clone();
        for i in 0..n {
            a[i * n + i] += w[i];
        }
        let b: Vec<f64> = (0..n).map(|i| w[i] * y[i]).collect();
        z = match solve(a, b, n) {
            Some(s) => s,
            None => break,
        };
        let w_new: Vec<f64> = (0..n)
            .map(|i| if y[i] > z[i] { p } else { 1.0 - p })
            .collect();
        if w_new == w {
            break;
        }
        w = w_new;
    }
    (0..n).map(|i| z[i].min(y[i])).collect()
}

fn ricker(width: f64, n: usize) -> Vec<f64> {
    let h = (n / 2) as i64;
    (-h..=h)
        .map(|x| {
            let x = x as f64;
            (1.0 - (x / width).powi(2)) * (-0.5 * (x / width).powi(2)).exp() / width.sqrt()
        })
        .collect()
}

fn cwt_ricker(y: &[f64], widths: &[f64]) -> Vec<Vec<f64>> {
    let n = y.len();
    widths
        .iter()
        .map(|&w| {
            let k = ((10.0 * w).min(n as f64 - 1.0) as usize) | 1;
            let wv = ricker(w, k);
            let h = (k / 2) as i64;
            (0..n)
                .map(|i| {
                    // np.convolve(np.pad(y, h, edge), wv, valid): the kernel is symmetric
                    (0..k)
                        .map(|j| {
                            let src = (i as i64 + j as i64 - h).clamp(0, n as i64 - 1) as usize;
                            y[src] * wv[k - 1 - j]
                        })
                        .sum::<f64>()
                })
                .collect()
        })
        .collect()
}

/// Height of the quadratic fit around index `i` (the second moment is not needed here), as `_parabola_apex`.
pub(crate) fn parabola_height(rt: &[f64], y: &[f64], i: usize) -> f64 {
    let n = y.len();
    let half = 2usize;
    let a = i.saturating_sub(half);
    let b = (i + half + 1).min(n);
    if b - a < 3 {
        return y[i];
    }
    let xs: Vec<f64> = rt[a..b].iter().map(|t| t - rt[i]).collect();
    let mut ata = vec![0.0; 9];
    let mut aty = vec![0.0; 3];
    for (x, &yy) in xs.iter().zip(&y[a..b]) {
        for j in 0..3 {
            aty[j] += x.powi(j as i32) * yy;
            for k in 0..3 {
                ata[j * 3 + k] += x.powi((j + k) as i32);
            }
        }
    }
    let coef = match solve(ata, aty, 3) {
        Some(c) => c,
        None => return y[i],
    };
    let (c0, c1, c2) = (coef[0], coef[1], coef[2]);
    if c2 >= 0.0 {
        return y[i];
    }
    let (lo, hi) = (xs[0], xs[xs.len() - 1]);
    let xa = (-c1 / (2.0 * c2)).clamp(lo, hi);
    c0 + c1 * xa + c2 * xa * xa
}

fn trapz(y: &[f64], x: &[f64]) -> f64 {
    (1..y.len())
        .map(|i| (y[i] + y[i - 1]) / 2.0 * (x[i] - x[i - 1]))
        .sum()
}

struct Ridge {
    pos: Vec<usize>,
    val: Vec<f64>,
    scale: Vec<usize>,
    last_scale: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub snr: f64,
    pub min_scans: usize,
    pub frac: f64,
    pub smooth: usize,
    /// true = AsLS baseline (default), false = SNIP.
    pub asls: bool,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            snr: 5.0,
            min_scans: 3,
            frac: 0.05,
            smooth: 5,
            asls: true,
        }
    }
}

/// Peaks of the trace (rt in minutes, y), sorted by decreasing height like the Python engine.
pub fn detect_peaks(rt: &[f64], y: &[f64], params: Params) -> Vec<Peak> {
    let n = y.len();
    if n < 7 || !y.iter().any(|&v| v > 0.0) {
        return Vec::new();
    }
    let base = if params.asls {
        asls_baseline(y, 1e5, 0.01, 10, 160)
    } else {
        snip_baseline(y, 20)
    };
    let yc: Vec<f64> = (0..n).map(|i| y[i] - base[i]).collect();
    let ys: Vec<f64> = savgol(&yc, params.smooth, 2)
        .into_iter()
        .map(|v| v.max(0.0))
        .collect();
    let noise = noise_level(&yc).max(1e-9);
    // widths = unique(round(geomspace(1, max(3, min(n/6, 12)), 7), 2))
    let stop = (n as f64 / 6.0).min(12.0).max(3.0);
    let mut widths: Vec<f64> = (0..7)
        .map(|i| {
            let g = if i == 0 {
                1.0
            } else if i == 6 {
                stop
            } else {
                (stop.ln() * i as f64 / 6.0).exp()
            };
            (g * 100.0).round_ties_even() / 100.0
        })
        .collect();
    widths.dedup();
    let c = cwt_ricker(&ys, &widths);
    let nw = widths.len();
    let peaks_per_scale: Vec<Vec<usize>> = (0..nw)
        .map(|i| {
            (1..n - 1)
                .filter(|&j| c[i][j] > c[i][j - 1] && c[i][j] >= c[i][j + 1] && c[i][j] > 0.0)
                .collect()
        })
        .collect();
    let scale_noise: Vec<f64> = (0..nw)
        .map(|i| {
            let m = median(&c[i]);
            let dev: Vec<f64> = c[i].iter().map(|v| (v - m).abs()).collect();
            (median(&dev) * 1.4826).max(1e-12)
        })
        .collect();
    let mut ridges: Vec<Ridge> = Vec::new();
    for (i, mx) in peaks_per_scale.iter().enumerate() {
        for &pos in mx {
            let mut joined = false;
            for r in ridges.iter_mut() {
                if r.last_scale + 1 == i || r.last_scale + 2 == i {
                    let last = *r.pos.last().unwrap() as f64;
                    if (last - pos as f64).abs() <= (widths[i] / 2.0).max(1.0) {
                        r.pos.push(pos);
                        r.val.push(c[i][pos] / scale_noise[i]);
                        r.scale.push(i);
                        r.last_scale = i;
                        joined = true;
                        break;
                    }
                }
            }
            if !joined {
                ridges.push(Ridge {
                    pos: vec![pos],
                    val: vec![c[i][pos] / scale_noise[i]],
                    scale: vec![i],
                    last_scale: i,
                });
            }
        }
    }
    let mut found: Vec<(usize, f64, f64)> = Vec::new();
    let mut taken: Vec<usize> = Vec::new();
    for r in &ridges {
        if r.pos.len() < 3 {
            continue;
        }
        let mut k = 0;
        for (j, v) in r.val.iter().enumerate() {
            if *v > r.val[k] {
                k = j;
            }
        }
        if r.val[k] < params.snr {
            continue;
        }
        let w = widths[r.scale[k]];
        let c0 = r.pos[k] as f64;
        let a = ((c0 - w).trunc().max(0.0)) as usize;
        let b = (((c0 + w).trunc() + 1.0).max(0.0) as usize).min(n);
        let mut apex = a;
        for j in a..b {
            if ys[j] > ys[apex] {
                apex = j;
            }
        }
        if ys[apex] < params.snr * noise {
            continue;
        }
        if taken
            .iter()
            .any(|&t| (apex as f64 - t as f64).abs() <= (w / 2.0).max(1.0))
        {
            continue;
        }
        taken.push(apex);
        found.push((apex, r.val[k], w));
    }
    found.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut peaks = Vec::new();
    for (apex, rsn, _w) in found {
        let h = ys[apex];
        let mut lo = apex;
        while lo > 0 && ys[lo - 1] > params.frac * h && ys[lo - 1] <= ys[lo] + 1e-12 {
            lo -= 1;
        }
        let mut hi = apex;
        while hi < n - 1 && ys[hi + 1] > params.frac * h && ys[hi + 1] <= ys[hi] + 1e-12 {
            hi += 1;
        }
        let lo2 = lo.saturating_sub(1);
        let hi2 = (hi + 1).min(n - 1);
        let hh = parabola_height(rt, &ys, apex);
        let seg: Vec<f64> = yc[lo2..=hi2].iter().map(|v| v.max(0.0)).collect();
        let area = trapz(&seg, &rt[lo2..=hi2]);
        let ns = hi2 - lo2 + 1;
        if ns < params.min_scans {
            continue;
        }
        peaks.push(Peak {
            apex_i: apex,
            height: hh,
            area,
            lo: lo2,
            hi: hi2,
            n_scans: ns,
            snr: rsn,
        });
    }
    peaks.sort_by(|a, b| {
        b.height
            .partial_cmp(&a.height)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    peaks
}
