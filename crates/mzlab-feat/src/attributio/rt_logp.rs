// Ported from Attributio diagnostics/rt_logp.py.
//! The isotonic fit of RT against the XLogP of the CSI:FingerID structures of a campaign (pool-adjacent-violators), with the robust z-score of the residuals:
//! the reference against which the check `xlogp_rt` judges the retention time of an identity.

/// Z-score above which a retention time is an outlier of the fit.
pub const Z_FLAG: f64 = 3.5;
/// Minimum number of valid candidates per polarity to trust the fit.
pub const MIN_N: usize = 15;

/// A Level 3 candidate whose level comes from CSI:FingerID, with its XLogP and RT.
#[derive(Clone, Debug, PartialEq)]
pub struct FitInput {
    pub pol: String,
    pub id: i64,
    pub xlogp: Option<f64>,
    pub rt: Option<f64>,
}

/// A candidate with the outcome of the fit of its polarity.
#[derive(Clone, Debug, PartialEq)]
pub struct Fitted {
    pub pol: String,
    pub id: i64,
    pub xlogp: Option<f64>,
    pub rt: Option<f64>,
    pub rt_pred: Option<f64>,
    pub resid: Option<f64>,
    pub z: Option<f64>,
    pub outlier: bool,
    pub note: String,
}

/// Median (the mean of the two middle values for an even count); NaN for none.
pub fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

/// Non-decreasing monotonic fit y ~ x (the expected y of each point, in the input order).
pub fn isotonic_fit(x: &[f64], y: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..x.len()).collect();
    order.sort_by(|&a, &b| x[a].total_cmp(&x[b]));
    let mut vals: Vec<f64> = Vec::new();
    let mut weights: Vec<f64> = Vec::new();
    for &i in &order {
        vals.push(y[i]);
        weights.push(1.0);
        while vals.len() > 1 && vals[vals.len() - 2] > vals[vals.len() - 1] + 1e-12 {
            let n = vals.len();
            let w = weights[n - 2] + weights[n - 1];
            let v = (vals[n - 2] * weights[n - 2] + vals[n - 1] * weights[n - 1]) / w;
            vals.truncate(n - 2);
            weights.truncate(n - 2);
            vals.push(v);
            weights.push(w);
        }
    }
    let mut fitted = vec![0.0; x.len()];
    let mut k = 0;
    for (v, w) in vals.iter().zip(&weights) {
        for _ in 0..(*w as usize) {
            fitted[order[k]] = *v;
            k += 1;
        }
    }
    fitted
}

/// MAD-based robust z-score (Iglewicz and Hoaglin): 0.6745 (x - median) / MAD; zeros when the MAD is 0.
pub fn robust_z(resid: &[f64]) -> Vec<f64> {
    let med = median(resid);
    let mad = median(&resid.iter().map(|r| (r - med).abs()).collect::<Vec<_>>());
    if mad == 0.0 {
        return vec![0.0; resid.len()];
    }
    resid.iter().map(|r| 0.6745 * (r - med) / mad).collect()
}

fn fit_polarity(group: &[&FitInput]) -> Vec<Fitted> {
    let valid: Vec<usize> = (0..group.len())
        .filter(|&i| group[i].xlogp.is_some() && group[i].rt.is_some())
        .collect();
    let base = |g: &FitInput, note: String| Fitted {
        pol: g.pol.clone(),
        id: g.id,
        xlogp: g.xlogp,
        rt: g.rt,
        rt_pred: None,
        resid: None,
        z: None,
        outlier: false,
        note,
    };
    if valid.len() < MIN_N {
        let note = format!(
            "batch too small ({} < {MIN_N}), fit unreliable",
            valid.len()
        );
        return group.iter().map(|g| base(g, note.clone())).collect();
    }
    let x: Vec<f64> = valid.iter().filter_map(|&i| group[i].xlogp).collect();
    let y: Vec<f64> = valid.iter().filter_map(|&i| group[i].rt).collect();
    let fitted = isotonic_fit(&x, &y);
    let resid: Vec<f64> = y.iter().zip(&fitted).map(|(a, b)| a - b).collect();
    let z = robust_z(&resid);
    let mut out: Vec<Fitted> = group
        .iter()
        .map(|g| base(g, "missing xlogp or RT, not evaluable".into()))
        .collect();
    for (k, &i) in valid.iter().enumerate() {
        let o = &mut out[i];
        o.rt_pred = Some(fitted[k]);
        o.resid = Some(resid[k]);
        o.z = Some(z[k]);
        o.outlier = z[k].abs() > Z_FLAG;
        o.note = if o.outlier {
            "OUTLIER: observed RT inconsistent with xlogp relative to remainder of batch"
        } else {
            "consistent with batch RT-logP trend"
        }
        .into();
    }
    out
}

/// For each polarity (in sorted order): the isotonic fit RT ~ XLogP on the valid candidates.
pub fn build_batch_fit(rows: &[FitInput]) -> Vec<Fitted> {
    let mut pols: Vec<&str> = rows.iter().map(|r| r.pol.as_str()).collect();
    pols.sort_unstable();
    pols.dedup();
    pols.into_iter()
        .flat_map(|p| fit_polarity(&rows.iter().filter(|r| r.pol == p).collect::<Vec<_>>()))
        .collect()
}

/// `(RT expected by the batch fit for this XLogP, robust z of the observed RT)`: the expected RT is the fit at the nearest XLogP of the batch (same
/// polarity); the z uses the median and MAD of the batch residuals. `None` when the batch is too small.
pub fn rt_vs_batch(
    fitted: &[Fitted],
    pol: &str,
    xlogp: f64,
    rt: Option<f64>,
) -> (Option<f64>, Option<f64>) {
    let bf: Vec<&Fitted> = fitted.iter().filter(|f| f.pol == pol).collect();
    if bf.iter().filter(|f| f.xlogp.is_some()).count() < MIN_N || xlogp.is_nan() {
        return (None, None);
    }
    let bfx: Vec<&Fitted> = bf
        .into_iter()
        .filter(|f| f.xlogp.is_some() && f.rt_pred.is_some())
        .collect();
    let mut best: Option<(&Fitted, f64)> = None;
    for f in &bfx {
        let d = (f.xlogp.unwrap_or(f64::NAN) - xlogp).abs();
        if best.is_none_or(|b| d < b.1) {
            best = Some((f, d));
        }
    }
    let Some((near, _)) = best else {
        return (None, None);
    };
    let rt_pred = near.rt_pred;
    let resid = rt.zip(rt_pred).map(|(r, p)| r - p);
    let resids: Vec<f64> = bfx.iter().filter_map(|f| f.resid).collect();
    let med = median(&resids);
    let mad = median(&resids.iter().map(|r| (r - med).abs()).collect::<Vec<_>>());
    let z = match resid {
        Some(r) if mad != 0.0 && !mad.is_nan() => Some(0.6745 * (r - med) / mad),
        _ => None,
    };
    (rt_pred, z)
}
