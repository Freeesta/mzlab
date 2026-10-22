//! Small numeric helpers of `tabcheck/1`: ranking, median, CV, correlation and slope.

/// Sorts (id, key) pairs by key, descending if asked, ties by id; one instance for every ranking of the checks.
pub(super) fn rank(v: &mut [(usize, f64)], desc: bool) {
    crate::sortc::by(v, |a, b| {
        let o = a.1.total_cmp(&b.1);
        (if desc { o.reverse() } else { o }).then(a.0.cmp(&b.0))
    });
}

pub(crate) fn median(mut v: Vec<f64>) -> f64 {
    crate::sortc::by(&mut v, f64::total_cmp);
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

pub(super) fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// Standard deviation over the mean (sample standard deviation, as the `qc_cv_max` rule); NaN with fewer than two values or a zero mean.
pub(super) fn cv(v: &[f64]) -> f64 {
    if v.len() < 2 {
        return f64::NAN;
    }
    let m = mean(v);
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64).sqrt() / m
}

/// Pearson correlation; None for fewer than two points or no variance in either series.
pub(super) fn pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 2 || x.len() != y.len() {
        return None;
    }
    let (mx, my) = (mean(x), mean(y));
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx).powi(2);
        syy += (b - my) * (b - my);
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy / (sxx * syy).sqrt())
}

/// Slope of the least-squares line of `y` on `x`; None without variance in `x`.
pub(super) fn slope(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 2 {
        return None;
    }
    let (mx, my) = (mean(x), mean(y));
    let sxx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum();
    (sxx > 0.0).then(|| {
        x.iter()
            .zip(y)
            .map(|(a, b)| (a - mx) * (b - my))
            .sum::<f64>()
            / sxx
    })
}
