//! Quantitative source-voltage (DP breakdown) ramp analysis (`mzlab/ionfamily.py:1396-1431`).
//!
//! Evaluates in-source fragmentation (ISF) breakdown curves: standards acquired at different
//! declustering potentials (DP, V). The ISF ratio `F / (F + P)` rises with the DP along a sigmoid
//! `y = ymax / (1 + exp(-(DP - DP50) / s))`. A co-eluting chemical interferent has a ratio that
//! does not depend on DP.

/// Fitted parameters of the sigmoid curve `y = ymax / (1 + exp(-(DP - DP50) / s))`.
#[derive(Debug, Clone, PartialEq)]
pub struct SigmoidFit {
    /// Saturation plateau of the ratio.
    pub ymax: f64,
    /// Voltage at half-maximal ratio (V).
    pub dp50: f64,
    /// Sigmoid slope factor (V).
    pub s: f64,
    /// Appearance DP voltage (DP at 10% of ymax: `dp50 - s * ln(9)`).
    pub dp_appearance: f64,
    /// Coefficient of determination `R^2` of the sigmoid fit against valid points.
    pub r2: f64,
}

/// Result of source-voltage ramp analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceRampResult {
    /// Number of valid points with finite ratio.
    pub n: usize,
    /// Input DP voltages, sorted ascending.
    pub dp: Vec<f64>,
    /// Ratio `F / (F + P)` corresponding to each sorted DP voltage.
    pub ratio: Vec<f64>,
    /// Spearman rank correlation between DP and ratio (for `n >= 3`).
    pub spearman: Option<f64>,
    /// Robust Theil-Sen slope per Volt (for `n >= 3`).
    pub slope_per_v: Option<f64>,
    /// Normalized dynamic range: `(ratio_last - ratio_first) / max(mean, 1e-12)`.
    pub dp_dependence: Option<f64>,
    /// Sigmoid grid search fit (for `n >= 4`).
    pub sigmoid: Option<SigmoidFit>,
    /// Diagnostic warnings (`of.warn.dp3`, `of.warn.dp4`).
    pub warnings: Vec<&'static str>,
}

fn linspace(start: f64, stop: f64, num: usize) -> Vec<f64> {
    if num == 0 {
        return Vec::new();
    }
    if num == 1 {
        return vec![start];
    }
    let step = (stop - start) / ((num - 1) as f64);
    let mut res = Vec::with_capacity(num);
    for i in 0..num {
        res.push(start + (i as f64) * step);
    }
    if let Some(last) = res.last_mut() {
        *last = stop;
    }
    res
}

fn geomspace(start: f64, stop: f64, num: usize) -> Vec<f64> {
    if num == 0 {
        return Vec::new();
    }
    if num == 1 {
        return vec![start];
    }
    if start <= 0.0 || stop <= 0.0 {
        return Vec::new();
    }
    let log_start = start.log10();
    let log_stop = stop.log10();
    let step = (log_stop - log_start) / ((num - 1) as f64);
    let mut res = Vec::with_capacity(num);
    for i in 0..num {
        let y = log_start + (i as f64) * step;
        res.push(10.0_f64.powf(y));
    }
    if let Some(first) = res.first_mut() {
        *first = start;
    }
    if let Some(last) = res.last_mut() {
        *last = stop;
    }
    res
}

