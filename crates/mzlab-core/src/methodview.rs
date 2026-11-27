//! What the Method window shows of the LC program and the diode array, as display-ready text: the one place that rounds numbers for display (`shown`: two
//! decimals, counts as integers; the full values stay next to them), the pump program held to the end of the run, and for every value where it comes from
//! (`read`: a stream and field of the file; `inferred`: computed from values read; no entry: the reader names no source). A pure function of `method/1`;
//! `filemethod.rs` writes it into `file_method/1`, so site, app and CLI give the same bytes.

use crate::json::{Out, Value};

/// Where a value of the LC method comes from: `read` (a stream and field of the file, named in `at`) or `inferred` (computed from values read; `at` says how).
#[derive(Debug, Clone)]
pub struct Src {
    pub kind: &'static str,
    pub at: String,
}

impl Src {
    pub fn read(at: &str) -> Self {
        Src {
            kind: "read",
            at: at.into(),
        }
    }
    pub fn inferred(at: &str) -> Self {
        Src {
            kind: "inferred",
            at: at.into(),
        }
    }
}

/// A number for display: two decimals, never `-0.00`. The same rule for every number of the Method window; the document keeps the full value beside it.
pub fn shown(v: f64) -> String {
    let s = format!("{v:.2}");
    if s == "-0.00" {
        "0.00".into()
    } else {
        s
    }
}

/// A count (points, channels) for display: an integer.
fn count(v: f64) -> String {
    format!("{v:.0}")
}

fn num_or_null(o: &mut Out, v: Option<f64>) {
    match v.filter(|x| x.is_finite()) {
        Some(x) => o.num(x),
        None => o.raw("null"),
    };
}

fn shown_or_null(o: &mut Out, v: Option<f64>) {
    match v.filter(|x| x.is_finite()) {
        Some(x) => o.str(&shown(x)),
        None => o.raw("null"),
    };
}

fn src(o: &mut Out, kind: &str, at: &str) {
    o.raw("{\"kind\":")
        .str(kind)
        .raw(",\"at\":")
        .str(at)
        .raw("}");
}

/// The source the reader recorded for `key` in `holder.sources` (`{kind, at}`), `null` when it names none.
fn named(holder: Option<&Value>, key: &str) -> Option<(String, String)> {
    let s = holder?.get("sources")?.get(key)?;
    Some((
        s.get("kind")?.str()?.to_string(),
        s.get("at")?.str()?.to_string(),
    ))
}

fn write_named(o: &mut Out, s: Option<(String, String)>) {
    match s {
        Some((k, a)) => src(o, &k, &a),
        None => {
            o.raw("null");
        }
    }
}

/// The last %B held to the end of the run: drawn, never added to the points; `null` when the program already ends at the run time.
fn write_hold(o: &mut Out, last: Option<&(f64, Option<f64>, Option<f64>)>, run: Option<f64>) {
    match (last, run) {
        (Some((t, Some(b), f)), Some(r)) if r > *t => {
            o.raw("{\"from_min\":")
                .num(*t)
                .raw(",\"to_min\":")
                .num(r)
                .raw(",\"B_pct\":")
                .num(*b)
                .raw(",\"flow_mL_min\":");
            num_or_null(o, *f);
            o.raw(",\"shown\":{\"from\":")
                .str(&shown(*t))
                .raw(",\"to\":")
                .str(&shown(r))
                .raw(",\"a\":")
                .str(&shown(100.0 - *b))
                .raw(",\"b\":")
                .str(&shown(*b));
            if let Some(f) = f.filter(|x| x.is_finite()) {
                o.raw(",\"flow\":").str(&shown(f));
            }
            o.raw("}}");
        }
        _ => {
            o.raw("null");
        }
    }
}

/// `"lc":{…}` of `file_method/1`, `null` when the method has no pump program, flow, oven or run time.
pub fn write_lc(o: &mut Out, doc: Option<&Value>) {
    let Some(lc) = doc.and_then(|d| d.get("lc")) else {
        o.raw("null");
        return;
    };
    let pts: Vec<(f64, Option<f64>, Option<f64>)> = lc
        .get("gradient")
        .map(Value::arr)
        .unwrap_or_default()
        .iter()
        .filter_map(|p| {
            Some((
                p.get("t_min")?.f64()?,
                p.get("B_pct").and_then(Value::f64),
                p.get("flow_mL_min").and_then(Value::f64),
            ))
        })
        .collect();
    let run = lc.get("duration_min").and_then(Value::f64);
    let oven = lc.get("oven_C").and_then(Value::f64);
    let flow = lc.get("flow_mL_min").and_then(Value::f64);
    o.raw("{\"run_time_min\":");
    num_or_null(o, run);
    o.raw(",\"run_time_shown\":");
    shown_or_null(o, run);
    o.raw(",\"gradient\":[");
    for (i, (t, b, f)) in pts.iter().enumerate() {
        o.raw(if i > 0 { ",{\"t_min\":" } else { "{\"t_min\":" })
            .num(*t);
        o.raw(",\"B_pct\":");
        num_or_null(o, *b);
        o.raw(",\"flow_mL_min\":");
        num_or_null(o, *f);
        o.raw(",\"shown\":{\"t\":").str(&shown(*t)).raw(",\"a\":");
        shown_or_null(o, b.map(|b| 100.0 - b));
        o.raw(",\"b\":");
        shown_or_null(o, *b);
        o.raw(",\"flow\":");
        shown_or_null(o, *f);
        o.raw("}}");
    }
    o.raw("],\"hold\":");
    write_hold(o, pts.last(), run);
    o.raw(",\"flow_mL_min\":");
    num_or_null(o, flow);
    o.raw(",\"flow_shown\":");
    shown_or_null(o, flow);
    o.raw(",\"oven_C\":");
    num_or_null(o, oven);
    o.raw(",\"oven_shown\":");
    shown_or_null(o, oven);
    write_eluents_and_sources(o, lc);
}

