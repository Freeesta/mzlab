//! Python's number formats and the texts of the verdict (`TEXT`, the evidence values, `_english`).

use super::isf::{fnum, nn, sub, truthy};
use mzlab_core::json::Value;

pub const LABELS: [&str; 9] = [
    "chromatographic profile (correlation within the peak window)",
    "apex relative to the parent",
    "peak width relative to the parent",
    "constancy of the F/P ratio within the peak",
    "constancy of the F/P ratio across samples",
    "variation of the F/P ratio across samples",
    "kinetics across samples relative to the parent",
    "specificity of the alignment (random shifts)",
    "the ion has a peak of its own",
];

/// `"%.{p}g" % x`.
pub fn pg(x: f64, p: usize) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let e = format!("{:.*e}", p - 1, x);
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    if exp < -4 || exp >= p as i32 {
        let m = if mant.contains('.') {
            mant.trim_end_matches('0').trim_end_matches('.')
        } else {
            mant
        };
        return format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs());
    }
    let s = format!("{:.*}", (p as i32 - 1 - exp).max(0) as usize, x);
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(|v| v.f64())
}

/// The value shown next to the evidence `name`; `None` where Python would raise (a field the report promises but lacks).
pub fn evidence_value(name: usize, feat: &[f64; 9], rep: &Value) -> Option<String> {
    let (pr, rf, rc, kin) = (
        sub(rep, "profile"),
        sub(rep, "ratio"),
        sub(rep, "ratio_constancy"),
        sub(rep, "kinetics"),
    );
    Some(match name {
        0 if feat[0].is_finite() => format!("r = {:.2}", feat[0]),
        0 => "n/a".into(),
        1 if truthy(pr.get("x_has_peak")) && nn(pr, "apex_diff_s").is_some() => {
            format!(
                "\u{394}apex {:+.1} \u{b1} {:.1} s",
                num(pr.get("apex_diff_s"))?,
                num(pr.get("apex_diff_sigma_s"))?
            )
        }
        1 => "no peak of its own".into(),
        2 => nn(pr, "fwhm_ratio").map_or(Some("n/a".into()), |v| {
            Some(format!("FWHM X/P = {:.2}", v.f64()?))
        })?,
        3 if truthy(rf.get("ok")) && nn(rf, "cv_ratio_mad").is_some() => {
            format!("CV (MAD) = {:.0}%", 100.0 * fnum(rf, "cv_ratio_mad"))
        }
        3 => "n/a".into(),
        4 if nn(rc, "p").is_some() => format!(
            "p (Cochran) = {} over {} samples",
            pg(num(rc.get("p"))?, 2),
            num(rc.get("k"))? as i64
        ),
        4 => "fewer than 2 samples".into(),
        5 if nn(rc, "cv").is_some() => {
            format!("CV across samples = {:.0}%", 100.0 * fnum(rc, "cv"))
        }
        5 => "n/a".into(),
        6 if nn(kin, "spearman_parent").is_some() => {
            format!("\u{3c1} = {:.2}", num(kin.get("spearman_parent"))?)
        }
        6 => "fewer than 3 samples with a time".into(),
        7 if nn(pr, "p_shift").is_some() => {
            format!("p shifts = {}", pg(num(pr.get("p_shift"))?, 2))
        }
        7 => "n/a".into(),
        _ => (if feat[8] != 0.0 { "yes" } else { "no" }).into(),
    })
}

/// `_english(msg)` for the two warnings the verdict repeats: the text of the catalog with the parameters in place.
pub fn english(msg: &Value) -> String {
    let key = msg.get("key").and_then(|v| v.str()).unwrap_or("");
    let mut t = match key {
        "of.warn.flatTop" => "The peak of the parent has a flat top ({n} scans at the maximum): possible detector saturation, the F/P ratio and the shape of the peak are affected.",
        "of.warn.massCoincidence" => "Possible mass coincidence: at unit resolution there are nearby ions (m/z {mz}) that co-elute.",
        k => k,
    }
    .to_string();
    if let Some(Value::Obj(ps)) = msg.get("params") {
        for (k, v) in ps {
            let s = match v {
                Value::Str(s) => s.clone(),
                Value::Num(n) if n.fract() == 0.0 => format!("{}", *n as i64),
                Value::Num(n) => format!("{n}"),
                Value::Bool(b) => (if *b { "True" } else { "False" }).into(),
                _ => "None".into(),
            };
            t = t.replace(&format!("{{{k}}}"), &s);
        }
    }
    t
}