fn fit_sigmoid(d: &[f64], v: &[f64], v_mean: f64) -> Option<SigmoidFit> {
    let d_min = *d.first()?;
    let d_max = *d.last()?;
    let span = d_max - d_min;
    if !span.is_finite() || span <= 0.0 {
        return None;
    }
    let s_lo = (span / 60.0).max(1e-3);
    let s_hi = span * 2.0;
    if s_hi <= s_lo || s_lo <= 0.0 {
        return None;
    }

    let v_max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let y_lo = v_max.max(1e-6);
    let y_hi = (v_max * 1.5).clamp(1e-6, 1.0);
    let ymax_grid = linspace(y_lo, y_hi, 30);
    let dp50_grid = linspace(d_min, d_max, 60);
    let s_grid = geomspace(s_lo, s_hi, 40);
    if s_grid.is_empty() {
        return None;
    }

    let mut best_sse = f64::INFINITY;
    let mut best_params = None;
    for &ymax in &ymax_grid {
        for &dp50 in &dp50_grid {
            for &s in &s_grid {
                let mut sse = 0.0;
                for (&dk, &vk) in d.iter().zip(v.iter()) {
                    let fit_k = ymax / (1.0 + (-(dk - dp50) / s).exp());
                    let diff = vk - fit_k;
                    sse += diff * diff;
                }
                if sse < best_sse {
                    best_sse = sse;
                    best_params = Some((ymax, dp50, s));
                }
            }
        }
    }

    let (best_ymax, best_dp50, best_s) = best_params?;
    let mut sst = 0.0;
    for &vk in v {
        let diff = vk - v_mean;
        sst += diff * diff;
    }
    let r2 = if sst > 0.0 {
        1.0 - best_sse / sst
    } else {
        f64::NAN
    };
    let dp_appearance = best_dp50 - best_s * 9.0_f64.ln();

    Some(SigmoidFit {
        ymax: best_ymax,
        dp50: best_dp50,
        s: best_s,
        dp_appearance,
        r2,
    })
}

/// Breakdown curves from standards acquired at different declustering potentials (DP, V).
///
/// Returns the relative intensity `F / (F + P)`, Spearman correlation, Theil-Sen slope,
/// DP-dependence dynamic range, and the 3-parameter sigmoid fit if `n >= 4`.
pub fn source_ramp(dp: &[f64], f_int: &[f64], p_int: &[f64]) -> SourceRampResult {
    let n_in = dp.len().min(f_int.len()).min(p_int.len());
    let mut order: Vec<usize> = (0..n_in).collect();
    crate::sortc::by(&mut order, |&i, &j| dp[i].total_cmp(&dp[j]));

    let mut dp_ord = Vec::with_capacity(n_in);
    let mut ratio_ord = Vec::with_capacity(n_in);
    for &idx in &order {
        let d_val = dp.get(idx).copied().unwrap_or(f64::NAN);
        let f = f_int.get(idx).copied().unwrap_or(f64::NAN);
        let p = p_int.get(idx).copied().unwrap_or(f64::NAN);
        let denom = f + p;
        let r = if denom <= 0.0 || denom.is_nan() || f.is_nan() {
            f64::NAN
        } else {
            let val = f / denom;
            if val.is_finite() {
                val
            } else {
                f64::NAN
            }
        };
        dp_ord.push(d_val);
        ratio_ord.push(r);
    }

    let mut d = Vec::new();
    let mut v = Vec::new();
    for (&dpi, &ri) in dp_ord.iter().zip(ratio_ord.iter()) {
        if ri.is_finite() {
            d.push(dpi);
            v.push(ri);
        }
    }
    let n = d.len();
    if n < 3 {
        return SourceRampResult {
            n,
            dp: dp_ord,
            ratio: ratio_ord,
            spearman: None,
            slope_per_v: None,
            dp_dependence: None,
            sigmoid: None,
            warnings: vec!["of.warn.dp3"],
        };
    }

    let spearman = Some(crate::stats::wspearman(&d, &v, None)).filter(|x| x.is_finite());
    let slope_per_v = Some(crate::stats::theil_sen(&d, &v, 80).0).filter(|x| x.is_finite());
    let v_mean = v.iter().sum::<f64>() / (n as f64);
    let v_first = v.first().copied().unwrap_or(0.0);
    let v_last = v.last().copied().unwrap_or(0.0);
    let dp_dependence = Some((v_last - v_first) / v_mean.max(1e-12)).filter(|x| x.is_finite());

    let (sigmoid, warnings) = if n >= 4 {
        (fit_sigmoid(&d, &v, v_mean), Vec::new())
    } else {
        (None, vec!["of.warn.dp4"])
    };

    SourceRampResult {
        n,
        dp: dp_ord,
        ratio: ratio_ord,
        spearman,
        slope_per_v,
        dp_dependence,
        sigmoid,
        warnings,
    }
}
