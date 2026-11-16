//! The text of `quant.curve@1`: calibration lines fitted to the integrated peaks of the standards (`mzlab_core::calib`), the back-calculation of every
//! standard and of the unknowns, LOD/LOQ by two named conventions, as a `taratura/1` JSON and a long CSV of concentrations. Responses are the CSV of
//! `chrom.transitions` (`peaks`), of the manual integrations or one written by hand; the standards are `sample;level;concentration;unit;role`.
//! No point is ever left out without a line of `log` that says why; outliers only on the user's `exclude`. Bytes in, text out: `flow run` and `flowcal`.

use mzlab_core::calib::{fit, Fit, Model, Weight};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};

use super::quant::{fields, param, quoted};

/// ICH Q2(R2) §3.2.4: LOD = 3.3σ/S and LOQ = 10σ/S with σ the residual standard deviation of the line; S/N 3:1 and 10:1.
const LOD_K: f64 = 3.3;
const LOQ_K: f64 = 10.0;
const LOD_SN: f64 = 3.0;
const LOQ_SN: f64 = 10.0;
/// The values of `peak` (`chrom.transitions`) that are a peak; any other one is no response.
const PEAK_OK: [&str; 2] = ["ok", "edge"];
pub const ROLES: [&str; 4] = ["standard", "qc", "sample", "blank"];
/// The header of the concentrations CSV.
pub const CSV_HEAD: &str =
    "analyte,sample,role,level,signal,istd_signal,response,concentration,unit,accuracy_pct,note\n";

/// One CSV: its delimiter, lower-case header and rows with their line numbers.
struct Csv {
    d: char,
    head: Vec<String>,
    rows: Vec<(usize, Vec<String>)>,
}

impl Csv {
    fn read(bytes: &[u8], bad: impl Fn(String) -> Error) -> Result<Csv> {
        let text = String::from_utf8_lossy(bytes);
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.trim_start_matches('\u{feff}')))
            .filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'));
        let (_, head) = lines.next().ok_or_else(|| bad("empty file".into()))?;
        let d = [';', '\t', ',']
            .into_iter()
            .find(|d| head.contains(*d))
            .unwrap_or(',');
        let head = fields(head, d).iter().map(|h| h.to_lowercase()).collect();
        let rows = lines.map(|(n, l)| (n, fields(l, d))).collect();
        Ok(Csv { d, head, rows })
    }
    fn col(&self, name: &str) -> Option<usize> {
        self.head.iter().position(|h| h == name)
    }
    /// A number of a cell; a decimal comma is read when the delimiter is not `,`. `Ok(None)` for an empty cell.
    fn num(&self, s: &str) -> std::result::Result<Option<f64>, ()> {
        if s.is_empty() {
            return Ok(None);
        }
        let s = if self.d == ',' {
            s.to_string()
        } else {
            s.replace(',', ".")
        };
        s.parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(Some)
            .ok_or(())
    }
}

fn cell(f: &[String], c: Option<usize>) -> &str {
    c.and_then(|c| f.get(c)).map(String::as_str).unwrap_or("")
}

fn row_err(file: &str, n: usize, what: &str) -> Error {
    Error::new("err.flow.curve.row")
        .with("file", file)
        .with("detail", format!("line {n}: {what}"))
}

/// One integrated peak: `signal` is `None` when the row has no peak.
struct Resp {
    analyte: String,
    sample: String,
    signal: Option<f64>,
    snr: Option<f64>,
}

