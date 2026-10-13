//! The evidence features and the logistic model of the in-source-fragment classifier (`TP_Mine/py/tpmine/isf.py::features`, `predict_logit`,
//! `_sigmoid`; the parameters of `isf_model.MODEL`, fitted offline, are constants here). A missing feature contributes nothing.

use mzlab_core::json::Value;

pub(super) static NIL: Value = Value::Null;

/// `rep.get(k) or {}`: the sub-object, or null (every lookup in null finds nothing).
pub(super) fn sub<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&NIL)
}

/// `v.get(k)` when present and not null (Python's `is not None`).
pub(super) fn nn<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    v.get(k).filter(|x| !x.is_null())
}

/// Python truthiness of a JSON value.
pub(super) fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Num(n)) => *n != 0.0,
        Some(Value::Str(s)) => !s.is_empty(),
        Some(Value::Arr(a)) => !a.is_empty(),
        Some(Value::Obj(o)) => !o.is_empty(),
    }
}

/// `_f(v, default)`: the number if finite, else the default.
pub(super) fn fdef(v: &Value, k: &str, default: f64) -> f64 {
    match v.get(k) {
        Some(Value::Num(n)) if n.is_finite() => *n,
        _ => default,
    }
}

pub(super) fn fnum(v: &Value, k: &str) -> f64 {
    fdef(v, k, f64::NAN)
}

pub const NAMES: [&str; 9] = [
    "pearson_w",
    "apex_z",
    "fwhm_log",
    "cv_ratio",
    "const_logp",
    "const_cv",
    "kin_sp",
    "shift_logp",
    "has_peak",
];
const MEAN: [f64; 9] = [
    0.16949898495551663,
    11.796746280178379,
    0.8955714975123599,
    0.951008195304268,
    -6.137873147094511,
    1.1892062263432295,
    0.6174003422057591,
    -0.8688803565942181,
    0.5746606334841629,
];
const STD: [f64; 9] = [
    0.5860063738057145,
    8.791008437063281,
    0.6154911972295407,
    0.5878489439611116,
    4.733562190175963,
    0.6745498156402884,
    0.544793166607442,
    0.7410907058871926,
    0.4943943666828576,
];
const COEF: [f64; 9] = [
    0.9191146971498082,
    -0.452095409754445,
    -0.3887878306877277,
    -1.0326114269162892,
    -0.13813881116477034,
    -0.631705462026846,
    0.2133616499654602,
    -0.4265070775047157,
    -0.05471843218574272,
];
const INTERCEPT: f64 = -1.7955394859039278;

/// The evidence features of an `origin_report` (NaN = not available), in the order of `NAMES`.
pub fn features(rep: &Value) -> [f64; 9] {
    let (pr, rf, rc, kin) = (
        sub(rep, "profile"),
        sub(rep, "ratio"),
        sub(rep, "ratio_constancy"),
        sub(rep, "kinetics"),
    );
    let has = pr.get("x_has_peak").and_then(|v| v.bool()) == Some(true);
    let (d, s) = (fnum(pr, "apex_diff_s"), fnum(pr, "apex_diff_sigma_s"));
    let apex_z = if has && d.is_finite() && s.is_finite() {
        (d.abs() / s.max(0.5)).min(20.0)
    } else {
        20.0
    };
    let fr = fnum(pr, "fwhm_ratio");
    let fwhm = if fr.is_finite() {
        fr.max(1e-2).ln().abs().min(3.0)
    } else {
        1.5
    };
    let pw = fdef(pr, "pearson_w", fnum(pr, "pearson"));
    let cv = if truthy(rf.get("ok")) {
        fnum(rf, "cv_ratio_mad")
    } else {
        f64::NAN
    };
    let (p_sh, p_c, cvs) = (fnum(pr, "p_shift"), fnum(rc, "p"), fnum(rc, "cv"));
    [
        if pw.is_finite() { pw } else { -0.2 },
        apex_z,
        fwhm,
        if cv.is_finite() { cv.min(3.0) } else { 1.5 },
        if p_c.is_finite() {
            p_c.max(1e-12).log10().max(-12.0)
        } else {
            f64::NAN
        },
        if cvs.is_finite() {
            cvs.min(3.0)
        } else {
            f64::NAN
        },
        fnum(kin, "spearman_parent"),
        if p_sh.is_finite() {
            p_sh.max(1e-3).log10().max(-3.0)
        } else {
            0.0
        },
        if has { 1.0 } else { 0.0 },
    ]
}

/// `numpy.sum` of a short array: pairwise in blocks of eight, like numpy, so the log-odds agree to the last digit.
fn np_sum(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        return a.iter().fold(0.0, |s, v| s + v);
    }
    let body = n - n % 8;
    let mut r = [0.0; 8];
    r.copy_from_slice(&a[..8]);
    for blk in a[8..body].chunks(8) {
        r.iter_mut().zip(blk).for_each(|(x, v)| *x += v);
    }
    let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
    for v in &a[body..] {
        res += v;
    }
    res
}

/// `(log-odds that the ion is inherited from the parent, contribution of every feature)`; a missing feature is zero after standardising.
pub fn predict_logit(x: &[f64; 9]) -> (f64, [f64; 9]) {
    let mut contrib = [0.0; 9];
    for i in 0..9 {
        let z = (x[i] - MEAN[i]) / STD[i];
        contrib[i] = if z.is_nan() {
            0.0
        } else {
            z.clamp(-f64::MAX, f64::MAX) * COEF[i]
        };
    }
    (INTERCEPT + np_sum(&contrib), contrib)
}

pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x.clamp(-30.0, 30.0)).exp())
}
