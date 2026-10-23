//! Python's `round(v, nd)` for the numbers written in JSON and in the binary traces; `json::round` re-exports it.

/// Python's `round(v, nd)`: the double nearest to the correctly rounded decimal of the exact binary value.
pub fn round(v: f64, nd: usize) -> f64 {
    if !v.is_finite() {
        return v;
    }
    if let Some(r) = round_fast(v, nd) {
        return r;
    }
    format!("{v:.nd$}").parse().unwrap_or(v)
}

/// `round` without the text: `v * 10^nd` is off by at most one rounding (2^-53 relative), so unless its fraction is within `s * 2.5e-16` of one half
/// the direction is certain; the integer over the exact power of ten is the double nearest to the decimal, which is what the text parses to.
fn round_fast(v: f64, nd: usize) -> Option<f64> {
    const P10: [f64; 16] = [
        1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15,
    ];
    let p = *P10.get(nd)?;
    let s = v.abs() * p;
    if s >= 4.0e15 {
        return None;
    }
    let f = s.floor();
    let d = s - f;
    if (d - 0.5).abs() <= s * 2.5e-16 {
        return None;
    }
    let up = if d > 0.5 { f + 1.0 } else { f };
    Some((up / p).copysign(v))
}

#[cfg(test)]
mod tests {
    use super::round;

    /// The fast path gives the very double of the text path, halves and their neighbours included (`MZLAB_ROUND_N` = number of values, default 2 million).
    #[test]
    fn fast_rounding_is_the_text_rounding() {
        let n: u64 = std::env::var("MZLAB_ROUND_N").ok().and_then(|v| v.parse().ok()).unwrap_or(2_000_000);
        let slow = |v: f64, nd: usize| format!("{v:.nd$}").parse::<f64>().unwrap_or(v);
        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for i in 0..n {
            let nd = if i % 2 == 0 { 4 } else { 1 };
            let scale = 10f64.powi((next() % 12) as i32 - 3);
            let u = (next() >> 11) as f64 / (1u64 << 53) as f64;
            let mut v = u * scale;
            if i % 3 == 0 {
                // a half of the last decimal, and the doubles around it
                let k = (v * 10f64.powi(nd as i32)).floor();
                v = (k + 0.5) / 10f64.powi(nd as i32);
                for _ in 0..(next() % 3) {
                    v = f64::from_bits(v.to_bits() + 1);
                }
            }
            if i % 5 == 0 {
                v = -v;
            }
            assert_eq!(round(v, nd).to_bits(), slow(v, nd).to_bits(), "{v:e} at {nd}");
        }
        assert_eq!(round(-0.00001, 4).to_bits(), (-0.0f64).to_bits());
        assert_eq!(round(0.0, 1).to_bits(), 0.0f64.to_bits());
    }
}
