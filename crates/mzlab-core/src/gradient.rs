//! The LC gradient model and its parsers: `mzlab/analog.py` (`gradient_model`, `from_dam`, `parse_chromeleon`, `parse_gradient_csv`, `course_gradient`).
//! One model for every source: `{"source", "points": [{"t", "b", "flow"?, "curve"?}], "run_time", "nonlinear", "solvents"?}`.

use crate::error::{Error, Result};
use crate::json::{round, Out, Value};
use crate::methodinfo::Lc;

#[derive(Debug, Clone)]
pub struct Point {
    pub t: f64,
    pub b: f64,
    pub flow: Option<f64>,
    pub curve: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Gradient {
    pub source: &'static str,
    pub points: Vec<Point>,
    pub run_time: f64,
    pub nonlinear: bool,
    /// (solvent A, solvent B) as the script names them.
    pub solvents: Option<(Option<String>, Option<String>)>,
}

/// A point as the caller gives it (not checked yet): a number, a text or nothing for each field.
#[derive(Debug, Clone, Default)]
pub struct Raw {
    pub t: Option<f64>,
    pub b: Option<f64>,
    pub flow: Option<f64>,
    pub curve: Option<i64>,
    /// How the time and %B were written, for the error message.
    pub shown: (String, String),
}

/// `analog._num`: a number from a number or a text with a decimal comma.
pub fn num_text(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse().ok()
}

fn num_value(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Null => None,
        x => x.f64().or_else(|| x.str().and_then(num_text)),
    }
}

fn shown(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "None".into(),
        Some(x) => x.str().map(String::from).or_else(|| x.f64().map(|n| n.to_string())).unwrap_or_default(),
    }
}

impl Raw {
    /// A point of a model sent by the page (`{t, b, flow?, curve?}`).
    pub fn from_json(p: &Value) -> Raw {
        let blank = |v: Option<&Value>| matches!(v, None | Some(Value::Null)) || v.and_then(Value::str) == Some("");
        Raw {
            t: num_value(p.get("t")),
            b: num_value(p.get("b")),
            flow: if blank(p.get("flow")) { None } else { num_value(p.get("flow")) },
            curve: if blank(p.get("curve")) { None } else { p.get("curve").and_then(|c| c.f64().or_else(|| c.str().and_then(num_text))).map(|c| c as i64) },
            shown: (shown(p.get("t")), shown(p.get("b"))),
        }
    }
}

fn err(key: &'static str) -> Error {
    Error::new(key)
}

/// Checks and normalizes a gradient: points sorted by time (a step = two points with the same time), %B 0-100; at least two points.
pub fn model(source: &'static str, points: &[Raw], run_time: Option<f64>, solvents: Option<(Option<String>, Option<String>)>) -> Result<Gradient> {
    let mut pts = Vec::new();
    for p in points {
        let (Some(t), Some(b)) = (p.t, p.b) else {
            return Err(err("gradient.err.point").with("t", &p.shown.0).with("b", &p.shown.1));
        };
        if t < 0.0 || !(0.0..=100.0).contains(&b) {
            return Err(err("gradient.err.point").with("t", &p.shown.0).with("b", &p.shown.1));
        }
        pts.push(Point { t: round(t, 4), b: round(b, 4), flow: p.flow.map(|f| round(f, 4)), curve: p.curve });
    }
    if pts.len() < 2 {
        return Err(err("gradient.err.few"));
    }
    if pts.windows(2).any(|w| w[1].t < w[0].t) {
        return Err(err("gradient.err.order"));
    }
    let nonlinear = pts.iter().any(|p| p.curve.is_some_and(|c| c != 5));
    let run_time = run_time.unwrap_or(pts[pts.len() - 1].t);
    Ok(Gradient { source, points: pts, run_time, nonlinear, solvents })
}

/// The gradient a page sent (`{source?, points, run_time?, solvents?}`).
pub fn model_from_json(m: &Value) -> Result<Gradient> {
    let pts: Vec<Raw> = m.get("points").map(|a| a.arr().iter().map(Raw::from_json).collect()).unwrap_or_default();
    let name = |k: &str| m.get("solvents").and_then(|s| s.get(k)).and_then(Value::str).map(String::from);
    let sol = m.get("solvents").filter(|s| !s.is_null()).map(|_| (name("a"), name("b")));
    let source = match m.get("source").and_then(Value::str) {
        Some("dam") => "dam",
        Some("raw") => "raw",
        Some("csv") => "csv",
        Some("course") => "course",
        _ => "manual",
    };
    model(source, &pts, m.get("run_time").and_then(Value::f64), sol)
}

