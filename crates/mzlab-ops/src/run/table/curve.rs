//! The text of `quant.curve@1`: calibration lines fitted to the integrated peaks of the standards (`mzlab_core::calib`), the back-calculation of every
//! standard and of the unknowns, LOD/LOQ by two named conventions, as a `taratura/1` JSON and a long CSV of concentrations. Inputs are read by
//! `curve_read.rs`, outputs written by `curve_write.rs`. Every automatic choice (a point left out or kept with a doubtful response, the range, a
//! limit not computed) is a line of `log`; outliers only on the user's `exclude`. Bytes in, text out: `flow run` and `flowcal`.

use mzlab_core::calib::{fit, Fit, Model};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};

use super::curve_read::{
    bad_param, parse_responses, parse_standards, settings, Resp, Settings, Std,
};
use super::curve_write::{
    accuracy, limits, opt_csv, opt_str, sn_limit, write_log, write_points, Log, Point,
};
use super::quant::quoted;

/// ICH Q2(R2) §3.2.4: LOD = 3.3σ/S and LOQ = 10σ/S with σ the residual standard deviation of the line; S/N 3:1 and 10:1.
const LOD_K: f64 = 3.3;
const LOQ_K: f64 = 10.0;
const LOD_SN: f64 = 3.0;
const LOQ_SN: f64 = 10.0;
pub const ROLES: [&str; 4] = ["standard", "qc", "sample", "blank"];
/// The header of the concentrations CSV.
pub const CSV_HEAD: &str =
    "analyte,sample,role,level,signal,istd_signal,response,concentration,unit,accuracy_pct,note\n";
/// What a sample absent from the standards table is: an unknown, without level or dilution.
static UNLISTED: Std = Std {
    sample: String::new(),
    analyte: String::new(),
    level: String::new(),
    conc: None,
    unit: String::new(),
    role: "sample",
    dilution: 1.0,
};

/// The response of a sample: signal, IS signal, ratio (or the signal), or the reason there is none.
type Response = (
    Option<f64>,
    Option<f64>,
    std::result::Result<f64, &'static str>,
);

/// One run: the inputs, the analytes and samples in the order met, the log and the conditions that become warnings.
struct Run<'a> {
    set: Settings,
    resp: &'a [Resp],
    stds: &'a [Std],
    analytes: Vec<&'a str>,
    samples: Vec<&'a str>,
    log: Vec<Log>,
    /// a curve failed, an unknown is out of range, a sample has no response
    warn: [bool; 3],
}

