// Ported from Attributio diagnostics/schymanski_export.py.
//! The text the reference tool writes for numbers and lists: the `repr` of a Python float, `"; ".join` of the distinct non-empty strings, and the values of the
//! campaign file as an f-string shows them.

use crate::attributio::toml::Value;

/// `repr(float)` of Python: the shortest text that reads back the same, `.0` for a whole number, an exponent under 1e-4 and from 1e16.
pub fn py_float(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf" } else { "inf" }.into();
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0.0" } else { "0.0" }.into();
    }
    let sci = format!("{x:e}");
    let (mant, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    if (-4..16).contains(&exp) {
        let s = format!("{x}");
        return if s.contains('.') { s } else { format!("{s}.0") };
    }
    format!(
        "{mant}e{}{:02}",
        if exp < 0 { '-' } else { '+' },
        exp.abs()
    )
}

/// `format(x, ".0f")` for a value that may be missing (`nan` as Python prints it).
pub fn fixed0(x: Option<f64>) -> String {
    x.map_or("nan".into(), |v| format!("{v:.0}"))
}

/// `repr` of an optional float as an f-string shows a numpy value (`nan` when missing).
pub fn float_or_nan(x: Option<f64>) -> String {
    x.map_or("nan".into(), py_float)
}

/// `"; ".join` of the distinct, non-empty, trimmed strings in order of first appearance (`_join`).
pub fn join_distinct<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for x in items {
        let t = x.trim();
        if !t.is_empty() && !seen.contains(&t) {
            seen.push(t);
        }
    }
    seen.join("; ")
}

/// How an f-string shows a value of the campaign file (`None` when it is not there).
pub fn toml_text(v: Option<&Value>) -> String {
    match v {
        None => "None".into(),
        Some(Value::Str(s)) => s.clone(),
        Some(Value::Int(i)) => i.to_string(),
        Some(Value::Float(f)) => py_float(*f),
        Some(Value::Bool(b)) => if *b { "True" } else { "False" }.into(),
        Some(Value::Array(a)) => format!(
            "[{}]",
            a.iter()
                .map(|x| match x {
                    Value::Str(s) => format!("'{s}'"),
                    other => toml_text(Some(other)),
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Some(Value::Table(_)) => "{...}".into(),
    }
}