/// `"eluents":[…],"sources":{…}` of the LC view: the eluents exist only when the file names them (Thermo script); nothing is ever filled in.
fn write_eluents_and_sources(o: &mut Out, lc: &Value) {
    // the eluents exist only when the file names them (Thermo script); nothing is ever filled in
    o.raw(",\"eluents\":[");
    for (i, s) in lc
        .get("solvents")
        .map(Value::arr)
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"channel\":")
            .str(s.get("channel").and_then(Value::str).unwrap_or(""))
            .raw(",\"name\":")
            .str(s.get("name").and_then(Value::str).unwrap_or(""))
            .raw("}");
    }
    o.raw("],\"sources\":{\"eluents\":");
    write_named(o, named(Some(lc), "solvents"));
    o.raw(",\"gradient\":");
    write_named(o, named(Some(lc), "gradient"));
    o.raw(",\"run_time\":");
    write_named(o, named(Some(lc), "run_time"));
    o.raw(",\"flow\":");
    write_named(o, named(Some(lc), "flow"));
    o.raw(",\"oven\":");
    write_named(o, named(Some(lc), "oven"));
    o.raw(",\"a_pct\":");
    src(o, "inferred", "100 - %B (a two-eluent program)");
    o.raw(",\"hold\":");
    src(
        o,
        "inferred",
        "no pump event after the last point: %B and flow stay as at that point until the run time",
    );
    o.raw("}}");
}

/// `head` (the `file_method/1` document without its views, `filemethod::document_head`) with `"lc":…,"dad":…` before its closing brace; `method` is the
/// `method/1` text the head was made from (`None`: the file carries none).
pub fn attach(head: &str, method: Option<&str>) -> crate::error::Result<String> {
    let doc = method.map(|m| crate::json::parse(m.as_bytes())).transpose()?;
    let body = head.strip_suffix('}').unwrap_or(head);
    let mut o = Out::default();
    o.raw(body).raw(",\"lc\":");
    write_lc(&mut o, doc.as_ref());
    o.raw(",\"dad\":");
    write_dad(&mut o, doc.as_ref());
    o.raw("}");
    Ok(o.0)
}

