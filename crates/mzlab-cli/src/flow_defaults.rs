//! `mzlab-cli flow defaults STEP[@V] [--json]`: every parameter of a Flow step with its default, limits, unit and the `--set` key. The answer is
//! `mzlab_core::step::defaults::route` over the descriptor of the registry, the same function the app route and the site's `.wasm` call.

use mzlab_core::error::Error;
use mzlab_core::json::{self, Value};
use mzlab_core::jsonb::write;
use mzlab_core::step::defaults;
use mzlab_core::step::json::step_json;

use crate::flow::steps;
use crate::flow_json::fail;
use crate::Outcome;

const HELP: &str = "mzlab-cli flow defaults: the parameters of one Flow step with their defaults, from the step descriptors of this program.

usage: mzlab-cli flow defaults STEP[@VERSION] [--json]

  (no option)       one line per parameter: the `--set` key, type, default (`off` or `estimated` when it has none), limits, unit; then one line per preset with every value it writes
  --json            step-params/1: per parameter name, set_key, type, default, when_empty, minimum, maximum, enum, unit, algorithm, advanced, description; `presets` (id, set_key, title, params, evidence) when the step has them
  --help            this text

without @VERSION the highest version. A parameter with default null and when_empty `off` is a rule that is not applied until you set it;
`estimated` is measured by the step and written with its proof.

exit codes: 0 done, 2 usage, 5 unknown step (err.flow.step.unknown, the closest ids in the hint)";

/// Edit distance between two ids (they are short).
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// The three registry ids closest to `want`, nearest first.
fn closest(want: &str) -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = steps().iter().map(|s| s.id).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.sort_by_key(|id| distance(want, id));
    ids.truncate(3);
    ids
}

/// One aligned line per parameter, read from the document the engine wrote.
fn text_lines(doc: &Value) -> String {
    let show = |v: Option<&Value>| v.filter(|v| !v.is_null()).map(write);
    let mut rows = Vec::new();
    for p in doc.get("params").map_or(&[][..], Value::arr) {
        let field = |k: &str| p.get(k);
        let default = show(field("default"))
            .or_else(|| {
                field("when_empty")
                    .and_then(Value::str)
                    .map(|w| format!("({w})"))
            })
            .unwrap_or_default();
        let limits = match (show(field("minimum")), show(field("maximum"))) {
            (None, None) => String::new(),
            (lo, hi) => format!("[{}..{}]", lo.unwrap_or_default(), hi.unwrap_or_default()),
        };
        let choices = show(field("enum")).unwrap_or_default();
        let mut tail = vec![limits, choices];
        tail.push(field("unit").and_then(Value::str).unwrap_or("").to_string());
        if let Some(a) = field("algorithm").and_then(Value::str) {
            tail.push(format!("algorithm={a}"));
        }
        tail.retain(|t| !t.is_empty());
        rows.push((
            field("set_key")
                .and_then(Value::str)
                .unwrap_or("")
                .to_string(),
            field("type").and_then(Value::str).unwrap_or("").to_string(),
            default,
            tail.join(" "),
        ));
    }
    let width =
        |f: fn(&(String, String, String, String)) -> usize| rows.iter().map(f).max().unwrap_or(0);
    let (w0, w1, w2) = (
        width(|r| r.0.len()),
        width(|r| r.1.len()),
        width(|r| r.2.len()),
    );
    let mut out: String = rows
        .iter()
        .map(|r| {
            format!("{:<w0$}  {:<w1$}  {:<w2$}  {}\n", r.0, r.1, r.2, r.3).replace("  \n", "\n")
        })
        .collect();
    // one line per preset: the `--set` that selects it and every value it writes
    for p in doc.get("presets").map_or(&[][..], Value::arr) {
        let field = |k: &str| p.get(k).and_then(Value::str).unwrap_or("");
        let set = |v: &Value| {
            let key = v.get("set_key").and_then(Value::str).unwrap_or("");
            format!("{key}={}", v.get("value").map(write).unwrap_or_default())
        };
        let values: Vec<String> = p
            .get("params")
            .map_or(&[][..], Value::arr)
            .iter()
            .map(set)
            .collect();
        out.push_str(&format!(
            "preset {}: --set {}={} writes {}\n",
            field("id"),
            field("set_key"),
            field("id"),
            values.join(" ")
        ));
    }
    out
}

/// `mzlab-cli flow defaults ARGS...` (the arguments after `defaults`).
pub(crate) fn run(args: Vec<String>) -> Outcome {
    let usage = |msg: &str| {
        let err = Error::new("err.cli.usage").with("message", msg);
        fail(&err, msg, "mzlab-cli flow defaults --help")
    };
    let (mut json, mut id) = (false, None);
    for x in args {
        match x.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--json" => json = true,
            s if s.starts_with('-') => return usage(&format!("unknown option {s}")),
            _ if id.is_some() => return usage("one STEP only"),
            _ => id = Some(x),
        }
    }
    let Some(want) = id else {
        return usage("flow defaults needs a step id");
    };
    let (name, version) = want
        .split_once('@')
        .map_or((want.as_str(), None), |(n, v)| (n, v.parse::<u32>().ok()));
    let reg = steps();
    let found = reg
        .iter()
        .filter(|s| s.id == name && version.is_none_or(|v| s.version == v))
        .max_by_key(|s| s.version);
    let Some(step) = found else {
        let err = Error::new("err.flow.step.unknown").with("step", &want);
        let hint = format!("closest: {}", closest(name).join(", "));
        return fail(&err, &format!("unknown step {want}"), &hint);
    };
    let doc = match defaults::route(&step_json(step)) {
        Ok(d) => d,
        Err(e) => return Outcome::fail_error(&e),
    };
    if json {
        // the engine's document and the citation member, nothing else
        let body = doc.trim_end().strip_suffix('}').unwrap_or(&doc);
        return Outcome::ok(format!("{body},{}}}\n", crate::citation::member()).into_bytes());
    }
    match json::parse(doc.as_bytes()) {
        Ok(v) => Outcome::ok(text_lines(&v).into_bytes()),
        Err(e) => Outcome::fail_error(&e),
    }
}

#[cfg(test)]
#[path = "flow_defaults_tests.rs"]
mod tests;
