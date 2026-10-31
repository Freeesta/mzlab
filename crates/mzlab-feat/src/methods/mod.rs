//! The text of `report.methods@1`: a methods paragraph in English from a `corsa/1` record. Every step of the run gets a sentence from the table of `templates.rs` (or the generic sentence listing
//! its parameters) with the version of the step, and the text starts with the version of mzLab. It states only what was set, never a result. Bytes in, text out: `flow run` (`mzlab-cli`) and
//! the browser module `flowtab` (`mzlab-feat-wasm`) both call it, so they write the same Markdown bytes.

mod templates;

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};
use templates::TEMPLATES;

/// How the text is laid out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    /// One paragraph per step, from the sentence templates.
    Paragraph,
    /// One bullet per step with every parameter, name and value.
    List,
}

/// The style the parameter `style` names (`paragraph` when it is missing or unknown).
pub fn style_of(params: &[(String, Value)]) -> Style {
    match params
        .iter()
        .find(|(k, _)| k == "style")
        .and_then(|(_, v)| v.str())
    {
        Some("list") => Style::List,
        _ => Style::Paragraph,
    }
}

/// A parameter of a step as the record holds it.
struct Param {
    name: String,
    value: String,
    estimated: bool,
}

/// One step of the record.
struct Step {
    id: String,
    version: String,
    instance: String,
    algorithm: String,
    preset: String,
    failed: bool,
    params: Vec<Param>,
}

/// A value as a word or number of the text.
fn show(v: &Value) -> String {
    match v {
        Value::Null => "not set".into(),
        Value::Bool(b) => if *b { "on" } else { "off" }.into(),
        Value::Num(x) => format!("{x}"),
        Value::Str(s) => s.clone(),
        Value::Arr(a) => a
            .iter()
            .map(show)
            .collect::<Vec<_>>()
            .join(if a.len() == 2 { " to " } else { ", " }),
        Value::Obj(_) => "set".into(),
    }
}

fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::str)
        .unwrap_or_default()
        .to_string()
}

