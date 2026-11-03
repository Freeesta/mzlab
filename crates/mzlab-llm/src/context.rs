//! What the assistant sees (`contesto/1`): a few summaries of what the engine already produced (file info, TIC, the plan, a run, a table, the MSn
//! graph, the annotation levels), reduced and rounded here so the same input gives the same bytes. Never spectra, profile points, file bytes or paths.
//! The page collects the sources; this module cuts them to a token budget and says what it cut. Used by the transport and `mzlab-cli llm context`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use crate::tools::value;

/// The longest TIC the assistant gets, in points.
pub const TIC_POINTS: usize = 200;
/// The most rows of a table summary.
pub const TOP_ROWS: usize = 50;

/// A number as the assistant sees it: whole numbers as they are, the rest to four significant digits.
pub(crate) fn sig4(x: f64) -> f64 {
    if x == 0.0 || x.fract() == 0.0 || !x.is_finite() {
        return x;
    }
    let m = 10f64.powf(3.0 - x.abs().log10().floor());
    (x * m).round() / m
}

fn rounded(v: &Value) -> Value {
    match v {
        Value::Num(n) => Value::Num(sig4(*n)),
        Value::Arr(a) => Value::Arr(a.iter().map(rounded).collect()),
        Value::Obj(o) => Value::Obj(o.iter().map(|(k, x)| (k.clone(), rounded(x))).collect()),
        other => other.clone(),
    }
}

/// The object with only `keys` (those present), rounded.
fn pick(v: &Value, keys: &[&str]) -> Value {
    Value::Obj(
        keys.iter()
            .filter_map(|k| v.get(k).map(|x| (k.to_string(), rounded(x))))
            .collect(),
    )
}

fn nums(v: Option<&Value>) -> Vec<f64> {
    v.map_or(&[][..], Value::arr)
        .iter()
        .filter_map(Value::f64)
        .collect()
}

/// At most `TIC_POINTS` points: the maximum of each interval, so the peaks stay.
pub(crate) fn tic(v: &Value) -> Value {
    let (rt, y) = (nums(v.get("rt")), nums(v.get("i")));
    let n = rt.len().min(y.len());
    let step = n.div_ceil(TIC_POINTS).max(1);
    let (mut r, mut i) = (Vec::new(), Vec::new());
    for k in (0..n).step_by(step) {
        let end = (k + step).min(n);
        let at = (k..end).fold(k, |b, j| if y[j] > y[b] { j } else { b });
        r.push(Value::Num(sig4(rt[at])));
        i.push(Value::Num(sig4(y[at])));
    }
    Value::Obj(vec![
        ("rt".into(), Value::Arr(r)),
        ("i".into(), Value::Arr(i)),
        ("sig".into(), Value::Num(4.0)),
    ])
}

fn file(v: &Value, k: usize, hide: bool) -> Value {
    let mut o = match pick(
        v,
        &[
            "instrument",
            "resolution",
            "polarity",
            "ms_levels",
            "scans",
            "rt_min",
            "mode",
            "warnings",
        ],
    ) {
        Value::Obj(o) => o,
        _ => Vec::new(),
    };
    let name = if hide { None } else { v.get("name") };
    let mut head = vec![("alias".to_string(), Value::Str(format!("f{}", k + 1)))];
    head.extend(name.map(|n| ("name".to_string(), n.clone())));
    if let Some(t) = v.get("tic") {
        o.push(("tic".into(), tic(t)));
    }
    head.extend(o);
    Value::Obj(head)
}

pub(crate) fn cells(line: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

fn cell(s: &str) -> Value {
    s.trim()
        .parse::<f64>()
        .map_or_else(|_| Value::Str(s.to_string()), |n| Value::Num(sig4(n)))
}

fn column(rows: &[Vec<String>], k: usize) -> Value {
    let filled: Vec<&str> = rows
        .iter()
        .filter_map(|r| r.get(k))
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
        .collect();
    let num: Vec<f64> = filled.iter().filter_map(|c| c.parse().ok()).collect();
    let mut o = vec![("filled".to_string(), Value::Num(filled.len() as f64))];
    if !num.is_empty() && num.len() == filled.len() {
        let lo = num.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = num.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        o.push(("min".into(), Value::Num(sig4(lo))));
        o.push(("max".into(), Value::Num(sig4(hi))));
    }
    Value::Obj(o)
}

/// A table as the assistant sees it: how many rows, per column the filled cells and the minimum and maximum, and the first rows (by area when
/// there is an `area` column).
pub fn table_summary(csv: &str) -> Value {
    let mut lines = csv.lines().filter(|l| !l.trim().is_empty());
    let head = lines.next().map(cells).unwrap_or_default();
    let mut rows: Vec<Vec<String>> = lines.map(cells).collect();
    let columns = head
        .iter()
        .enumerate()
        .map(|(k, h)| (h.clone(), column(&rows, k)))
        .collect();
    if let Some(a) = head.iter().position(|h| h == "area") {
        let key = |r: &Vec<String>| {
            r.get(a)
                .and_then(|c| c.trim().parse::<f64>().ok())
                .unwrap_or(f64::NEG_INFINITY)
        };
        rows.sort_by(|x, y| key(y).total_cmp(&key(x)));
    }
    let total = rows.len();
    let top = rows
        .iter()
        .take(TOP_ROWS)
        .map(|r| {
            Value::Obj(
                head.iter()
                    .zip(r)
                    .map(|(h, c)| (h.clone(), cell(c)))
                    .collect(),
            )
        })
        .collect();
    Value::Obj(vec![
        ("rows".into(), Value::Num(total as f64)),
        ("columns".into(), Value::Obj(columns)),
        ("top".into(), Value::Arr(top)),
    ])
}

fn run(v: &Value) -> Value {
    let steps = v
        .get("steps")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|s| pick(s, &["instance", "step", "status", "ms", "params"]));
    let mut o = vec![("steps".to_string(), Value::Arr(steps.collect()))];
    if let Value::Obj(extra) = pick(v, &["total_ms", "levels"]) {
        o.extend(extra);
    }
    Value::Obj(o)
}

