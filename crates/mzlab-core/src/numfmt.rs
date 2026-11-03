//! The one number system of areas and heights: scientific notation, decimal point, no thousands separator, a signed two-digit exponent (`7.84077E+04`),
//! so a spreadsheet reads it as a number in any locale. CSV writers use `sci(x, 5)`; the page shows `I18N.sciE` (i18n.js), the same form with 3 decimals.

/// `x` in scientific notation with `decimals` digits after the point; "" for a value that is not finite.
pub fn sci(x: f64, decimals: usize) -> String {
    if !x.is_finite() {
        return String::new();
    }
    let s = format!("{:.*E}", decimals, if x == 0.0 { 0.0 } else { x });
    let (m, e) = s.split_once('E').unwrap_or((&s, "0"));
    let e: i32 = e.parse().unwrap_or(0);
    format!("{m}E{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
}

#[cfg(test)]
mod tests {
    use super::sci;

    #[test]
    fn signed_two_digit_exponent() {
        assert_eq!(sci(78407.7, 5), "7.84077E+04");
        assert_eq!(sci(78407.7, 3), "7.841E+04");
        assert_eq!(sci(0.0, 5), "0.00000E+00");
        assert_eq!(sci(-0.0, 2), "0.00E+00");
        assert_eq!(sci(0.00123, 2), "1.23E-03");
        assert_eq!(sci(9.999996e4, 5), "1.00000E+05");
        assert_eq!(sci(3.2e123, 1), "3.2E+123");
        assert_eq!(sci(f64::NAN, 5), "");
    }
}