fn steps_of(run: &Value) -> Vec<Step> {
    run.get("steps")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|s| {
            let full = text(s, "step");
            let (id, version) = full.split_once('@').unwrap_or((full.as_str(), ""));
            Step {
                id: id.to_string(),
                version: version.to_string(),
                instance: text(s, "instance"),
                algorithm: text(s, "algorithm"),
                preset: text(s, "preset"),
                failed: s.get("status").and_then(Value::str) == Some("error"),
                params: s
                    .get("params")
                    .map_or(&[][..], Value::arr)
                    .iter()
                    .map(|p| Param {
                        name: text(p, "name"),
                        value: p.get("value").map_or_else(|| "not set".into(), show),
                        estimated: p.get("state").and_then(Value::str) == Some("estimated"),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// The name of an algorithm without its version (`centwave@1` is `centwave`).
fn bare(algorithm: &str) -> &str {
    algorithm.split('@').next().unwrap_or(algorithm)
}

/// The template of `step` for `algorithm` (the entry of the algorithm first, then the one for any), or None.
pub fn template(step: &str, algorithm: &str) -> Option<&'static str> {
    let algorithm = bare(algorithm);
    let find = |alg: &str| {
        TEMPLATES
            .iter()
            .find(|(s, a, _)| *s == step && *a == alg)
            .map(|t| t.2)
    };
    find(algorithm).or_else(|| find(""))
}

/// The `(step id, algorithm)` pairs the table holds, for the tests of the registry.
pub fn templated() -> Vec<(&'static str, &'static str)> {
    TEMPLATES.iter().map(|(s, a, _)| (*s, *a)).collect()
}

/// The names `{...}` a template uses.
pub fn placeholders(template: &str) -> Vec<&str> {
    template
        .split('{')
        .skip(1)
        .filter_map(|t| t.split_once('}').map(|x| x.0))
        .collect()
}

/// A value of a parameter in the words of the text; a value the step measured says so.
fn value_of(s: &Step, name: &str) -> String {
    match name {
        "algorithm" => bare(&s.algorithm).to_string(),
        "preset" => s.preset.clone(),
        _ => match s.params.iter().find(|p| p.name == name) {
            Some(p) if p.estimated => format!("{} (estimated from the data)", p.value),
            Some(p) => p.value.clone(),
            None => "unspecified".into(),
        },
    }
}

/// The parameters of a step as `name value` pairs separated by semicolons.
fn listing(s: &Step) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !s.algorithm.is_empty() {
        parts.push(format!("algorithm {}", s.algorithm));
    }
    if !s.preset.is_empty() {
        parts.push(format!("preset {}", s.preset));
    }
    parts.extend(s.params.iter().map(|p| {
        format!(
            "{} {}{}",
            p.name,
            p.value,
            if p.estimated {
                " (estimated from the data)"
            } else {
                ""
            }
        )
    }));
    if parts.is_empty() {
        "no parameters".into()
    } else {
        parts.join("; ")
    }
}

/// `template` without the clauses of the parameters the record does not hold: a clause runs between `(` or `, ` and the next `,` or `)`. A parameter outside a parenthesis reads «unspecified».
fn without_missing(template: &str, s: &Step) -> String {
    let mut t = template.to_string();
    'again: loop {
        let mut at = 0;
        while let Some(open) = t[at..].find('{') {
            let (from, close) = (at + open, at + open + t[at + open..].find('}').unwrap_or(0));
            let name = &t[from + 1..close];
            at = close;
            if matches!(name, "algorithm" | "preset") || s.params.iter().any(|p| p.name == name) {
                continue;
            }
            let (comma, paren) = (t[..from].rfind(", "), t[..from].rfind('('));
            let end = t[close..].find([',', ')']).map(|e| close + e);
            match (comma, paren, end) {
                (Some(c), p, Some(e)) if p.is_none_or(|p| c > p) => t.replace_range(c..e, ""),
                (_, Some(p), Some(e)) if t[e..].starts_with(',') => {
                    t.replace_range(p + 1..e + 2, "")
                }
                (_, Some(p), Some(e)) => t.replace_range(p - 1..e + 1, ""),
                _ => t.replace_range(from..=close, "unspecified"),
            }
            continue 'again;
        }
        return t;
    }
}

/// The sentence of a step: the template filled with the parameters, else the generic sentence.
fn sentence(s: &Step) -> String {
    let mut out = match template(&s.id, &s.algorithm) {
        Some(t) => {
            let t = without_missing(t, s);
            let mut done = String::new();
            let mut rest = t.as_str();
            while let Some((head, tail)) = rest.split_once('{') {
                let (name, after) = tail.split_once('}').unwrap_or((tail, ""));
                done.push_str(head);
                done.push_str(&value_of(s, name));
                rest = after;
            }
            done.push_str(rest);
            done
        }
        None => format!("The step {} was run with {}.", s.id, listing(s)),
    };
    if s.failed {
        out.push_str(" This step did not complete.");
    }
    out
}

/// The generic sentence of a step, for the tests.
pub fn generic_sentence(id: &str, params: &[(&str, &str)]) -> String {
    let s = Step {
        id: id.into(),
        version: String::new(),
        instance: String::new(),
        algorithm: String::new(),
        preset: String::new(),
        failed: false,
        params: params
            .iter()
            .map(|(n, v)| Param {
                name: n.to_string(),
                value: v.to_string(),
                estimated: false,
            })
            .collect(),
    };
    sentence(&s)
}

/// The Markdown of the methods for the record `run` (a `corsa/1` text); `params` are the parameters of the step.
pub fn methods_text(params: &[(String, Value)], run: &[u8]) -> Result<String> {
    let doc = parse(run)?;
    if doc.get("schema").and_then(Value::str) != Some("corsa/1") {
        return Err(Error::new("err.corsa.prov"));
    }
    let engine = doc.get("engine");
    let field = |k: &str| engine.map(|e| text(e, k)).unwrap_or_default();
    let (name, version) = (field("name"), field("version"));
    let mut head = format!(
        "Data were processed with {} {}",
        if name.is_empty() { "mzLab" } else { &name },
        if version.is_empty() {
            "(version not recorded)"
        } else {
            &version
        }
    );
    if !field("commit").is_empty() {
        head.push_str(&format!(" (commit {})", field("commit")));
    }
    head.push('.');
    let steps = steps_of(&doc);
    let mut out = String::from("# Methods\n\n");
    match style_of(params) {
        Style::Paragraph => {
            out.push_str(&head);
            out.push_str("\n\n");
            for s in &steps {
                out.push_str(&format!(
                    "{} (`{}`, step version {})\n\n",
                    sentence(s),
                    s.id,
                    s.version
                ));
            }
        }
        Style::List => {
            out.push_str(&head);
            out.push_str("\n\n");
            for s in &steps {
                out.push_str(&format!(
                    "- `{}@{}` (instance `{}`): {}{}\n",
                    s.id,
                    s.version,
                    s.instance,
                    listing(s),
                    if s.failed { "; did not complete" } else { "" }
                ));
            }
            out.push('\n');
        }
    }
    if let Some(sha) = doc.get("plan_sha256").and_then(Value::str) {
        out.push_str(&format!("Plan SHA-256: `{sha}`.\n"));
    }
    Ok(out.trim_end().to_string() + "\n")
}

#[cfg(test)]
mod tests;
