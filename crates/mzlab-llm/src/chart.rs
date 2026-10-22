//! Charts as a specification (`grafico/1`, docs/schemi/grafico-1.json). The model never writes data: it names a view that Rust builds from what the page
//! collected (`views`) and picks its fields; `validate` checks the specification and `resolve` fills `data.rows` (at most 5 000, reduced by the maximum of
//! each interval like the TIC of `contesto/1`, same bytes for the same input). Colours and styles belong to the page. Used by the `chart_validate`
//! family of the `llm` module and by `chart_preset` (the charts offered without a model).

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

/// The most rows a chart carries.
pub const MAX_ROWS: usize = 5000;
/// The longest title, in characters.
pub const MAX_TITLE: usize = 60;
/// The most values of the colour channel.
pub const MAX_COLORS: usize = 8;
/// The most panels of a faceted chart.
pub const MAX_FACETS: usize = 12;
const MAX_DETAIL: usize = 50;
const MAX_WHY: usize = 300;
const MARKS: [&str; 7] = ["line", "point", "bar", "stick", "box", "hist", "heat"];
const STYLE: [&str; 14] = [
    "colors",
    "palette",
    "font",
    "font_size",
    "size",
    "style",
    "theme",
    "width",
    "height",
    "stroke",
    "fill",
    "opacity",
    "scale",
    "layer",
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Quantitative,
    Ordinal,
    Nominal,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Quantitative => "quantitative",
            Kind::Ordinal => "ordinal",
            Kind::Nominal => "nominal",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: &'static str,
    pub kind: Kind,
    pub unit: &'static str,
}

#[derive(Clone, Debug)]
pub enum Col {
    Num(Vec<f64>),
    Text(Vec<String>),
}

impl Col {
    fn len(&self) -> usize {
        match self {
            Col::Num(v) => v.len(),
            Col::Text(v) => v.len(),
        }
    }
    /// Whether more than `max` different values occur (stops as soon as it does, with no sorting).
    fn more_than(&self, max: usize) -> bool {
        fn count<T: PartialEq + Copy>(v: impl Iterator<Item = T>, max: usize) -> bool {
            let mut seen: Vec<T> = Vec::new();
            for x in v {
                if !seen.contains(&x) {
                    seen.push(x);
                    if seen.len() > max {
                        return true;
                    }
                }
            }
            false
        }
        match self {
            Col::Num(v) => count(v.iter().map(|x| x.to_bits()), max),
            Col::Text(v) => count(v.iter().map(String::as_str), max),
        }
    }
}

/// A named view: typed fields and one column per field, all of the same length.
#[derive(Clone, Debug)]
pub struct View {
    pub id: String,
    pub description: String,
    pub fields: Vec<Field>,
    pub cols: Vec<Col>,
}

impl View {
    pub fn rows(&self) -> usize {
        self.cols.first().map_or(0, Col::len)
    }
    pub(crate) fn at(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }
}

/// A specification that passed `validate`; `error` is the `no_view` answer.
#[derive(Clone, Debug, Default)]
pub struct Spec {
    pub title: String,
    pub view: String,
    pub mark: String,
    pub x: String,
    pub y: Option<String>,
    pub color: Option<String>,
    pub detail: Option<String>,
    pub facet: Option<String>,
    pub log_y: bool,
    pub why: String,
    pub no_view: bool,
}

pub(crate) fn err(motive: &'static str) -> Error {
    Error::new(match motive {
        "schema" => "err.llm.chart.schema",
        "title" => "err.llm.chart.title",
        "ref" => "err.llm.chart.ref",
        "mark" => "err.llm.chart.mark",
        "field" => "err.llm.chart.field",
        "type" => "err.llm.chart.type",
        "color" => "err.llm.chart.color",
        "detail" => "err.llm.chart.detail",
        "facet" => "err.llm.chart.facet",
        "style" => "err.llm.chart.style",
        "rows" => "err.llm.chart.rows",
        "why" => "err.llm.chart.why",
        _ => "err.llm.chart.schema",
    })
}