impl<'a> Run<'a> {
    fn new(set: Settings, resp: &'a [Resp], stds: &'a [Std]) -> Result<Run<'a>> {
        let mut analytes: Vec<&str> = Vec::new();
        let mut samples: Vec<&str> = Vec::new();
        for r in resp {
            if !analytes.contains(&r.analyte.as_str()) {
                analytes.push(&r.analyte);
            }
            if !samples.contains(&r.sample.as_str()) {
                samples.push(&r.sample);
            }
        }
        for s in stds {
            if !samples.contains(&s.sample.as_str()) {
                samples.push(&s.sample);
            }
        }
        for (a, is) in &set.istd {
            if !analytes.contains(&is.as_str()) || !(a.is_empty() || analytes.contains(&a.as_str()))
            {
                return Err(bad_param("istd", &format!("{a}|{is}")));
            }
        }
        for (a, s, _) in &set.exclude {
            if !(a == "*" || analytes.contains(&a.as_str())) || !samples.contains(&s.as_str()) {
                return Err(bad_param("exclude", &format!("{a}|{s}")));
            }
        }
        let mut run = Run {
            set,
            resp,
            stds,
            analytes,
            samples,
            log: Vec::new(),
            warn: [false; 3],
        };
        for s in run.samples.clone() {
            if !stds.iter().any(|t| t.sample == s) {
                run.note("", s, "treated_as_sample", "not_in_standards", "");
            }
        }
        Ok(run)
    }

    fn note(
        &mut self,
        analyte: &str,
        sample: &str,
        choice: &'static str,
        reason: &'static str,
        note: &str,
    ) {
        self.log.push(Log {
            analyte: analyte.to_string(),
            sample: sample.to_string(),
            choice,
            reason,
            note: note.to_string(),
        });
    }

    fn istd_of(&self, a: &str) -> Option<&'a str> {
        let istd: &[(String, String)] = &self.set.istd;
        let found = istd
            .iter()
            .find(|(x, _)| x == a)
            .or_else(|| istd.iter().find(|(x, _)| x.is_empty()));
        found
            .map(|(_, s)| s.as_str())
            .filter(|s| *s != a)
            .and_then(|s| self.analytes.iter().copied().find(|x| *x == s))
    }

    fn peak(&self, a: &str, s: &str) -> Option<&'a Resp> {
        self.resp.iter().find(|r| r.analyte == a && r.sample == s)
    }

    /// The row of the standards table for this analyte and sample (an analyte-specific row before a row for every analyte).
    fn info(&self, a: &str, s: &str) -> &'a Std {
        let stds = self.stds;
        stds.iter()
            .find(|t| t.sample == s && t.analyte == a)
            .or_else(|| stds.iter().find(|t| t.sample == s && t.analyte.is_empty()))
            .unwrap_or(&UNLISTED)
    }

    fn response(&self, a: &str, istd: Option<&str>, s: &str) -> Response {
        let sig = self.peak(a, s).and_then(|r| r.signal);
        let Some(is) = istd else {
            return (sig, None, sig.ok_or("no_response"));
        };
        let isig = self.peak(is, s).and_then(|r| r.signal);
        let y = match (sig, isig) {
            (None, _) => Err("no_response"),
            (_, Some(i)) if i <= 0.0 => Err("no_istd"),
            (_, None) => Err("no_istd"),
            (Some(v), Some(i)) => Ok(v / i),
        };
        (sig, isig, y)
    }

    /// The one unit of the standards of an analyte.
    fn unit(&self, a: &str) -> Result<String> {
        let units: Vec<&str> = (self.samples.iter().map(|s| self.info(a, s)))
            .filter(|t| t.role == "standard")
            .map(|t| t.unit.as_str())
            .collect();
        if units.iter().any(|u| *u != units[0]) {
            return Err(Error::new("err.flow.curve.unit").with("analyte", a));
        }
        Ok(units.first().copied().unwrap_or("").to_string())
    }

    /// The standards of an analyte, each in the fit or left out with a line of the log.
    fn points(&mut self, a: &str, istd: Option<&str>) -> Vec<Point<'a>> {
        let mut points = Vec::new();
        for s in self.samples.clone() {
            let t = self.info(a, s);
            if t.role != "standard" {
                continue;
            }
            let x = t.conc.unwrap_or(0.0);
            let (_, _, y) = self.response(a, istd, s);
            let mut p = Point {
                std: t,
                x,
                y: y.ok(),
                w: None,
                out: y.err(),
            };
            let user = self
                .set
                .exclude
                .iter()
                .find(|(ea, es, _)| (ea == a || ea == "*") && es == s);
            if let Some((_, _, why)) = user {
                p.out = Some("excluded_by_user");
                let why = why.clone();
                self.note(a, s, "excluded", "excluded_by_user", &why);
            } else if let Some(reason) = p.out {
                self.note(a, s, "excluded", reason, "");
            }
            if let Some(yv) = p.y {
                p.w = self.set.weight.of(x, yv);
                if p.out.is_none() && p.w.is_none() {
                    p.out = Some("weight_undefined");
                    self.note(a, s, "excluded", "weight_undefined", "");
                } else if p.out.is_none() && yv <= 0.0 {
                    // a blank-subtracted area or a ratio can be ≤ 0: it stays in the fit, but never unseen
                    self.note(a, s, "kept", "response_not_positive", &format!("{yv}"));
                }
            }
            points.push(p);
        }
        points
    }

    /// One element of `curves` and the rows of the analyte in the CSV.
    fn curve(&mut self, o: &mut Out, csv: &mut String, a: &str) -> Result<()> {
        let istd = self.istd_of(a);
        let unit = self.unit(a)?;
        let points = self.points(a, istd);
        let used: Vec<&Point> = points.iter().filter(|p| p.out.is_none()).collect();
        let xs: Vec<f64> = used.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = used.iter().filter_map(|p| p.y).collect();
        let ws: Vec<f64> = used.iter().filter_map(|p| p.w).collect();
        let fitted = fit(&xs, &ys, &ws, self.set.model, self.set.zero).map_err(|e| e.key());
        self.warn[0] |= fitted.is_err();
        let f = fitted.as_ref().ok();
        let range = self.range(a, &xs, f.is_some());
        o.raw("{\"analyte\":").str(a).raw(",\"istd\":");
        opt_str(o, istd);
        o.raw(",\"unit\":").str(&unit).raw(",\"error\":");
        opt_str(o, fitted.as_ref().err().copied());
        for (k, v) in [
            ("a", f.map(|f| f.a)),
            ("b", f.map(|f| f.b)),
            ("c", f.map(|f| f.c)),
        ] {
            o.raw(",\"").raw(k).raw("\":").opt(v);
        }
        o.raw(",\"n\":").int(used.len() as i64);
        o.raw(",\"r2\":").opt(f.and_then(|f| f.r2));
        o.raw(",\"s_yx\":").opt(f.and_then(|f| f.s_yx));
        o.raw(",\"s_w\":").opt(f.and_then(|f| f.s_w));
        o.raw(",\"range\":");
        match f {
            Some(_) => o.floats(&[range.0, range.1]),
            None => o.raw("null"),
        };
        let sn_pts: Vec<(f64, f64)> = (used.iter())
            .filter_map(|p| Some((p.x, self.peak(a, &p.std.sample)?.snr?)))
            .filter(|p| p.0 > 0.0)
            .collect();
        self.limits(o, a, f, &sn_pts);
        write_points(o, &points, f);
        o.raw("}");
        for s in self.samples.clone() {
            self.row(csv, a, istd, s, (f, range, &points), &unit);
        }
        Ok(())
    }

    /// The calibrated range: from the lowest standard above zero (the LLOQ; a zero standard helps the fit but quantifies nothing) to the highest.
    fn range(&mut self, a: &str, xs: &[f64], fitted: bool) -> (f64, f64) {
        let min = |v: &mut dyn Iterator<Item = f64>| v.fold(f64::INFINITY, f64::min);
        let lo = min(&mut xs.iter().copied().filter(|x| *x > 0.0));
        let all = min(&mut xs.iter().copied());
        let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if !lo.is_finite() {
            return (all, hi);
        }
        if fitted && all < lo {
            let note = format!("{lo}");
            self.note(a, "", "range_lower", "lowest_nonzero_standard", &note);
        }
        (lo, hi)
    }

    /// LOD and LOQ by both conventions; an ICH value that cannot be computed says why in the log.
    fn limits(&mut self, o: &mut Out, a: &str, f: Option<&Fit>, sn_pts: &[(f64, f64)]) {
        let slope = f.map(|f| f.b).filter(|b| *b > 0.0);
        if let Some(f) = f {
            if f.s_yx.is_none() {
                self.note(a, "", "limit_not_computed", "no_degrees_of_freedom", "ich");
            }
            if slope.is_none() {
                let note = format!("{}", f.b);
                self.note(a, "", "limit_not_computed", "slope_not_positive", &note);
            }
        }
        let ich = |k: f64| f.and_then(|f| f.s_yx).zip(slope).map(|(s, b)| k * s / b);
        let sn = |k: f64| f.and(sn_limit(sn_pts, k));
        limits(o, "lod", ich(LOD_K), sn(LOD_SN), ["3.3", "3"]);
        limits(o, "loq", ich(LOQ_K), sn(LOQ_SN), ["10", "10"]);
    }

    /// The row of one analyte in one sample: a standard says why it is out of the fit (its accuracy, not the range, is what it shows).
    fn row(
        &mut self,
        csv: &mut String,
        a: &str,
        istd: Option<&str>,
        s: &str,
        curve: (Option<&Fit>, (f64, f64), &[Point]),
        unit: &str,
    ) {
        let (f, range, points) = curve;
        let t = self.info(a, s);
        let (sig, isig, y) = self.response(a, istd, s);
        let back = match (f, y) {
            (None, _) => Err("no_curve"),
            (_, Err(r)) => Err(r),
            (Some(f), Ok(y)) => f.inverse(y).ok_or("no_root"),
        };
        let left_out = points
            .iter()
            .find(|p| p.std.sample == s)
            .and_then(|p| p.out);
        let note = match back {
            Err(r) => r,
            Ok(_) if t.role == "standard" => left_out.unwrap_or("ok"),
            Ok(b) if b < range.0 => "below_range",
            Ok(b) if b > range.1 => "above_range",
            Ok(_) => "ok",
        };
        self.warn[1] |= note.ends_with("_range") && t.role != "standard";
        self.warn[2] |= matches!(note, "no_response" | "no_istd" | "no_root");
        let conc = back.ok().map(|b| b * t.dilution);
        let acc = (back.ok())
            .zip(t.conc.filter(|c| *c > 0.0 && t.role != "sample"))
            .map(|(b, n)| accuracy(b, n));
        let cells = [
            quoted(a),
            quoted(s),
            t.role.to_string(),
            quoted(&t.level),
            opt_csv(sig),
            opt_csv(isig),
        ];
        let more = [opt_csv(y.ok()), opt_csv(conc), quoted(unit), opt_csv(acc)];
        for c in cells.iter().chain(&more) {
            csv.push_str(c);
            csv.push(',');
        }
        csv.push_str(note);
        csv.push('\n');
    }
}