fn annot(v: &Value) -> Value {
    let mut counts: Vec<(String, f64)> = Vec::new();
    for l in v
        .get("rows")
        .map_or(&[][..], Value::arr)
        .iter()
        .filter_map(|r| r.get("level").and_then(Value::str))
    {
        match counts.iter_mut().find(|(k, _)| k == l) {
            Some(c) => c.1 += 1.0,
            None => counts.push((l.to_string(), 1.0)),
        }
    }
    counts.sort_by(|a, b| a.0.cmp(&b.0));
    Value::Obj(vec![(
        "levels".into(),
        Value::Obj(
            counts
                .into_iter()
                .map(|(k, n)| (k, Value::Num(n)))
                .collect(),
        ),
    )])
}

fn msn(v: &Value) -> Value {
    let c = v.get("conteggi");
    let n = |k: &str| c.and_then(|c| c.get(k)).cloned().unwrap_or(Value::Num(0.0));
    Value::Obj(vec![
        ("nodes".into(), n("nodi")),
        ("edges".into(), n("archi")),
    ])
}

fn plan(v: &Value) -> Value {
    let instances = v
        .get("instances")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|i| pick(i, &["id", "step", "params"]));
    let problems = v
        .get("problems")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|p| pick(p, &["error_key", "instance", "port"]));
    Value::Obj(vec![
        (
            "sha256".into(),
            v.get("sha256").cloned().unwrap_or(Value::Null),
        ),
        ("instances".into(), Value::Arr(instances.collect())),
        ("problems".into(), Value::Arr(problems.collect())),
    ])
}

fn sections(src: &Value, hide: bool) -> Vec<(&'static str, Value)> {
    let mut v = Vec::new();
    if let Some(f) = src.get("files") {
        v.push((
            "files",
            Value::Arr(
                f.arr()
                    .iter()
                    .enumerate()
                    .map(|(k, x)| file(x, k, hide))
                    .collect(),
            ),
        ));
    }
    let table = src
        .get("features_csv")
        .and_then(Value::str)
        .map(table_summary);
    let parts: [(&'static str, Option<Value>); 5] = [
        ("plan", src.get("plan").map(plan)),
        ("run", src.get("run").map(run)),
        ("features", table),
        ("msn", src.get("msn").map(msn)),
        ("annot", src.get("annot").map(annot)),
    ];
    v.extend(parts.into_iter().filter_map(|(k, x)| x.map(|x| (k, x))));
    v
}

fn render(budget: u64, cut: &[&str], parts: &[(&'static str, Value)]) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"contesto/1\",\"profile\":\"ricerca\",\"budget_tokens\":")
        .int(budget as i64);
    o.raw(",\"truncated\":[");
    for (k, c) in cut.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(c);
    }
    o.raw("]");
    for (k, v) in parts {
        o.raw(",").str(k).raw(":");
        value(&mut o, v);
    }
    o.raw("}");
    o.0
}

fn without_tic(parts: &mut [(&'static str, Value)]) {
    for (k, v) in parts.iter_mut() {
        if let (&mut "files", Value::Arr(files)) = (k, v) {
            for f in files.iter_mut() {
                if let Value::Obj(o) = f {
                    o.retain(|(n, _)| n != "tic");
                }
            }
        }
    }
}

/// The `contesto/1` text for `sources` within `budget_tokens` (`ceil(bytes / 3)`); sections that do not fit are dropped, last-useful first, and named
/// in `truncated`. The `corso` profile gets `err.llm.profile.course`: the assistant does not exist there.
pub fn build(sources: &Value, budget_tokens: u64) -> Result<String> {
    if sources.get("profile").and_then(Value::str) != Some("ricerca") {
        return Err(Error::new("err.llm.profile.course"));
    }
    let hide = sources.get("hide_names").and_then(Value::bool) == Some(true);
    let mut parts = sections(sources, hide);
    let mut cut: Vec<&str> = Vec::new();
    let fits = |t: &str| (t.len() as u64).div_ceil(3) <= budget_tokens;
    for name in ["features", "msn", "run", "annot"] {
        if fits(&render(budget_tokens, &cut, &parts)) {
            break;
        }
        if parts.iter().any(|(k, _)| *k == name) {
            parts.retain(|(k, _)| *k != name);
            cut.push(name);
        }
    }
    if !fits(&render(budget_tokens, &cut, &parts)) {
        without_tic(&mut parts);
        cut.push("tic");
    }
    let text = render(budget_tokens, &cut, &parts);
    if fits(&text) {
        Ok(text)
    } else {
        Err(Error::new("err.llm.context.budget").with("budget", budget_tokens))
    }
}

/// `build` from JSON text.
pub fn build_text(sources: &str, budget_tokens: u64) -> Result<String> {
    let v = parse(sources.as_bytes()).map_err(|_| Error::new("err.llm.context.format"))?;
    build(&v, budget_tokens)
}
