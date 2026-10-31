//! Fast natural logarithm for the Flash entropy terms.

/// Natural logarithm to ~1e-13 relative error (range reduction to [√½, √2) and the atanh series), about three times faster than `f64::ln`;
/// subnormal, zero, negative or non-finite arguments use `f64::ln`.
#[inline]
pub(crate) fn fast_ln(x: f64) -> f64 {
    let bits = x.to_bits();
    let ex = ((bits >> 52) & 0x7ff) as i64;
    if ex == 0 || ex == 0x7ff || x < 0.0 {
        return x.ln();
    }
    let mut e = ex - 1023;
    let mut m = f64::from_bits((bits & 0x000F_FFFF_FFFF_FFFF) | (1023u64 << 52));
    if m > std::f64::consts::SQRT_2 {
        m *= 0.5;
        e += 1;
    }
    let t = (m - 1.0) / (m + 1.0);
    let t2 = t * t;
    let poly = 1.0
        + t2 * (1.0 / 3.0
            + t2 * (1.0 / 5.0
                + t2 * (1.0 / 7.0
                    + t2 * (1.0 / 9.0
                        + t2 * (1.0 / 11.0 + t2 * (1.0 / 13.0 + t2 * (1.0 / 15.0)))))));
    e as f64 * std::f64::consts::LN_2 + 2.0 * t * poly
}

#[cfg(test)]
mod tests {
    use super::fast_ln;

    #[test]
    fn fast_ln_matches_std() {
        let mut x = 1e-9f64;
        while x < 1e9 {
            let (a, b) = (fast_ln(x), x.ln());
            assert!((a - b).abs() <= 1e-12 * b.abs().max(1.0), "{x}: {a} vs {b}");
            x *= 1.0137;
        }
        assert!(fast_ln(0.0).is_infinite() && fast_ln(-1.0).is_nan());
    }
}
