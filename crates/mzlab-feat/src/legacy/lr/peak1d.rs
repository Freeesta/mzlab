//! `peaks.find_peak` of mzFinder for one trace: smoothing, the peak around a time (or the highest one), noise
//! from the far points, edges, points across the peak, area (linear baseline removed, counts x s), FWHM (s) and S/N. Used for the MRM traces.

/// The peak of a trace.
#[derive(Debug, Clone, Copy)]
pub struct Found {
    pub apex_rt: f64,
    pub height: f64,
    pub area: f64,
    pub fwhm: f64,
    pub snr: f64,
    pub points: usize,
    pub left: f64,
    pub right: f64,
    pub ok: bool,
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    mzlab_core::sortc::by(&mut s, |a, b| a.total_cmp(b));
    match s.len() {
        0 => 0.0,
        n if n % 2 == 1 => s[n / 2],
        n => (s[n / 2 - 1] + s[n / 2]) / 2.0,
    }
}

/// 1-2-1 (k = 3) or 1-4-6-4-1 (k = 5) smoothing; the edges keep the weight of the points they have.
pub fn smooth(y: &[f64], k: usize) -> Vec<f64> {
    if k <= 1 || y.len() < k {
        return y.to_vec();
    }
    let w: &[f64] = if k == 3 {
        &[1.0, 2.0, 1.0]
    } else {
        &[1.0, 4.0, 6.0, 4.0, 1.0]
    };
    let h = (w.len() / 2) as isize;
    (0..y.len() as isize)
        .map(|i| {
            let (mut num, mut den) = (0.0, 0.0);
            for (j, wj) in w.iter().enumerate() {
                let p = i + j as isize - h;
                if p >= 0 && (p as usize) < y.len() {
                    num += wj * y[p as usize];
                    den += wj;
                }
            }
            num / den
        })
        .collect()
}

/// Baseline and noise of the smoothed trace: median and 1.4826 MAD of the points far from the apex (all of them when fewer than 10).
fn noise_of(rt: &[f64], ys: &[f64], ia: usize, rt_tol: f64) -> (f64, f64) {
    let far: Vec<f64> = (0..rt.len())
        .filter(|&i| (rt[i] - rt[ia]).abs() > rt_tol.max(0.3))
        .map(|i| ys[i])
        .collect();
    let refv = if far.len() >= 10 { far } else { ys.to_vec() };
    let base = median(&refv);
    let mad = median(&refv.iter().map(|v| (v - base).abs()).collect::<Vec<_>>());
    let mut noise = 1.4826 * mad;
    if noise == 0.0 {
        let m = refv.iter().sum::<f64>() / refv.len() as f64;
        noise = (refv.iter().map(|v| (v - m).powi(2)).sum::<f64>() / refv.len() as f64).sqrt();
    }
    (base, if noise == 0.0 { 1.0 } else { noise })
}

/// Edges of the peak: walk down until the signal is under `thr` or goes up again (ripples in the upper half are ignored).
fn edges(ys: &[f64], ia: usize, thr: f64, top: f64, noise: f64) -> (usize, usize) {
    let (mut l, mut r) = (ia, ia);
    while l > 0 && ys[l - 1] > thr && (ys[l - 1] <= ys[l] * 1.05 + noise || ys[l] > top) {
        l -= 1;
    }
    while r + 1 < ys.len() && ys[r + 1] > thr && (ys[r + 1] <= ys[r] * 1.05 + noise || ys[r] > top)
    {
        r += 1;
    }
    (l, r)
}

/// Contiguous raw scans around the apex above `lim`.
fn points_across(y: &[f64], ia: usize, lim: f64) -> usize {
    let lo = ia.saturating_sub(1);
    let hi = (ia + 1).min(y.len() - 1);
    let mut best = lo;
    for i in lo..=hi {
        if y[i] > y[best] {
            best = i;
        }
    }
    let (mut a, mut b) = (best, best);
    while a > 0 && y[a - 1] >= lim {
        a -= 1;
    }
    while b + 1 < y.len() && y[b + 1] >= lim {
        b += 1;
    }
    b - a + 1
}

/// Area (counts x s) of `y` over `l..=r` above the straight line between its ends.
fn area_of(rt: &[f64], y: &[f64], l: usize, r: usize) -> f64 {
    if r <= l {
        return 0.0;
    }
    let span = (rt[r] - rt[l]).max(1e-9);
    let f = |i: usize| (y[i] - (y[l] + (y[r] - y[l]) * (rt[i] - rt[l]) / span)).max(0.0);
    (l..r)
        .map(|i| 0.5 * (f(i) + f(i + 1)) * (rt[i + 1] - rt[i]))
        .sum::<f64>()
        * 60.0
}

/// `find_peak(rt, y, rt_center, rt_tol, smooth_k, min_points)`; `None` for an empty trace or no point in the window.
pub fn find_peak(
    rt: &[f64],
    y: &[f64],
    center: Option<f64>,
    rt_tol: f64,
    smooth_k: usize,
    min_points: usize,
) -> Option<Found> {
    let ymax = y.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if rt.len() < 3 || rt.len() != y.len() || ymax <= 0.0 {
        return None;
    }
    let ys = smooth(y, smooth_k);
    let (lo, hi) = center.map_or((rt[0], rt[rt.len() - 1]), |c| (c - rt_tol, c + rt_tol));
    let ia = (0..rt.len()).filter(|&i| rt[i] >= lo && rt[i] <= hi).fold(
        None::<usize>,
        |b, i| match b {
            Some(j) if ys[j] >= ys[i] => Some(j),
            _ => Some(i),
        },
    )?;
    if ys[ia] <= 0.0 {
        return None;
    }
    let (base, noise) = noise_of(rt, &ys, ia, rt_tol);
    let height = ys[ia] - base;
    let thr = base + (0.01 * height).max(noise);
    let (l, r) = edges(&ys, ia, thr, base + 0.5 * height, noise);
    let points = points_across(y, ia, base + 0.05 * height);
    let half = base + height / 2.0;
    let above: Vec<usize> = (l..=r).filter(|&i| ys[i] >= half).collect();
    let fwhm = match (above.first(), above.last()) {
        (Some(&a), Some(&b)) if above.len() >= 2 => (rt[b] - rt[a]) * 60.0,
        _ => 0.0,
    };
    Some(Found {
        apex_rt: rt[ia],
        height,
        area: area_of(rt, y, l, r),
        fwhm,
        snr: height / noise,
        points,
        left: rt[l],
        right: rt[r],
        ok: points >= min_points,
    })
}
