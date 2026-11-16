//! The text of `quant.curve@1`: calibration lines fitted to the integrated peaks of the standards (`mzlab_core::calib`), the back-calculation of every
//! standard and of the unknowns, LOD/LOQ by two named conventions, as a `taratura/1` JSON and a long CSV of concentrations. The inputs are read by
//! `curve_read.rs`. No point is ever left out without a line of `log` that says why; outliers only on the user's `exclude`. Bytes in, text out:
//! `flow run` and `flowcal`.

use mzlab_core::calib::{fit, Fit, Model};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};

use super::curve_read::{
    bad_param, parse_responses, parse_standards, settings, Resp, Settings, Std,
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

/// One line of `log`: a choice made on a point or an analyte, with its reason.
struct Log {
    analyte: String,
    sample: String,
    choice: &'static str,
    reason: &'static str,
    note: String,
}

/// One standard of a curve with everything the output shows about it.
struct Point<'a> {
    std: &'a Std,
    x: f64,
    y: Option<f64>,
    w: Option<f64>,
    /// `None` = in the fit, else why not.
    out: Option<&'static str>,
}

/// The response of a sample: signal, IS signal, ratio (or the signal), or the reason there is none.
type Response = (
    Option<f64>,
    Option<f64>,
    std::result::Result<f64, &'static str>,
);

fn opt_csv(v: Option<f64>) -> String {
    v.filter(|v| v.is_finite())
        .map(|v| format!("{v}"))
        .unwrap_or_default()
}

fn opt_str(o: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => o.str(s),
        None => o.raw("null"),
    };
}

/// LOD or LOQ by S/N: k·x / (S/N) averaged over the lowest standard level of the fit that has an S/N.
fn sn_limit(points: &[(f64, f64)], k: f64) -> Option<f64> {
    let lo = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let at: Vec<f64> = points
        .iter()
        .filter(|p| p.0 == lo)
        .map(|p| k * p.0 / p.1)
        .collect();
    (!at.is_empty()).then(|| at.iter().sum::<f64>() / at.len() as f64)
}

/// The two values of a limit, each with the name of its method (`ich_<k>_s_yx_over_slope`, `sn_<k>_lowest_standard`).
fn limits(o: &mut Out, key: &str, ich: Option<f64>, sn: Option<f64>, k: [&str; 2]) {
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
        let range = (
            xs.iter().copied().fold(f64::INFINITY, f64::min),
            xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        self.warn[0] |= fitted.is_err();
        let f = fitted.as_ref().ok();
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
        let slope = f.map(|f| f.b).filter(|b| *b > 0.0);
        let ich = |k: f64| f.and_then(|f| f.s_yx).zip(slope).map(|(s, b)| k * s / b);
        let sn_pts: Vec<(f64, f64)> = (used.iter())
            .filter_map(|p| Some((p.x, self.peak(a, &p.std.sample)?.snr?)))
            .filter(|p| p.0 > 0.0)
            .collect();
        let sn = |k: f64| f.and(sn_limit(&sn_pts, k));
        limits(o, "lod", ich(LOD_K), sn(LOD_SN), ["3.3", "3"]);
        limits(o, "loq", ich(LOQ_K), sn(LOQ_SN), ["10", "10"]);
        write_points(o, &points, f);
        o.raw("}");
        for s in self.samples.clone() {
            self.row(csv, a, istd, s, (f, range, &points), &unit);
        }
        Ok(())
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
        let acc = conc
            .zip(t.conc.filter(|c| *c > 0.0 && t.role != "sample"))
            .map(|(c, n)| 100.0 * c / n);
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

/// The `points` of a curve: every standard with its fit, residual, back-calculated value and accuracy.
fn write_points(o: &mut Out, points: &[Point], f: Option<&Fit>) {
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
            .opt(back.filter(|_| p.x > 0.0).map(|b| 100.0 * b / p.x));
        o.raw("}");
    }
    o.raw("]");
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
    o.raw("],\"log\":[");
    for (k, l) in run.log.iter().enumerate() {
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
    o.raw("]}\n");
    let excluded = run.log.iter().any(|l| l.choice == "excluded");
    let unlisted = run.log.iter().any(|l| l.choice == "treated_as_sample");
    let keys = [
        "warn.flow.curve.failed",
        "warn.flow.curve.excluded",
        "warn.flow.curve.range",
        "warn.flow.curve.noResponse",
        "warn.flow.curve.unlisted",
    ];
    let on = [run.warn[0], excluded, run.warn[1], run.warn[2], unlisted];
    let warnings = (keys.iter().zip(on))
        .filter(|(_, on)| *on)
        .map(|(k, _)| k.to_string())
        .collect();
    Ok((o.0, csv, warnings))
}

#[cfg(test)]
#[path = "curve_tests.rs"]
mod tests;