/// The analyte of a row is `analyte`, else the user's `label`, else `name`, else `channel` (the manual integrations); `sum` rows are skipped.
fn parse_responses(bytes: &[u8], by_height: bool) -> Result<Vec<Resp>> {
    let bad = |d: String| Error::new("err.flow.curve.responses").with("detail", d);
    let csv = Csv::read(bytes, bad)?;
    let need = |n: &str| csv.col(n).ok_or_else(|| bad(format!("no column {n}")));
    let sample = need("sample")?;
    let signal = need(if by_height { "height" } else { "area" })?;
    let names = ["analyte", "label", "name", "channel"].map(|n| csv.col(n));
    if names.iter().all(Option::is_none) {
        return Err(bad("no column analyte, name or channel".into()));
    }
    let (snr, peak) = (csv.col("snr"), csv.col("peak"));
    let mut out: Vec<Resp> = Vec::new();
    for (n, f) in &csv.rows {
        let s = cell(f, Some(sample)).to_string();
        if s.is_empty() || s == "sum" {
            continue;
        }
        let analyte = names
            .iter()
            .map(|c| cell(f, *c))
            .find(|v| !v.is_empty())
            .ok_or_else(|| row_err("responses", *n, "no analyte"))?
            .to_string();
        if out.iter().any(|r| r.analyte == analyte && r.sample == s) {
            return Err(Error::new("err.flow.curve.duplicate")
                .with("analyte", analyte)
                .with("sample", s));
        }
        let is_peak = peak.is_none() || PEAK_OK.contains(&cell(f, peak));
        let value = csv
            .num(cell(f, Some(signal)))
            .map_err(|_| row_err("responses", *n, "signal"))?;
        let sn = csv
            .num(cell(f, snr))
            .map_err(|_| row_err("responses", *n, "snr"))?;
        out.push(Resp {
            analyte,
            sample: s,
            signal: value.filter(|_| is_peak),
            snr: sn.filter(|v| *v > 0.0 && is_peak),
        });
    }
    if out.is_empty() {
        return Err(bad("no row".into()));
    }
    Ok(out)
}

/// One row of the standards table; `analyte` empty = every analyte.
struct Std {
    sample: String,
    analyte: String,
    level: String,
    conc: Option<f64>,
    unit: String,
    role: &'static str,
    dilution: f64,
}

fn role_of(s: &str) -> Option<&'static str> {
    match s.to_lowercase().as_str() {
        "standard" | "std" | "calibration" | "cal" => Some("standard"),
        "qc" => Some("qc"),
        "sample" | "unknown" | "" => Some("sample"),
        "blank" => Some("blank"),
        _ => None,
    }
}

fn parse_standards(bytes: &[u8]) -> Result<Vec<Std>> {
    let bad = |d: String| Error::new("err.flow.curve.standards").with("detail", d);
    let csv = Csv::read(bytes, bad)?;
    let sample = csv
        .col("sample")
        .ok_or_else(|| bad("no column sample".into()))?;
    let conc = csv
        .col("concentration")
        .ok_or_else(|| bad("no column concentration".into()))?;
    let [analyte, level, unit, role, dil] =
        ["analyte", "level", "unit", "role", "dilution"].map(|n| csv.col(n));
    let mut out: Vec<Std> = Vec::new();
    for (n, f) in &csv.rows {
        let r = cell(f, role);
        let role = role_of(r).ok_or_else(|| Error::new("err.flow.curve.role").with("role", r))?;
        let s = Std {
            sample: cell(f, Some(sample)).to_string(),
            analyte: cell(f, analyte).to_string(),
            level: cell(f, level).to_string(),
            conc: csv
                .num(cell(f, Some(conc)))
                .map_err(|_| row_err("standards", *n, "concentration"))?,
            unit: cell(f, unit).to_string(),
            role,
            dilution: csv
                .num(cell(f, dil))
                .map_err(|_| row_err("standards", *n, "dilution"))?
                .unwrap_or(1.0),
        };
        if s.sample.is_empty() {
            return Err(row_err("standards", *n, "sample"));
        }
        if s.role == "standard" && s.conc.is_none_or(|c| c < 0.0) {
            return Err(row_err("standards", *n, "concentration of a standard"));
        }
        if s.dilution <= 0.0 {
            return Err(row_err("standards", *n, "dilution"));
        }
        if out
            .iter()
            .any(|o| o.sample == s.sample && o.analyte == s.analyte)
        {
            return Err(Error::new("err.flow.curve.duplicate")
                .with("analyte", s.analyte)
                .with("sample", s.sample));
        }
        out.push(s);
    }
    if out.is_empty() {
        return Err(bad("no row".into()));
    }
    Ok(out)
}

/// The resolved parameters.
struct Settings {
    by_height: bool,
    model: Model,
    weight: Weight,
    zero: bool,
    /// (analyte or empty = every analyte, internal standard)
    istd: Vec<(String, String)>,
    /// (analyte or `*`, sample, the user's reason)
    exclude: Vec<(String, String, String)>,
}

fn bad_param(name: &'static str, value: &str) -> Error {
    Error::new("err.flow.curve.param")
        .with("name", name)
        .with("value", value)
}

