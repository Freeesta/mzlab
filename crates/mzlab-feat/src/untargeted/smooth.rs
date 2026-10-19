//! Smoothing of the intensities of a run before the search of maxima and valleys (feature `untargeted`).
//!
//! After the smoothers of mzmine 4.10.90 (Gaussian and Savitzky-Golay, quadratic; MIT, (c) 2004-2022 The mzmine Development Team); no code copied. The
//! smoothed series serves ONLY the maxima and valleys: centroid, height and area are always taken from the raw data. Used by the untargeted stages.
//! Each consecutive run `[min, max)` is smoothed on its own into a new array (zeros elsewhere); the window is TRUNCATED at the ends of the run, with
//! no padding and no renormalisation of the weights after the cut.

/// Smoothing method; `Gaussian` is the default of mzmine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    None,
    Gaussian,
    SavitzkyGolay,
}

/// Window of the smoother: at least 3 and odd (an even value becomes the next odd one).
pub fn window_size(w: usize) -> usize {
    let w = w.max(3);
    if w.is_multiple_of(2) {
        w + 1
    } else {
        w
    }
}

/// Weights of the whole window, offsets `-m..=m`, summing to 1: Gaussian with sigma `w / 6`, or the quadratic Savitzky-Golay `(3m(m+1) - 1 - 5k^2) / sum`.
pub fn weights(method: Method, window: usize) -> Vec<f64> {
    let w = window_size(window);
    let m = ((w - 1) / 2) as i64;
    let raw: Vec<f64> = (-m..=m)
        .map(|k| match method {
            Method::SavitzkyGolay => (3 * m * (m + 1) - 1 - 5 * k * k) as f64,
            _ => {
                let sigma = w as f64 / 6.0;
                (-((k * k) as f64) / (2.0 * sigma * sigma)).exp()
            }
        })
        .collect();
    let sum: f64 = raw.iter().sum();
    raw.iter().map(|v| v / sum).collect()
}

/// Splits `mz` into runs of consecutive points: a new run starts where the m/z jump is at least `max_diff`. Returns `[start, end)` pairs.
pub fn runs(mz: &[f64], max_diff: f64) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    for i in 1..=mz.len() {
        if i == mz.len() || mz[i] - mz[i - 1] >= max_diff {
            if i > start {
                out.push((start, i));
            }
            start = i;
        }
    }
    out
}

/// Smooths the run `[min, max)` of `y` into a new array of the same length (zeros outside the run). Gaussian results are not clamped; the
/// Savitzky-Golay result is `>= 0`.
pub fn smooth_run(y: &[f64], min: usize, max: usize, method: Method, window: usize) -> Vec<f64> {
    let mut out = vec![0.0; y.len()];
    if method == Method::None {
        out[min..max].copy_from_slice(&y[min..max]);
        return out;
    }
    let wts = weights(method, window);
    let m = (wts.len() / 2) as i64;
    for (i, slot) in out.iter_mut().enumerate().take(max).skip(min) {
        let mut acc = 0.0;
        for (j, wt) in wts.iter().enumerate() {
            let p = i as i64 + j as i64 - m;
            if p >= min as i64 && p < max as i64 {
                acc += wt * y[p as usize];
            }
        }
        *slot = if method == Method::SavitzkyGolay {
            acc.max(0.0)
        } else {
            acc
        };
    }
    out
}

/// Smooths every run of `mz` (see [`runs`]) of the series `y`.
pub fn smooth(mz: &[f64], y: &[f64], max_diff: f64, method: Method, window: usize) -> Vec<f64> {
    let mut out = vec![0.0; y.len()];
    for (s, e) in runs(mz, max_diff) {
        out[s..e].copy_from_slice(&smooth_run(y, s, e, method, window)[s..e]);
    }
    out
}
