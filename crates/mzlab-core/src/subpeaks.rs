//! Retention times of the apexes of chromatographic sub-peaks (Savitzky-Golay smoothing, local maxima, minimum separation).

fn short_trace(rt: &[f64], y: &[f64], n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    let mut best_i = 0;
    let mut best_v = match y.first() {
        Some(&v) => v,
        None => return Vec::new(),
    };
    for (i, &v) in y[..n].iter().enumerate().skip(1) {
        if v > best_v {
            best_v = v;
            best_i = i;
        }
    }
    match rt.get(best_i) {
        Some(&t) => vec![t],
        None => Vec::new(),
    }
}

/// Retention times of the apexes of the sub-peaks of an chromatographic trace.
///
/// Savitzky-Golay smoothing over `smooth_min` minutes, local maxima above `min_rel` of the
/// highest, and at least `min_sep` minutes apart (higher peak preserved on conflict).
pub fn subpeaks(rt: &[f64], y: &[f64], min_rel: f64, smooth_min: f64, min_sep: f64) -> Vec<f64> {
    let n = y.len().min(rt.len());
    if n < 5 {
        return short_trace(rt, y, n);
    }

    let diff: Vec<f64> = rt[..n].windows(2).map(|w| w[1] - w[0]).collect();
    let dt = crate::signal::median(&diff);

    let raw_w = (smooth_min / dt.max(1e-9)).round() as usize;
    let mut w = 5.max(raw_w | 1);
    let max_odd = n - (1 - n % 2);
    w = w.min(max_odd);

    let ys = if w >= 5 {
        crate::signal::savgol(&y[..n], w, 2)
    } else {
        y[..n].to_vec()
    };

    let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let threshold = min_rel * max_y;

    let mut peaks: Vec<usize> = Vec::new();
    for i in 1..ys.len().saturating_sub(1) {
        if ys[i] >= ys[i - 1] && ys[i] > ys[i + 1] && ys[i] >= threshold {
            peaks.push(i);
        }
    }

    crate::sortc::by(&mut peaks, |&a, &b| {
        ys[b].total_cmp(&ys[a]).then_with(|| a.cmp(&b))
    });

    let mut kept: Vec<usize> = Vec::new();
    for &i in &peaks {
        if kept.iter().all(|&j| (rt[i] - rt[j]).abs() >= min_sep) {
            kept.push(i);
        }
    }

    let mut out: Vec<f64> = kept.iter().map(|&i| rt[i]).collect();
    crate::sortc::by(&mut out, f64::total_cmp);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subpeaks_empty_and_short() {
        assert_eq!(subpeaks(&[], &[], 0.15, 0.03, 0.08), Vec::<f64>::new());
        assert_eq!(subpeaks(&[1.5], &[10.0], 0.15, 0.03, 0.08), vec![1.5]);
        assert_eq!(
            subpeaks(&[1.0, 2.0, 3.0], &[10.0, 50.0, 20.0], 0.15, 0.03, 0.08),
            vec![2.0]
        );
        assert_eq!(
            subpeaks(&[1.0, 2.0, 3.0], &[50.0, 50.0, 20.0], 0.15, 0.03, 0.08),
            vec![1.0]
        );
    }
}