fn settings(params: &[(String, Value)]) -> Result<Settings> {
    let text = |n: &str, d: &'static str| {
        param(params, n)
            .and_then(|v| v.str())
            .unwrap_or(d)
            .trim()
            .to_string()
    };
    let signal = text("signal", "area");
    if signal != "area" && signal != "height" {
        return Err(bad_param("signal", &signal));
    }
    let model = match text("model", "linear").as_str() {
        "linear" => Model::Linear,
        "quadratic" => Model::Quadratic,
        m => return Err(bad_param("model", m)),
    };
    let w = text("weight", "1/x");
    let weight = Weight::parse(&w).ok_or_else(|| bad_param("weight", &w))?;
    let zero = match param(params, "zero") {
        Some(v) => v
            .bool()
            .or_else(|| v.str().map(|s| s == "true"))
            .unwrap_or(false),
        None => false,
    };
    let is = text("istd", "");
    let istd = if is.is_empty() {
        Vec::new()
    } else if is.contains('|') {
        let mut v = Vec::new();
        for pair in is.split(';').map(str::trim).filter(|p| !p.is_empty()) {
            match pair.split_once('|') {
                Some((a, s)) if !a.trim().is_empty() && !s.trim().is_empty() => {
                    v.push((a.trim().to_string(), s.trim().to_string()))
                }
                _ => return Err(bad_param("istd", pair)),
            }
        }
        v
    } else {
        vec![(String::new(), is)]
    };
    let mut exclude = Vec::new();
    for item in text("exclude", "")
        .split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let mut parts = item.splitn(3, '|').map(str::trim);
        match (parts.next(), parts.next(), parts.next()) {
            (Some(a), Some(s), why) if !a.is_empty() && !s.is_empty() => {
                exclude.push((a.to_string(), s.to_string(), why.unwrap_or("").to_string()))
            }
            _ => return Err(bad_param("exclude", item)),
        }
    }
    Ok(Settings {
        by_height: signal == "height",
        model,
        weight,
        zero,
        istd,
        exclude,
    })
}

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