/// The `field` name of a channel (`{field}`), or an error when the channel carries anything else (a style, a scale).
fn channel(enc: &Value, key: &str) -> Result<Option<String>> {
    let Some(c) = enc.get(key) else {
        return Ok(None);
    };
    let Value::Obj(o) = c else {
        return Err(err("schema").with("field", key));
    };
    for (k, _) in o {
        if k != "field" {
            return Err(err("style").with("field", format!("{key}.{k}")));
        }
    }
    match c.get("field").and_then(Value::str) {
        Some(f) if !f.is_empty() => Ok(Some(f.to_string())),
        _ => Err(err("schema").with("field", key)),
    }
}

fn top_level(spec: &Value) -> Result<()> {
    let Value::Obj(o) = spec else {
        return Err(err("schema").with("field", "spec"));
    };
    for (k, _) in o {
        match k.as_str() {
            "schema" | "title" | "data" | "mark" | "encoding" | "facet" | "log_y" | "why"
            | "error" => {}
            k if STYLE.contains(&k) => return Err(err("style").with("field", k)),
            k => return Err(err("schema").with("field", k)),
        }
    }
    Ok(())
}

type Chan<'a> = Option<(usize, &'a Field)>;

fn field_of<'a>(
    v: &'a View,
    name: &Option<String>,
    role: &'static str,
) -> Result<Option<(usize, &'a Field)>> {
    let Some(n) = name else { return Ok(None) };
    v.at(n).map(|i| Some((i, &v.fields[i]))).ok_or_else(|| {
        err("field")
            .with("field", n)
            .with("channel", role)
            .with("ref", &v.id)
    })
}

fn need(kinds: &[Kind], f: Chan, role: &'static str, mark: &str) -> Result<()> {
    match f {
        Some((_, f)) if kinds.contains(&f.kind) => Ok(()),
        Some((_, f)) => Err(err("type")
            .with("channel", role)
            .with("field", f.name)
            .with("kind", f.kind.name())
            .with("mark", mark)),
        None => Err(err("type")
            .with("channel", role)
            .with("mark", mark)
            .with("kind", "missing")),
    }
}

/// The kinds of fields each mark takes; for a heat map the colour channel carries the quantity.
fn check_types(mark: &str, fx: Chan, fy: Chan, fc: Chan) -> Result<()> {
    use Kind::{Nominal as N, Ordinal as O, Quantitative as Q};
    match mark {
        "line" | "point" | "stick" => {
            need(&[Q, O], fx, "x", mark)?;
            need(&[Q], fy, "y", mark)
        }
        "bar" => {
            need(&[N, O], fx, "x", mark)?;
            need(&[Q], fy, "y", mark)
        }
        "box" => {
            need(&[N], fx, "x", mark)?;
            need(&[Q], fy, "y", mark)
        }
        "hist" if fy.is_some() => Err(err("type")
            .with("channel", "y")
            .with("mark", mark)
            .with("kind", "unexpected")),
        "hist" => need(&[Q], fx, "x", mark),
        _ => {
            need(&[N], fx, "x", mark)?;
            need(&[N], fy, "y", mark)?;
            need(&[Q], fc, "color", mark)
        }
    }
}

/// Nominal channels with few enough values: colour at most 8, detail at most 50, facet at most 12.
fn check_limits(view: &View, mark: &str, fc: Chan, fd: Chan, ff: Chan) -> Result<()> {
    let one = |c: Chan, motive: &'static str, max: usize| match c {
        Some((i, f)) if f.kind != Kind::Nominal || view.cols[i].more_than(max) => {
            Err(err(motive).with("field", f.name).with("max", max))
        }
        _ => Ok(()),
    };
    if mark != "heat" {
        one(fc, "color", MAX_COLORS)?;
    }
    one(fd, "detail", MAX_DETAIL)?;
    one(ff, "facet", MAX_FACETS)
}

/// A required text of at most `max` characters (`why`, `title`).
fn text_of(spec: &Value, key: &str, max: usize, motive: &'static str) -> Result<String> {
    let t = spec.get(key).and_then(Value::str).unwrap_or("").trim();
    if t.is_empty() || t.chars().count() > max {
        return Err(err(motive).with("max", max));
    }
    Ok(t.to_string())
}

