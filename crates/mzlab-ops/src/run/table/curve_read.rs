//! The two CSV inputs of `quant.curve@1`: the integrated peaks (`chrom.transitions`, the manual integrations or one written by hand) and the
//! standards table `sample;level;concentration;unit;role`, and the parameters of the step. A cell that is not a number is an error with its line; nothing is guessed.

use mzlab_core::calib::{Model, Weight};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

use super::quant::{fields, param};

/// The values of `peak` (`chrom.transitions`) that are a peak: `edge` (cut at the edge of the window) is kept by default and logged, the user can
/// leave it out with `exclude`; any other value is no response, logged too.
const PEAK_OK: [&str; 2] = ["ok", "edge"];

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
pub(super) struct Resp {
    pub(super) analyte: String,
    pub(super) sample: String,
    pub(super) signal: Option<f64>,
    pub(super) snr: Option<f64>,
    /// the `peak` cell when it is not `ok`: `edge` keeps the signal, any other value drops it
    pub(super) peak: Option<String>,
    /// an S/N ≤ 0 of a peak, left out of the S/N limits
    pub(super) bad_snr: Option<f64>,
}

/// A row of the responses that is not a peak: (analyte or empty, line, reason).
pub(super) type Skipped = (String, usize, &'static str);

/// The analyte of a row is `analyte`, else the user's `label`, else `name`, else `channel` (the manual integrations); rows without a sample and the
/// `sum` rows of `chrom.transitions` are skipped and returned, so that the log can say so.
pub(super) fn parse_responses(bytes: &[u8], by_height: bool) -> Result<(Vec<Resp>, Vec<Skipped>)> {
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
    let mut skipped = Vec::new();
    for (n, f) in &csv.rows {
        let s = cell(f, Some(sample)).to_string();
        let name = names.iter().map(|c| cell(f, *c)).find(|v| !v.is_empty());
        if s.is_empty() || s == "sum" {
            let why = if s.is_empty() { "no_sample" } else { "sum_row" };
            skipped.push((name.unwrap_or("").to_string(), *n, why));
            continue;
        }
        let analyte = name
            .ok_or_else(|| row_err("responses", *n, "no analyte"))?
            .to_string();
        if out.iter().any(|r| r.analyte == analyte && r.sample == s) {
            return Err(Error::new("err.flow.curve.duplicate")
                .with("analyte", analyte)
                .with("sample", s));
        }
        let status = cell(f, peak);
        let is_peak = peak.is_none() || PEAK_OK.contains(&status);
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
            peak: (peak.is_some() && status != "ok").then(|| status.to_string()),
            bad_snr: sn.filter(|v| *v <= 0.0 && is_peak),
        });
    }
    if out.is_empty() {
        return Err(bad("no row".into()));
    }
    Ok((out, skipped))
}

/// One row of the standards table; `analyte` empty = every analyte.
pub(super) struct Std {
    pub(super) sample: String,
    pub(super) analyte: String,
    pub(super) level: String,
    pub(super) conc: Option<f64>,
    pub(super) unit: String,
    pub(super) role: &'static str,
    pub(super) dilution: f64,
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

pub(super) fn parse_standards(bytes: &[u8]) -> Result<Vec<Std>> {
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
pub(super) struct Settings {
    pub(super) by_height: bool,
    pub(super) model: Model,
    pub(super) weight: Weight,
    pub(super) zero: bool,
    /// (analyte or empty = every analyte, internal standard)
    pub(super) istd: Vec<(String, String)>,
    /// (analyte or `*`, sample, the user's reason)
    pub(super) exclude: Vec<(String, String, String)>,
}

pub(super) fn bad_param(name: &'static str, value: &str) -> Error {
    Error::new("err.flow.curve.param")
        .with("name", name)
        .with("value", value)
}

pub(super) fn settings(params: &[(String, Value)]) -> Result<Settings> {
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
    let zero = match param(params, "zero").map(|v| (v.bool(), v.str().map(str::trim))) {
        None => false,
        Some((Some(b), _)) => b,
        Some((_, Some("true"))) => true,
        Some((_, Some("false"))) => false,
        Some((_, s)) => return Err(bad_param("zero", s.unwrap_or("not a boolean"))),
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