fn opt_csv(v: Option<f64>) -> String {
    v.filter(|v| v.is_finite())
        .map(|v| format!("{v}"))
        .unwrap_or_default()
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

/// The `taratura/1` JSON, the concentrations CSV and the `warn.flow.curve.*` keys of the conditions met.
pub fn curve_text(
    params: &[(String, Value)],
    responses: &[u8],
    standards: &[u8],
) -> Result<(String, String, Vec<String>)> {
    let set = settings(params)?;
    let resp = parse_responses(responses, set.by_height)?;
    let stds = parse_standards(standards)?;
    let mut analytes: Vec<&str> = Vec::new();
    let mut samples: Vec<&str> = Vec::new();
    for r in &resp {
        if !analytes.contains(&r.analyte.as_str()) {
            analytes.push(&r.analyte);
        }
        if !samples.contains(&r.sample.as_str()) {
            samples.push(&r.sample);
        }
    }
    for s in &stds {
        if !samples.contains(&s.sample.as_str()) {
            samples.push(&s.sample);
        }
    }
    let istd_of = |a: &str| {
        set.istd
            .iter()
            .find(|(x, _)| x == a)
            .or_else(|| set.istd.iter().find(|(x, _)| x.is_empty()))
            .map(|(_, s)| s.as_str())
            .filter(|s| *s != a)
    };
    for (a, is) in &set.istd {
        if !analytes.contains(&is.as_str()) || !(a.is_empty() || analytes.contains(&a.as_str())) {
            return Err(bad_param("istd", &format!("{a}|{is}")));
        }
    }
    for (a, s, _) in &set.exclude {
        if !(a == "*" || analytes.contains(&a.as_str())) || !samples.contains(&s.as_str()) {
            return Err(bad_param("exclude", &format!("{a}|{s}")));
        }
    }
    let is_names: Vec<&str> = set.istd.iter().map(|(_, s)| s.as_str()).collect();
    let signal_of = |a: &str, s: &str| resp.iter().find(|r| r.analyte == a && r.sample == s);
    let mut log: Vec<Log> = Vec::new();
    let unlisted = Std {
        sample: String::new(),
        analyte: String::new(),
        level: String::new(),
        conc: None,
        unit: String::new(),
        role: "sample",
        dilution: 1.0,
    };
    for s in &samples {
        if !stds.iter().any(|t| t.sample == *s) {
            log.push(Log {
                analyte: String::new(),
                sample: s.to_string(),
                choice: "treated_as_sample",
                reason: "not_in_standards",
                note: String::new(),
            });
        }
    }
    let (mut warn_failed, mut warn_range, mut warn_none) = (false, false, false);
    let mut o = Out::default();
    o.raw("{\"format\":\"taratura/1\",\"signal\":")
        .str(if set.by_height { "height" } else { "area" })
        .raw(",\"model\":")
        .str(if set.model == Model::Linear {
            "linear"
        } else {
            "quadratic"
        })
        .raw(",\"weight\":")
        .str(set.weight.name())
        .raw(",\"zero\":")
        .raw(if set.zero { "true" } else { "false" })
        .raw(",\"curves\":[");
    let mut csv = String::from(CSV_HEAD);
    let mut first = true;
    for a in analytes.iter().copied() {
        if is_names.contains(&a) {
            log.push(Log {
                analyte: a.to_string(),
                sample: String::new(),
                choice: "not_fitted",
                reason: "internal_standard",
                note: String::new(),
            });
            continue;
        }
        let istd = istd_of(a);
        let info = |s: &str| {
            stds.iter()
                .find(|t| t.sample == s && t.analyte == a)
                .or_else(|| stds.iter().find(|t| t.sample == s && t.analyte.is_empty()))
                .unwrap_or(&unlisted)
        };
        // the response of a sample: signal, IS signal, ratio (or the signal), or the reason there is none
        let response = |s: &str| -> (
            Option<f64>,
            Option<f64>,
            std::result::Result<f64, &'static str>,
        ) {
            let sig = signal_of(a, s).and_then(|r| r.signal);
            let Some(is) = istd else {
                return (sig, None, sig.ok_or("no_response"));
            };
            let isig = signal_of(is, s).and_then(|r| r.signal);
            let y = match (sig, isig) {
                (None, _) => Err("no_response"),
                (_, Some(i)) if i <= 0.0 => Err("no_istd"),
                (_, None) => Err("no_istd"),
                (Some(v), Some(i)) => Ok(v / i),
            };
            (sig, isig, y)
        };
        let unit = {
            let units: Vec<&str> = samples
                .iter()
                .map(|s| info(s))
                .filter(|t| t.role == "standard")
                .map(|t| t.unit.as_str())
                .collect();
            if units.iter().any(|u| *u != units[0]) {
                return Err(Error::new("err.flow.curve.unit").with("analyte", a));
            }
            units.first().copied().unwrap_or("").to_string()
        };
        let mut points: Vec<Point> = Vec::new();
        for s in samples.iter().copied() {
            let t = info(s);
            if t.role != "standard" {
                continue;
            }
            let x = t.conc.unwrap_or(0.0);
            let (_, _, y) = response(s);
            let mut p = Point {
                std: t,
                x,
                y: y.ok(),
                w: None,
                out: y.err(),
            };
            let user = set
                .exclude
                .iter()
                .find(|(ea, es, _)| (ea == a || ea == "*") && es == s);
            if let Some((_, _, why)) = user {
                p.out = Some("excluded_by_user");
                log.push(Log {
                    analyte: a.to_string(),
                    sample: s.to_string(),
                    choice: "excluded",
                    reason: "excluded_by_user",
                    note: why.clone(),
                });
            } else if let Some(reason) = p.out {
                log.push(Log {
                    analyte: a.to_string(),
                    sample: s.to_string(),
                    choice: "excluded",
                    reason,
                    note: String::new(),
                });
            }
            if let Some(yv) = p.y {
                p.w = set.weight.of(x, yv);
                if p.out.is_none() && p.w.is_none() {
                    p.out = Some("weight_undefined");
                    log.push(Log {
                        analyte: a.to_string(),
                        sample: s.to_string(),
                        choice: "excluded",
                        reason: "weight_undefined",
                        note: String::new(),
                    });
                }
            }
            points.push(p);
        }
        let used: Vec<&Point> = points.iter().filter(|p| p.out.is_none()).collect();
        let xs: Vec<f64> = used.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = used.iter().filter_map(|p| p.y).collect();
        let ws: Vec<f64> = used.iter().filter_map(|p| p.w).collect();
        let fitted: std::result::Result<Fit, &'static str> =
            fit(&xs, &ys, &ws, set.model, set.zero).map_err(|e| e.key());
        let range = (
            xs.iter().copied().fold(f64::INFINITY, f64::min),
            xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        warn_failed |= fitted.is_err();
        if !first {
            o.raw(",");
        }
        first = false;
        o.raw("{\"analyte\":").str(a).raw(",\"istd\":");
        match istd {
            Some(i) => o.str(i),
            None => o.raw("null"),
        };
        o.raw(",\"unit\":").str(&unit).raw(",\"error\":");
        match &fitted {
            Err(k) => o.str(k),
            Ok(_) => o.raw("null"),
        };
        let f = fitted.as_ref().ok();
        o.raw(",\"a\":").opt(f.map(|f| f.a));
        o.raw(",\"b\":").opt(f.map(|f| f.b));
        o.raw(",\"c\":").opt(f.map(|f| f.c));
        o.raw(",\"n\":").int(used.len() as i64);
        o.raw(",\"r2\":").opt(f.and_then(|f| f.r2));
        o.raw(",\"s_yx\":").opt(f.and_then(|f| f.s_yx));
        o.raw(",\"s_w\":").opt(f.and_then(|f| f.s_w));
        o.raw(",\"range\":");
        if f.is_some() {
            o.floats(&[range.0, range.1]);
        } else {
            o.raw("null");
        }
        let slope = f.map(|f| f.b).filter(|b| *b > 0.0);
        let ich = |k: f64| f.and_then(|f| f.s_yx).zip(slope).map(|(s, b)| k * s / b);
        let sn_pts: Vec<(f64, f64)> = used
            .iter()
            .filter_map(|p| Some((p.x, signal_of(a, &p.std.sample)?.snr?)))
            .filter(|p| p.0 > 0.0)
            .collect();
        let sn = |k: f64| f.and(sn_limit(&sn_pts, k));
        limits(&mut o, "lod", ich(LOD_K), sn(LOD_SN), ["3.3", "3"]);
        limits(&mut o, "loq", ich(LOQ_K), sn(LOQ_SN), ["10", "10"]);
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
            match p.out {
                Some(r) => o.str(r),
                None => o.raw("null"),
            };
            o.raw(",\"fit\":").opt(f.map(|f| f.predict(p.x)));
            o.raw(",\"residual\":")
                .opt(f.zip(p.y).map(|(f, y)| y - f.predict(p.x)));
            o.raw(",\"back\":").opt(back);
            o.raw(",\"accuracy_pct\":")
                .opt(back.filter(|_| p.x > 0.0).map(|b| 100.0 * b / p.x));
            o.raw("}");
        }
        o.raw("]}");
        for s in samples.iter().copied() {
            let t = info(s);
            let (sig, isig, y) = response(s);
            let back = match (&fitted, y) {
                (Err(_), _) => Err("no_curve"),
                (_, Err(r)) => Err(r),
                (Ok(f), Ok(y)) => f.inverse(y).ok_or("no_root"),
            };
            // a standard says why it is out of the fit; its accuracy, not the range, is what it shows
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
            warn_range |= note.ends_with("_range") && t.role != "standard";
            warn_none |= matches!(note, "no_response" | "no_istd" | "no_root");
            let conc = back.ok().map(|b| b * t.dilution);
            let acc = conc
                .zip(t.conc.filter(|c| *c > 0.0 && t.role != "sample"))
                .map(|(c, n)| 100.0 * c / n);
            csv += &format!(
                "{},{},{},{},{},{},{},{},{},{},{note}\n",
                quoted(a),
                quoted(s),
                t.role,
                quoted(&t.level),
                opt_csv(sig),
                opt_csv(isig),
                opt_csv(y.ok()),
                opt_csv(conc),
                quoted(&unit),
                opt_csv(acc)
            );
        }
    }
    o.raw("],\"log\":[");
    for (k, l) in log.iter().enumerate() {
        if k > 0 {
            o.raw(",");
        }
        o.raw("{\"analyte\":").str(&l.analyte);
        o.raw(",\"sample\":").str(&l.sample);
        o.raw(",\"choice\":").str(l.choice);
        o.raw(",\"reason\":").str(l.reason);
        o.raw(",\"note\":").str(&l.note);
        o.raw("}");
    }
    o.raw("]}\n");
    let mut warnings = Vec::new();
    let excluded = log.iter().any(|l| l.choice == "excluded");
    let unlisted = log.iter().any(|l| l.choice == "treated_as_sample");
    for (on, key) in [
        (warn_failed, "warn.flow.curve.failed"),
        (excluded, "warn.flow.curve.excluded"),
        (warn_range, "warn.flow.curve.range"),
        (warn_none, "warn.flow.curve.noResponse"),
        (unlisted, "warn.flow.curve.unlisted"),
    ] {
        if on {
            warnings.push(key.to_string());
        }
    }
    Ok((o.0, csv, warnings))
}

#[cfg(test)]
#[path = "curve_tests.rs"]
mod tests;
