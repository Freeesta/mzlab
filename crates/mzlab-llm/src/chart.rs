//! Charts as a specification (`grafico/1`, docs/schemi/grafico-1.json). The model never writes data: it names a view that Rust builds from what the page
//! collected (`views`) and picks its fields; `validate` checks the specification and `resolve` fills `data.rows` (at most 5 000, reduced by the maximum of
//! each interval like the TIC of `contesto/1`, same bytes for the same input). Colours and styles belong to the page. Used by the `chart_validate`
//! family of the `llm` module and by `chart_preset` (the charts offered without a model).

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

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
    fn distinct(&self) -> usize {
        match self {
            Col::Num(v) => {
                let mut s: Vec<u64> = v.iter().map(|x| x.to_bits()).collect();
                s.sort_unstable();
                s.dedup();
                s.len()
            }
            Col::Text(v) => {
                let mut s: Vec<&String> = v.iter().collect();
                s.sort_unstable();
                s.dedup();
                s.len()
            }
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
    fn at(&self, name: &str) -> Option<usize> {
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

fn err(motive: &'static str) -> Error {
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

fn need(kinds: &[Kind], f: Option<(usize, &Field)>, role: &'static str, mark: &str) -> Result<()> {
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

/// Checks `spec` against the views offered now. A well-formed `{error:"no_view"}` is accepted and returned with `no_view` set.
pub fn validate(spec: &Value, views: &[View]) -> Result<Spec> {
    top_level(spec)?;
    if spec.get("schema").and_then(Value::str) != Some("grafico/1") {
        return Err(err("schema").with("field", "schema"));
    }
    let why = spec
        .get("why")
        .and_then(Value::str)
        .unwrap_or("")
        .trim()
        .to_string();
    if why.is_empty() || why.chars().count() > MAX_WHY {
        return Err(err("why").with("max", MAX_WHY));
    }
    if let Some(e) = spec.get("error") {
        return if e.str() == Some("no_view")
            && spec.get("title").is_none()
            && spec.get("data").is_none()
        {
            Ok(Spec {
                why,
                no_view: true,
                ..Spec::default()
            })
        } else {
            Err(err("schema").with("field", "error"))
        };
    }
    let title = spec
        .get("title")
        .and_then(Value::str)
        .unwrap_or("")
        .trim()
        .to_string();
    if title.is_empty() || title.chars().count() > MAX_TITLE {
        return Err(err("title").with("max", MAX_TITLE));
    }
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
    let enc = spec
        .get("encoding")
        .ok_or_else(|| err("schema").with("field", "encoding"))?;
    if let Value::Obj(o) = enc {
        if let Some((k, _)) = o
            .iter()
            .find(|(k, _)| !["x", "y", "color", "detail"].contains(&k.as_str()))
        {
            return Err(err("style").with("field", format!("encoding.{k}")));
        }
    } else {
        return Err(err("schema").with("field", "encoding"));
    }
    let (x, y, color, detail) = (
        channel(enc, "x")?,
        channel(enc, "y")?,
        channel(enc, "color")?,
        channel(enc, "detail")?,
    );
    let facet = match spec.get("facet") {
        None => None,
        Some(f) => Some(
            f.str()
                .map(str::to_string)
                .ok_or_else(|| err("schema").with("field", "facet"))?,
        ),
    };
    let (fx, fy, fc, fd, ff) = (
        field_of(view, &x, "x")?,
        field_of(view, &y, "y")?,
        field_of(view, &color, "color")?,
        field_of(view, &detail, "detail")?,
        field_of(view, &facet, "facet")?,
    );
    use Kind::{Nominal as N, Ordinal as O, Quantitative as Q};
    match mark {
        "line" | "point" | "stick" => {
            need(&[Q, O], fx, "x", mark)?;
            need(&[Q], fy, "y", mark)?;
        }
        "bar" => {
            need(&[N, O], fx, "x", mark)?;
            need(&[Q], fy, "y", mark)?;
        }
        "box" => {
            need(&[N], fx, "x", mark)?;
            need(&[Q], fy, "y", mark)?;
        }
        "hist" => {
            need(&[Q], fx, "x", mark)?;
            if fy.is_some() {
                return Err(err("type")
                    .with("channel", "y")
                    .with("mark", mark)
                    .with("kind", "unexpected"));
            }
        }
        _ => {
            need(&[N], fx, "x", mark)?;
            need(&[N], fy, "y", mark)?;
            need(&[Q], fc, "color", mark)?;
        }
    }
    // for a heat map the colour channel carries the quantity; everywhere else it names series
    if mark != "heat" {
        if let Some((i, f)) = fc {
            if f.kind != N || view.cols[i].distinct() > MAX_COLORS {
                return Err(err("color").with("field", f.name).with("max", MAX_COLORS));
            }
        }
    }
    if let Some((i, f)) = fd {
        if f.kind != N || view.cols[i].distinct() > MAX_DETAIL {
            return Err(err("detail").with("field", f.name).with("max", MAX_DETAIL));
        }
    }
    if let Some((i, f)) = ff {
        if f.kind != N || view.cols[i].distinct() > MAX_FACETS {
            return Err(err("facet").with("field", f.name).with("max", MAX_FACETS));
        }
    }
    let log_y = match spec.get("log_y") {
        None => false,
        Some(v) => v
            .bool()
            .ok_or_else(|| err("schema").with("field", "log_y"))?,
    };
    if log_y && fy.is_none() {
        return Err(err("type")
            .with("channel", "y")
            .with("mark", mark)
            .with("kind", "missing"));
    }
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

/// Keeps at most `cap` of `idx` (indices into `y`): the row with the largest `y` of each interval, in the original order.
fn peaks(idx: &[usize], y: &[f64], cap: usize) -> Vec<usize> {
    if idx.len() <= cap {
        return idx.to_vec();
    }
    let step = idx.len().div_ceil(cap).max(1);
    idx.chunks(step)
        .map(|c| {
            c.iter()
                .copied()
                .fold(c[0], |b, j| if y[j] > y[b] { j } else { b })
        })
        .collect()
}

fn cell(o: &mut Out, c: &Col, i: usize) {
    match c {
        Col::Num(v) => {
            o.num(v[i]);
        }
        Col::Text(v) => {
            o.str(&v[i]);
        }
    }
}

/// The chart with its data: `data.rows` as arrays in the order of `data.fields` (only the fields the chart uses), reduced to `MAX_ROWS`.
pub fn resolve(spec: &Spec, views: &[View]) -> Result<String> {
    let mut o = Out::default();
    if spec.no_view {
        o.raw("{\"schema\":\"grafico/1\",\"error\":\"no_view\",\"why\":")
            .str(&spec.why)
            .raw("}");
        return Ok(o.0);
    }
    let view = views
        .iter()
        .find(|v| v.id == spec.view)
        .ok_or_else(|| err("ref").with("ref", &spec.view))?;
    let mut names: Vec<&str> = vec![&spec.x];
    for n in [&spec.y, &spec.color, &spec.detail, &spec.facet]
        .into_iter()
        .flatten()
    {
        if !names.contains(&n.as_str()) {
            names.push(n);
        }
    }
    let used: Vec<usize> = names
        .iter()
        .map(|n| view.at(n).ok_or_else(|| err("field").with("field", n)))
        .collect::<Result<_>>()?;
    let n = view.rows();
    // series = rows that share the nominal channels; each keeps its share of the cap
    let keys: Vec<usize> = [&spec.color, &spec.detail, &spec.facet]
        .into_iter()
        .flatten()
        .filter_map(|k| view.at(k))
        .collect();
    let nominal = |c: usize, i: usize| match &view.cols[c] {
        Col::Text(v) => v[i].clone(),
        Col::Num(v) => v[i].to_string(),
    };
    let mut order: Vec<Vec<String>> = Vec::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for i in 0..n {
        let key: Vec<String> = keys.iter().map(|&c| nominal(c, i)).collect();
        match order.iter().position(|k| *k == key) {
            Some(g) => groups[g].push(i),
            None => {
                order.push(key);
                groups.push(vec![i]);
            }
        }
    }
    let by = spec
        .y
        .as_ref()
        .or(Some(&spec.x))
        .and_then(|f| view.at(f))
        .map(|c| match &view.cols[c] {
            Col::Num(v) => v.clone(),
            Col::Text(_) => vec![0.0; n],
        });
    let by = by.unwrap_or_else(|| vec![0.0; n]);
    let cap = (MAX_ROWS / groups.len().max(1)).max(1);
    let kept: Vec<usize> = groups.iter().flat_map(|g| peaks(g, &by, cap)).collect();
    o.raw("{\"schema\":\"grafico/1\",\"title\":")
        .str(&spec.title)
        .raw(",\"data\":{\"ref\":")
        .str(&view.id)
        .raw(",\"fields\":[");
    for (k, &c) in used.iter().enumerate() {
        let f = &view.fields[c];
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(f.name)
            .raw(",\"type\":")
            .str(f.kind.name())
            .raw(",\"unit\":")
            .str(f.unit)
            .raw("}");
    }
    o.raw("],\"rows\":[");
    for (k, &i) in kept.iter().enumerate() {
        o.raw(if k > 0 { ",[" } else { "[" });
        for (j, &c) in used.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" });
            cell(&mut o, &view.cols[c], i);
        }
        o.raw("]");
    }
    o.raw("],\"source_rows\":")
        .int(n as i64)
        .raw(",\"reduced\":")
        .raw(if kept.len() < n { "true" } else { "false" })
        .raw("},\"mark\":")
        .str(&spec.mark);
    o.raw(",\"encoding\":{\"x\":{\"field\":")
        .str(&spec.x)
        .raw("}");
    for (k, v) in [
        ("y", &spec.y),
        ("color", &spec.color),
        ("detail", &spec.detail),
    ] {
        if let Some(f) = v {
            o.raw(",").str(k).raw(":{\"field\":").str(f).raw("}");
        }
    }
    o.raw("}");
    if let Some(f) = &spec.facet {
        o.raw(",\"facet\":").str(f);
    }
    if spec.log_y {
        o.raw(",\"log_y\":true");
    }
    o.raw(",\"why\":").str(&spec.why).raw("}");
    Ok(o.0)
}

/// Validate and resolve in one step: the text of the chart for the page.
pub fn make(spec: &str, views: &[View]) -> Result<String> {
    resolve(&validate_text(spec, views)?, views)
}