/// The `taratura/1` JSON, the concentrations CSV and the `warn.flow.curve.*` keys of the conditions met.
pub fn curve_text(
    params: &[(String, Value)],
    responses: &[u8],
    standards: &[u8],
) -> Result<(String, String, Vec<String>)> {
    let set = settings(params)?;
    let resp = parse_responses(responses, set.by_height)?;
    let stds = parse_standards(standards)?;
    let mut run = Run::new(set, &resp, &stds)?;
    let mut o = Out::default();
    let model = if run.set.model == Model::Linear {
        "linear"
    } else {
        "quadratic"
    };
    o.raw("{\"format\":\"taratura/1\",\"signal\":")
        .str(if run.set.by_height { "height" } else { "area" });
    o.raw(",\"model\":")
        .str(model)
        .raw(",\"weight\":")
        .str(run.set.weight.name());
    o.raw(",\"zero\":")
        .raw(if run.set.zero { "true" } else { "false" })
        .raw(",\"curves\":[");
    let mut csv = String::from(CSV_HEAD);
    let is_names: Vec<String> = run.set.istd.iter().map(|(_, s)| s.clone()).collect();
    let mut first = true;
    for a in run.analytes.clone() {
        if is_names.iter().any(|n| n == a) {
            run.note(a, "", "not_fitted", "internal_standard", "");
            continue;
        }
        if !first {
            o.raw(",");
        }
        first = false;
        run.curve(&mut o, &mut csv, a)?;
    }
    o.raw("],\"log\":");
    write_log(&mut o, &run.log);
    o.raw("}\n");
    let excluded = run.log.iter().any(|l| l.choice == "excluded");
    let unlisted = run.log.iter().any(|l| l.choice == "treated_as_sample");
    let not_positive = run.log.iter().any(|l| l.reason == "response_not_positive");
    let keys = [
        "warn.flow.curve.failed",
        "warn.flow.curve.excluded",
        "warn.flow.curve.range",
        "warn.flow.curve.noResponse",
        "warn.flow.curve.unlisted",
        "warn.flow.curve.notPositive",
    ];
    let on = [
        run.warn[0],
        excluded,
        run.warn[1],
        run.warn[2],
        unlisted,
        not_positive,
    ];
    let warnings = (keys.iter().zip(on))
        .filter(|(_, on)| *on)
        .map(|(k, _)| k.to_string())
        .collect();
    Ok((o.0, csv, warnings))
}

#[cfg(test)]
#[path = "curve_tests.rs"]
mod tests;
