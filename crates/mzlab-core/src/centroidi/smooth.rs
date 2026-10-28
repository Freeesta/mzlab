//! Savitzky-Golay smoothing, background (25th percentile on blocks) and noise (MAD of the first differences).

use super::Buf;

/// Quadratic Savitzky-Golay coefficients of an odd window `l`, centre last-first symmetric: c_k = 3 (3m^2 + 3m - 1 - 5k^2) / ((2m-1)(2m+1)(2m+3)).
pub(super) fn sg_coef(l: usize) -> Vec<f64> {
    let m = ((l - 1) / 2) as f64;
    let den = (2.0 * m - 1.0) * (2.0 * m + 1.0) * (2.0 * m + 3.0);
    (0..l)
        .map(|i| {
            let k = i as f64 - m;
            3.0 * (3.0 * m * m + 3.0 * m - 1.0 - 5.0 * k * k) / den
        })
        .collect()
}

/// `buf.s` = smoothed `buf.v` (zeros beyond the ends), negatives set to zero (only to look for maxima: areas use the raw values).
pub(super) fn smooth(buf: &mut Buf, l: usize) {
    let c = sg_coef(l);
    let m = (l - 1) / 2;
    let n = buf.v.len();
    buf.s.clear();
    for j in 0..n {
        let mut acc = 0.0;
        for (t, ck) in c.iter().enumerate() {
            let i = j as isize + t as isize - m as isize;
            if i >= 0 && (i as usize) < n {
                acc += ck * buf.v[i as usize];
            }
        }
        buf.s.push(acc.max(0.0));
    }
}

/// Element of rank `floor(q (n-1))` of the sorted values (the 'lower' percentile).
pub(super) fn lower(tmp: &mut [f64], q: f64) -> f64 {
    crate::sortc::by(tmp, f64::total_cmp);
    tmp[((tmp.len() - 1) as f64 * q).floor() as usize]
}

/// `buf.b`: 25th percentile of the smoothed signal within +-5 w of the centre of each block of 2.5 w, linear between the centres. With `mean` (counts:
/// a mostly empty scan has a 25th percentile of zero, which would make every stray count a peak) the mean of the lowest 75 % of the values instead.
pub(super) fn background(buf: &mut Buf, wb: usize, mean: bool) {
    let n = buf.s.len();
    let bs = ((2.5 * wb as f64).round() as usize).max(1);
    let half = 5 * wb;
    let nb = n.div_ceil(bs);
    let mut cen = Vec::with_capacity(nb);
    let mut val = Vec::with_capacity(nb);
    for b in 0..nb {
        let c = (b * bs + bs / 2).min(n - 1);
        let (lo, hi) = (c.saturating_sub(half), (c + half).min(n - 1));
        buf.tmp.clear();
        buf.tmp.extend_from_slice(&buf.s[lo..=hi]);
        cen.push(c);
        let q = lower(&mut buf.tmp, 0.25);
        val.push(if mean {
            let k = ((buf.tmp.len() - 1) as f64 * 0.75).floor() as usize;
            buf.tmp[..=k].iter().sum::<f64>() / (k + 1) as f64
        } else {
            q
        });
    }
    buf.b.clear();
    let mut b = 0;
    for j in 0..n {
        while b + 1 < nb && cen[b + 1] <= j {
            b += 1;
        }
        if j <= cen[0] || b + 1 >= nb {
            buf.b.push(if j <= cen[0] { val[0] } else { val[nb - 1] });
        } else {
            let f = (j - cen[b]) as f64 / (cen[b + 1] - cen[b]) as f64;
            buf.b.push(val[b] + f * (val[b + 1] - val[b]));
        }
    }
}

fn median(tmp: &mut [f64]) -> f64 {
    crate::sortc::by(tmp, f64::total_cmp);
    tmp[(tmp.len() - 1) / 2]
}

/// sigma = 1.4826 MAD / sqrt(2) of the first differences `v[a+1] - v[a]` .. `v[b] - v[b-1]`; `pad` is scratch.
fn mad_sigma(v: &[f64], a: usize, b: usize, pad: &mut Vec<f64>) -> f64 {
    pad.clear();
    pad.extend((a..b).map(|i| v[i + 1] - v[i]));
    let med = median(pad);
    pad.clear();
    pad.extend((a..b).map(|i| (v[i + 1] - v[i] - med).abs()));
    1.4826 * median(pad) / std::f64::consts::SQRT_2
}

/// Noise of a continuous scan: sigma = 1.4826 MAD(first differences) / sqrt(2) per block of 10 w, then the 25th percentile of the blocks (>= 5 values).
pub(super) fn noise(buf: &mut Buf, wb: usize) -> f64 {
    let nd = buf.v.len().saturating_sub(1);
    let mut sig = Vec::new();
    for a in (0..nd).step_by(10 * wb) {
        let b = (a + 10 * wb).min(nd);
        if b - a >= 5 {
            sig.push(mad_sigma(&buf.v, a, b, &mut buf.pad));
        }
    }
    if sig.is_empty() {
        if nd == 0 {
            return 1e-300;
        }
        sig.push(mad_sigma(&buf.v, 0, nd, &mut buf.pad));
    }
    lower(&mut sig, 0.25).max(1e-300)
}

#[cfg(test)]
mod tests;