/// The gradient of an Analyst method (`from_dam`), `None` when it has fewer than two points.
pub fn from_dam(lc: Option<&Lc>) -> Result<Option<Gradient>> {
    let Some(lc) = lc.filter(|l| l.gradient.len() >= 2) else { return Ok(None) };
    let pts: Vec<Raw> = lc
        .gradient
        .iter()
        .map(|g| Raw { t: Some(g.t), b: g.b, flow: g.flow, curve: None, shown: (g.t.to_string(), g.b.map_or("None".into(), |b| b.to_string())) })
        .collect();
    model("dam", &pts, lc.run_time, None).map(Some)
}

impl Gradient {
    pub fn write(&self, o: &mut Out) {
        o.raw("{\"source\":").str(self.source).raw(",\"points\":[");
        for (i, p) in self.points.iter().enumerate() {
            o.raw(if i > 0 { ",{\"t\":" } else { "{\"t\":" }).num(p.t).raw(",\"b\":").num(p.b);
            if let Some(f) = p.flow {
                o.raw(",\"flow\":").num(f);
            }
            if let Some(c) = p.curve {
                o.raw(",\"curve\":").int(c);
            }
            o.raw("}");
        }
        o.raw("],\"run_time\":").num(self.run_time).raw(",\"nonlinear\":").raw(if self.nonlinear { "true" } else { "false" });
        if let Some((a, b)) = &self.solvents {
            let one = |o: &mut Out, v: &Option<String>| {
                match v {
                    Some(s) => o.str(s),
                    None => o.raw("null"),
                };
            };
            o.raw(",\"solvents\":{\"a\":");
            one(o, a);
            o.raw(",\"b\":");
            one(o, b);
            o.raw("}");
        }
        o.raw("}");
    }
}

const COURSE: [(f64, f64); 8] = [(0.0, 10.0), (2.0, 10.0), (12.0, 60.0), (13.0, 70.0), (15.0, 100.0), (17.0, 100.0), (17.1, 10.0), (20.0, 10.0)];

/// The gradient of the course: chosen by hand by the student (never a default of the program).
pub fn course() -> Result<Gradient> {
    let pts: Vec<Raw> = COURSE.iter().map(|&(t, b)| Raw { t: Some(t), b: Some(b), ..Default::default() }).collect();
    model("course", &pts, Some(20.0), None)
}

/// Printable text only: a script read from a binary file may carry NUL bytes and broken characters.
fn clean(text: &str) -> String {
    text.replace('\r', "\n").chars().map(|c| if (c == '\n' || c == '\t' || c >= ' ') && c != '\x7f' && c != '\u{fffd}' { c } else { ' ' }).collect()
}

/// Gradient from a CSV `time;%B[;flow]` (separator ; , or tab; decimal comma or point). A step is two rows with the same time.
pub fn parse_csv(text: &str) -> Result<Gradient> {
    let cleaned = clean(text);
    let body: Vec<&str> = cleaned.split('\n').filter(|l| !l.trim().is_empty()).collect();
    if body.is_empty() {
        return Err(err("gradient.err.empty"));
    }
    let sep = if text.contains(';') { ';' } else if text.contains('\t') { '\t' } else { ',' };
    let mut pts: Vec<Raw> = Vec::new();
    for (i, ln) in body.iter().enumerate() {
        let line = i + 1;
        let f: Vec<&str> = ln.split(sep).map(str::trim).collect();
        let (t, b) = if f.len() >= 2 { (num_text(f[0]), num_text(f[1])) } else { (None, None) };
        let (Some(t), Some(b)) = (t, b) else {
            if line == 1 && pts.is_empty() {
                continue;
            }
            return Err(err("gradient.err.row").with("line", line));
        };
        if !(0.0..=100.0).contains(&b) {
            return Err(err("gradient.err.bRange").with("line", line));
        }
        if pts.last().is_some_and(|p| t < p.t.unwrap_or(0.0)) {
            return Err(err("gradient.err.order").with("line", line));
        }
        let flow = if f.len() >= 3 { num_text(f[2]) } else { None };
        pts.push(Raw { t: Some(t), b: Some(b), flow, curve: None, shown: (t.to_string(), b.to_string()) });
    }
    if pts.len() < 2 {
        return Err(err("gradient.err.few"));
    }
    model("csv", &pts, None, None)
}
