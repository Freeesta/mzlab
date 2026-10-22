//! The rows of a validated chart (`grafico/1` with `data.rows`): the used fields only, at most 5 000 rows, reduced by the maximum of each interval per series so
//! the peaks stay and the same input gives the same bytes. Used by `chart` (`make`) and the chart-drawing path of the assistant.

use mzlab_core::error::Result;
use mzlab_core::json::Out;

use crate::chart::{err, validate_text, Col, Spec, View, MAX_ROWS};

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

/// The columns the chart uses, x first, without repeats.
fn used_fields(spec: &Spec, view: &View) -> Result<Vec<usize>> {
    let mut names: Vec<&str> = vec![&spec.x];
    for n in [&spec.y, &spec.color, &spec.detail, &spec.facet]
        .into_iter()
        .flatten()
    {
        if !names.contains(&n.as_str()) {
            names.push(n);
        }
    }
    names
        .iter()
        .map(|n| view.at(n).ok_or_else(|| err("field").with("field", n)))
        .collect()
}

/// The rows kept: rows sharing the nominal channels are a series and each series keeps its share of `MAX_ROWS`.
fn kept_rows(spec: &Spec, view: &View) -> Vec<usize> {
    let n = view.rows();
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
        .and_then(|c| match &view.cols[c] {
            Col::Num(v) => Some(v.clone()),
            Col::Text(_) => None,
        });
    let by = by.unwrap_or_else(|| vec![0.0; n]);
    let cap = (MAX_ROWS / groups.len().max(1)).max(1);
    groups.iter().flat_map(|g| peaks(g, &by, cap)).collect()
}

/// The `encoding`, `facet`, `log_y` and `why` of the chart.
fn tail(o: &mut Out, spec: &Spec) {
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
    let used = used_fields(spec, view)?;
    let kept = kept_rows(spec, view);
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
    let n = view.rows();
    o.raw("],\"source_rows\":")
        .int(n as i64)
        .raw(",\"reduced\":")
        .raw(if kept.len() < n { "true" } else { "false" })
        .raw("},\"mark\":")
        .str(&spec.mark);
    tail(&mut o, spec);
    Ok(o.0)
}

/// Validate and resolve in one step: the text of the chart for the page.
pub fn make(spec: &str, views: &[View]) -> Result<String> {
    resolve(&validate_text(spec, views)?, views)
}
