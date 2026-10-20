//! Tail probabilities of Student's t and of the F distribution through the regularised incomplete beta function (continued fraction of Lentz).
//! Used by `compare.rs` for the Welch test and the one-way ANOVA; no table, no dependency beyond `libm`.

const EPS: f64 = 1e-15;
const TINY: f64 = 1e-300;

/// Continued fraction of the incomplete beta function (modified Lentz).
fn betacf(a: f64, b: f64, x: f64) -> f64 {
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=400 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

/// Regularised incomplete beta function `I_x(a, b)` for `x` in `[0, 1]`.
pub fn betai(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front = libm::exp(
        libm::lgamma(a + b) - libm::lgamma(a) - libm::lgamma(b)
            + a * libm::log(x)
            + b * libm::log(1.0 - x),
    );
    if x < (a + 1.0) / (a + b + 2.0) {
        front * betacf(a, b, x) / a
    } else {
        1.0 - front * betacf(b, a, 1.0 - x) / b
    }
}

/// Two-sided p-value of Student's t with `df` degrees of freedom (`df` may be fractional, as in the Welch test).
pub fn t_two_sided(t: f64, df: f64) -> f64 {
    if !t.is_finite() || df.is_nan() || df <= 0.0 {
        return f64::NAN;
    }
    betai(df / 2.0, 0.5, df / (df + t * t))
}

/// Upper-tail p-value of the F distribution with `d1` and `d2` degrees of freedom.
pub fn f_upper(f: f64, d1: f64, d2: f64) -> f64 {
    if !f.is_finite() || f < 0.0 || d1.is_nan() || d1 <= 0.0 || d2.is_nan() || d2 <= 0.0 {
        return f64::NAN;
    }
    betai(d2 / 2.0, d1 / 2.0, d2 / (d2 + d1 * f))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference values of scipy.stats (t.sf * 2, f.sf).
    #[test]
    fn tails_match_the_reference_values() {
        assert!((t_two_sided(2.0, 10.0) - 0.073_388_034_734_3).abs() < 1e-9);
        assert!((t_two_sided(2.0, 2.0) - 0.183_503_419_072_274).abs() < 1e-12);
        assert!((f_upper(4.0, 2.0, 12.0) - 0.046_656).abs() < 1e-12);
        assert!((t_two_sided(0.0, 5.0) - 1.0).abs() < 1e-12);
    }
}