/// The four channels of `encoding` as field names; a channel with anything but `field` (a scale, a colour) is a style.
fn channels(spec: &Value) -> Result<[Option<String>; 4]> {
    let enc = spec
        .get("encoding")
        .ok_or_else(|| err("schema").with("field", "encoding"))?;
    let Value::Obj(o) = enc else {
        return Err(err("schema").with("field", "encoding"));
    };
    if let Some((k, _)) = o
        .iter()
        .find(|(k, _)| !["x", "y", "color", "detail"].contains(&k.as_str()))
    {
        return Err(err("style").with("field", format!("encoding.{k}")));
    }
    Ok([
        channel(enc, "x")?,
        channel(enc, "y")?,
        channel(enc, "color")?,
        channel(enc, "detail")?,
    ])
}

/// `log_y`: a boolean, and only for a chart with a `y`.
fn log_flag(spec: &Value, mark: &str, fy: Chan) -> Result<bool> {
    let on = spec
        .get("log_y")
        .map(|v| v.bool().ok_or_else(|| err("schema").with("field", "log_y")))
        .transpose()?
        .unwrap_or(false);
    if on && fy.is_none() {
        return Err(err("type")
            .with("channel", "y")
            .with("mark", mark)
            .with("kind", "missing"));
    }
    Ok(on)
}

/// Checks `spec` against the views offered now. A well-formed `{error:"no_view"}` is accepted and returned with `no_view` set.
pub fn validate(spec: &Value, views: &[View]) -> Result<Spec> {
    top_level(spec)?;
    if spec.get("schema").and_then(Value::str) != Some("grafico/1") {
        return Err(err("schema").with("field", "schema"));
    }
    let why = text_of(spec, "why", MAX_WHY, "why")?;
    if let Some(e) = spec.get("error") {
        let bare = spec.get("title").is_none() && spec.get("data").is_none();
        return if e.str() == Some("no_view") && bare {
            Ok(Spec {
                why,
                no_view: true,
                ..Spec::default()
            })
        } else {
            Err(err("schema").with("field", "error"))
        };
    }
    let title = text_of(spec, "title", MAX_TITLE, "title")?;
    let data = spec
        .get("data")
        .ok_or_else(|| err("schema").with("field", "data"))?;
    if data.get("rows").is_some() {
        return Err(err("rows"));
    }
    let r = data.get("ref").and_then(Value::str).unwrap_or("");
    let view = views
        .iter()
        .find(|v| v.id == r)
        .ok_or_else(|| err("ref").with("ref", r))?;
    let mark = spec.get("mark").and_then(Value::str).unwrap_or("");
    if !MARKS.contains(&mark) {
        return Err(err("mark").with("mark", mark));
    }
    let [x, y, color, detail] = channels(spec)?;
    let facet = spec
        .get("facet")
        .map(|f| {
            f.str()
                .map(str::to_string)
                .ok_or_else(|| err("schema").with("field", "facet"))
        })
        .transpose()?;
    let (fx, fy, fc) = (
        field_of(view, &x, "x")?,
        field_of(view, &y, "y")?,
        field_of(view, &color, "color")?,
    );
    let (fd, ff) = (
        field_of(view, &detail, "detail")?,
        field_of(view, &facet, "facet")?,
    );
    check_types(mark, fx, fy, fc)?;
    check_limits(view, mark, fc, fd, ff)?;
    let log_y = log_flag(spec, mark, fy)?;
    Ok(Spec {
        title,
        view: view.id.clone(),
        mark: mark.into(),
        x: x.unwrap_or_default(),
        y,
        color,
        detail,
        facet,
        log_y,
        why,
        no_view: false,
    })
}

/// `validate` from JSON text.
pub fn validate_text(spec: &str, views: &[View]) -> Result<Spec> {
    let v = parse(spec.as_bytes()).map_err(|_| err("schema").with("field", "spec"))?;
    validate(&v, views)
}

/// `make` and `resolve` live in `chart_data`.
pub use crate::chart_data::{make, resolve};
