//! The `--traces PATH...` of `peak table`: the JSON list of samples the page sends to the engine, passed on byte for byte, or the answers of
//! `trace xic` / `trace chrom` (`trace-set/1`, `trace/1`, one sample per document, one channel per trace), so the XICs of a target list in
//! many full-scan files are integrated in one call. Any other shape is refused: an empty table must never come from a misread file.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

fn list(v: &Value) -> Option<&[Value]> {
    match v {
        Value::Arr(a) => Some(a),
        _ => None,
    }
}

fn shape(path: &str) -> Error {
    Error::new("err.cli.usage").with(
        "message",
        format!("--traces {path}: not a JSON list of samples, a trace-set/1 or a trace/1 (the output of trace xic or trace chrom)"),
    )
}

/// One `trace/1` as a transition of the list: its points, its id as the name, the m/z of an XIC as its Q1 (the channel of the table).
fn transition(o: &mut Out, t: &Value) {
    o.raw("{\"rt\":[");
    let pts = t.get("points").map_or(&[][..], Value::arr);
    for (i, p) in pts.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw(
            &p.arr()
                .first()
                .and_then(Value::f64)
                .unwrap_or(0.0)
                .to_string(),
        );
    }
    o.raw("],\"y\":[");
    for (i, p) in pts.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw(
            &p.arr()
                .get(1)
                .and_then(Value::f64)
                .unwrap_or(0.0)
                .to_string(),
        );
    }
    o.raw("],\"name\":");
    o.str(t.get("id").and_then(Value::str).unwrap_or(""));
    if let Some(mz) = t
        .get("evidence")
        .and_then(|e| e.get("mz"))
        .and_then(Value::f64)
    {
        o.raw(",\"q1\":").raw(&mz.to_string());
    }
    o.raw("}");
}

/// The sample of one `trace-set/1` or `trace/1` document, named by its file (and sample of a multi-sample file).
fn sample(o: &mut Out, d: &Value, path: &str) -> Result<()> {
    let schema = d.get("schema").and_then(Value::str).unwrap_or("");
    let traces: Vec<&Value> = match schema {
        "trace-set/1" => d.get("traces").map_or(&[][..], Value::arr).iter().collect(),
        "trace/1" => vec![d],
        _ => return Err(shape(path)),
    };
    let file = d.get("file").and_then(Value::str).unwrap_or(path);
    // the sample of a multi-sample file is a number (`trace xic --sample 2`), null for a file of one
    let name = match d.get("sample") {
        Some(Value::Str(s)) => format!("{file}#{s}"),
        Some(Value::Num(n)) => format!("{file}#{n}"),
        _ => file.to_string(),
    };
    o.raw("{\"name\":");
    o.str(&name);
    o.raw(",\"transitions\":[");
    for (i, t) in traces.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        transition(o, t);
    }
    o.raw("]}");
    Ok(())
}

/// The list of samples for the engine from the `--traces` files, in their order.
pub(crate) fn read(paths: &[std::path::PathBuf]) -> Result<String> {
    let mut texts = Vec::new();
    for p in paths {
        let t = std::fs::read_to_string(p).map_err(|e| {
            Error::new("err.cli.file").with("path", format!("{}: {e}", p.display()))
        })?;
        texts.push((p.display().to_string(), t));
    }
    if let [(path, t)] = texts.as_slice() {
        let v = parse(t.as_bytes()).map_err(|_| shape(path))?;
        // the page's own list (samples with their transitions), unchanged: the same bytes as before
        if list(&v).is_some_and(|a| {
            !a.is_empty()
                && a.iter()
                    .all(|s| s.get("schema").is_none() && s.get("transitions").is_some())
        }) {
            return Ok(t.clone());
        }
        if v.get("schema").is_none() && list(&v).is_none() {
            return Err(shape(path));
        }
    }
    let mut o = Out::default();
    o.raw("[");
    let mut first = true;
    for (path, t) in &texts {
        let v = parse(t.as_bytes()).map_err(|_| shape(path))?;
        let docs: Vec<&Value> = match list(&v) {
            Some(a) => a.iter().collect(),
            None => vec![&v],
        };
        for d in docs {
            o.raw(if first { "" } else { "," });
            first = false;
            sample(&mut o, d, path)?;
        }
    }
    // `[]` (or files with no document) would be an empty table
    if first {
        return Err(shape(texts.first().map_or("", |t| t.0.as_str())));
    }
    o.raw("]");
    Ok(o.0)
}

#[cfg(test)]
#[path = "traces_in_tests.rs"]
mod tests;
