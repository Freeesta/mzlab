//! The pieces of the `quant.curve@1` texts that only write: the points of a curve with fit, residual and back-calculated value, the LOD/LOQ
//! pairs with the name of their method, the `log` array, the CSV cells. The choices are made in `curve.rs`; nothing here decides.

use mzlab_core::calib::Fit;
use mzlab_core::json::Out;

use super::curve_read::Std;

/// One line of `log`: a choice made on a point or an analyte, with its reason.
pub(super) struct Log {
    pub(super) analyte: String,
    pub(super) sample: String,
    pub(super) choice: &'static str,
    pub(super) reason: &'static str,
    pub(super) note: String,
}

/// One standard of a curve with everything the output shows about it.
pub(super) struct Point<'a> {
    pub(super) std: &'a Std,
    pub(super) x: f64,
    pub(super) y: Option<f64>,
    pub(super) w: Option<f64>,
    /// `None` = in the fit, else why not.
    pub(super) out: Option<&'static str>,
}

/// Accuracy of a back-calculated value against its nominal concentration, both in the injected solution (the dilution is not applied): the
/// same number in the JSON points and in the CSV.
pub(super) fn accuracy(back: f64, nominal: f64) -> f64 {
    100.0 * back / nominal
}

pub(super) fn opt_csv(v: Option<f64>) -> String {
    v.filter(|v| v.is_finite())
        .map(|v| format!("{v}"))
        .unwrap_or_default()
}

pub(super) fn opt_str(o: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => o.str(s),
        None => o.raw("null"),
    };
}

/// LOD or LOQ by S/N: k·x / (S/N) averaged over the lowest standard level of the fit that has an S/N.
pub(super) fn sn_limit(points: &[(f64, f64)], k: f64) -> Option<f64> {
    let lo = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let at: Vec<f64> = points
        .iter()
        .filter(|p| p.0 == lo)
        .map(|p| k * p.0 / p.1)
        .collect();
    (!at.is_empty()).then(|| at.iter().sum::<f64>() / at.len() as f64)
}

/// The two values of a limit, each with the name of its method (`ich_<k>_s_yx_over_slope`, `sn_<k>_lowest_standard`).
pub(super) fn limits(o: &mut Out, key: &str, ich: Option<f64>, sn: Option<f64>, k: [&str; 2]) {
    o.raw(",\"").raw(key).raw("\":[{\"method\":");
    o.str(&format!("ich_{}_s_yx_over_slope", k[0]))
        .raw(",\"value\":")
        .opt(ich);
    o.raw("},{\"method\":");
    o.str(&format!("sn_{}_lowest_standard", k[1]))
        .raw(",\"value\":")
        .opt(sn);
    o.raw("}]");
}

/// The `points` of a curve: every standard with its fit, residual, back-calculated value and accuracy.
pub(super) fn write_points(o: &mut Out, points: &[Point], f: Option<&Fit>) {
    o.raw(",\"points\":[");
    for (k, p) in points.iter().enumerate() {
        if k > 0 {
            o.raw(",");
        }
        let back = f.zip(p.y).and_then(|(f, y)| f.inverse(y));
        o.raw("{\"sample\":").str(&p.std.sample);
        o.raw(",\"level\":").str(&p.std.level);
        o.raw(",\"x\":").num(p.x);
        o.raw(",\"y\":").opt(p.y);
        o.raw(",\"weight\":").opt(p.w.filter(|_| p.out.is_none()));
        o.raw(",\"used\":")
            .raw(if p.out.is_none() { "true" } else { "false" });
        o.raw(",\"reason\":");
        opt_str(o, p.out);
        o.raw(",\"fit\":").opt(f.map(|f| f.predict(p.x)));
        o.raw(",\"residual\":")
            .opt(f.zip(p.y).map(|(f, y)| y - f.predict(p.x)));
        o.raw(",\"back\":").opt(back);
        o.raw(",\"accuracy_pct\":")
            .opt(back.filter(|_| p.x > 0.0).map(|b| accuracy(b, p.x)));
        o.raw("}");
    }
    o.raw("]");
}

/// The `log` array.
pub(super) fn write_log(o: &mut Out, log: &[Log]) {
    o.raw("[");
    for (k, l) in log.iter().enumerate() {
        o.raw(if k > 0 {
            ",{\"analyte\":"
        } else {
            "{\"analyte\":"
        })
        .str(&l.analyte);
        o.raw(",\"sample\":")
            .str(&l.sample)
            .raw(",\"choice\":")
            .str(l.choice);
        o.raw(",\"reason\":")
            .str(l.reason)
            .raw(",\"note\":")
            .str(&l.note)
            .raw("}");
    }
    o.raw("]");
}
