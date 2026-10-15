// Ported from Attributio annotate.py.
//! The small formats of the explanations: a number or a dash, and `%g`.

/// A number with `nd` decimals, or `-` when there is none.
pub fn num_txt(v: Option<f64>, nd: usize) -> String {
    match v {
        Some(x) if !x.is_nan() => format!("{x:.nd$}"),
        _ => "-".into(),
    }
}

/// Python's `{:g}`: six significant digits, no trailing zeros.
pub fn fmt_g(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return if v == 0.0 { "0".into() } else { format!("{v}") };
    }
    let exp = v.abs().log10().floor() as i32;
    let sci = format!("{:.5e}", v);
    // the exponent after rounding to six digits decides the notation
    let (mant, e) = sci.split_once('e').unwrap_or((&sci, "0"));
    let e: i32 = e.parse().unwrap_or(exp);
    let trim = |s: &str| {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if !(-4..6).contains(&e) {
        let sign = if e < 0 { '-' } else { '+' };
        format!("{}e{}{:02}", trim(mant), sign, e.abs())
    } else {
        trim(&format!("{:.*}", (5 - e).max(0) as usize, v))
    }
}