/// `"view":{"lc":…,"dad":…}` of the LC method of a `.dam` (the same two views `file_method/1` writes for a `method/1` document), for the Method window of a file whose
/// method comes from a loaded `.dam`.
pub fn write_view_of(o: &mut Out, lc: &crate::methodinfo::Lc) {
    let (l, det) = crate::method::lc_sections(lc);
    let part = |k: &str, v: Option<crate::method::V>| {
        v.map_or(String::new(), |v| format!("\"{k}\":{}", v.json()))
    };
    let doc = format!(
        "{{{}}}",
        [part("lc", l), part("detectors", det)]
            .iter()
            .filter(|p| !p.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    let parsed = crate::json::parse(doc.as_bytes()).ok();
    o.raw("{\"lc\":");
    write_lc(o, parsed.as_ref());
    o.raw(",\"dad\":");
    write_dad(o, parsed.as_ref());
    o.raw("}");
}

/// The DAD header fields of the sample: (id, value, unit, stream path, field key), only the keys the header decoder writes.
fn header_rows(doc: &Value) -> Vec<(&'static str, f64, String, String)> {
    const IDS: [(&str, &str); 6] = [
        ("points", "points"),
        ("channels", "channels"),
        ("duration", "duration"),
        ("wavelength_min", "wl_min"),
        ("wavelength_max", "wl_max"),
        ("step", "step"),
    ];
    let mut out = Vec::new();
    for f in doc.get("fields").map(Value::arr).unwrap_or_default() {
        let (Some(origin), Some(key)) = (
            f.get("origin").and_then(Value::str),
            f.get("key").and_then(Value::str),
        ) else {
            continue;
        };
        if !origin.ends_with("/DAD/DADSampleHeader") {
            continue;
        }
        if let (Some((_, id)), Some(v)) = (
            IDS.iter().find(|(k, _)| *k == key),
            f.get("value").and_then(Value::f64),
        ) {
            out.push((*id, v, origin.to_string(), key.to_string()));
        }
    }
    out
}

/// `"dad":{"rows":[…]}`: the diode array as labelled rows, `null` when the file says nothing about it. A row: `id` (the catalog key `method.dad.<id>`), `value`,
/// `shown`, `unit`, `source`. What the file's DAD header says (acquired) goes first; the detector of the method (programmed) adds what the header lacks.
pub fn write_dad(o: &mut Out, doc: Option<&Value>) {
    let Some(doc) = doc else {
        o.raw("null");
        return;
    };
    let head = header_rows(doc);
    let mut rows = rows_of_header(&head);
    let det = doc
        .get("detectors")
        .map(Value::arr)
        .unwrap_or_default()
        .iter()
        .find(|d| d.get("type").and_then(Value::str) == Some("PDA"));
    if let Some(d) = det {
        rows_of_detector(d, &mut rows, head.iter().any(|h| h.0 == "wl_min"));
    }
    if rows.is_empty() {
        o.raw("null");
        return;
    }
    o.raw("{\"rows\":[");
    for (i, r) in rows.iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .str(r.id)
            .raw(",\"value\":");
        num_or_null(o, r.value);
        o.raw(",\"shown\":")
            .str(&r.text)
            .raw(",\"unit\":")
            .str(r.unit)
            .raw(",\"source\":");
        write_named(o, r.source.clone());
        o.raw("}");
    }
    o.raw("]}");
}

/// One labelled row of the diode array: `id` is the catalog key `method.dad.<id>`.
struct Row {
    id: &'static str,
    value: Option<f64>,
    text: String,
    unit: &'static str,
    source: Option<(String, String)>,
}

fn row(
    id: &'static str,
    value: Option<f64>,
    text: String,
    unit: &'static str,
    source: Option<(String, String)>,
) -> Row {
    Row {
        id,
        value,
        text,
        unit,
        source,
    }
}

type Head = (&'static str, f64, String, String);

/// The rows the file's DAD header gives (acquired), each with its stream and field as the source.
fn rows_of_header(head: &[Head]) -> Vec<Row> {
    let from = |id: &str| head.iter().find(|h| h.0 == id);
    let hsrc = |h: &Head| Some(("read".to_string(), format!("{} / {}", h.2, h.3)));
    let mut rows = Vec::new();
    for (id, unit) in [("points", ""), ("channels", ""), ("duration", "min")] {
        if let Some(h) = from(id) {
            let t = if unit.is_empty() {
                count(h.1)
            } else {
                shown(h.1)
            };
            rows.push(row(id, Some(h.1), t, unit, hsrc(h)));
        }
    }
    if let (Some(a), Some(b)) = (from("wl_min"), from("wl_max")) {
        let at = format!("{} / wavelength_min, wavelength_max", a.2);
        rows.push(row(
            "range",
            None,
            format!("{}\u{2013}{}", shown(a.1), shown(b.1)),
            "nm",
            Some(("read".into(), at)),
        ));
    }
    if let Some(h) = from("step") {
        rows.push(row("step", Some(h.1), shown(h.1), "nm", hsrc(h)));
    }
    rows
}

/// What the detector of the method (programmed) adds to the rows the header gave; it keeps its own source.
fn rows_of_detector(d: &Value, rows: &mut Vec<Row>, header_range: bool) {
    let dsrc = d.get("source").and_then(|s| {
        Some((
            s.get("kind")?.str()?.to_string(),
            s.get("at")?.str()?.to_string(),
        ))
    });
    if let [a, b, ..] = d.get("nm").map(Value::arr).unwrap_or_default()
        && let (Some(a), Some(b)) = (a.f64(), b.f64())
    {
        let t = format!("{}\u{2013}{}", shown(a), shown(b));
        if !rows.iter().any(|r| r.id == "range" && r.text == t) {
            let id = if header_range {
                "range_method"
            } else {
                "range"
            };
            rows.push(row(id, None, t, "nm", dsrc.clone()));
        }
    }
    for (key, id, unit) in [
        ("step_nm", "step", "nm"),
        ("frequency_Hz", "frequency", "Hz"),
        ("cell_C", "cell", "\u{b0}C"),
        ("slit_nm", "slit", "nm"),
    ] {
        if rows.iter().all(|r| r.id != id)
            && let Some(v) = d.get(key).and_then(Value::f64)
        {
            rows.push(row(id, Some(v), shown(v), unit, dsrc.clone()));
        }
    }
    if let Some(l) = d.get("lamp").and_then(Value::str) {
        rows.push(row("lamp", None, l.to_string(), "", dsrc.clone()));
    }
    let n = d.get("channels").map(Value::arr).unwrap_or_default().len();
    if n > 0 && rows.iter().all(|r| r.id != "channels") {
        rows.push(row("channels", Some(n as f64), count(n as f64), "", dsrc));
    }
}

#[cfg(test)]
#[path = "methodview_tests.rs"]
mod tests;
