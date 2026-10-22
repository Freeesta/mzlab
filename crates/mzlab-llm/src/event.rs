//! The normalized events of a model answer (`llm-provider/1`, stream events) and their JSON line, shared by the adapters, the log and the recordings.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    End,
    Tool,
    Length,
    Refusal,
}

impl StopReason {
    pub fn name(self) -> &'static str {
        match self {
            StopReason::End => "end",
            StopReason::Tool => "tool",
            StopReason::Length => "length",
            StopReason::Refusal => "refusal",
        }
    }
}

/// Token counts of a turn; `input` includes neither `cache_read` nor the cache writes, which are billed at their own rates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    /// Tokens written to the cache with the 5-minute lifetime.
    pub cache_write_5m: u64,
    /// Tokens written to the cache with the 1-hour lifetime.
    pub cache_write_1h: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    TextDelta(String),
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallDelta(String),
    ToolCallEnd {
        id: String,
    },
    Usage(Usage),
    Stop(StopReason),
    /// `key` is an `err.llm.*` i18n key, `params` its values (never a secret).
    Error {
        key: String,
        params: Vec<(String, String)>,
    },
}

impl Event {
    pub fn error(key: &str, params: &[(&str, String)]) -> Event {
        Event::Error {
            key: key.to_string(),
            params: params
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        }
    }

    /// The fields of the JSON line as (key, JSON text), sorted by key.
    pub fn fields(&self) -> Vec<(&'static str, String)> {
        let s = |t: &str| {
            let mut o = Out::default();
            o.str(t);
            o.0
        };
        let mut f: Vec<(&'static str, String)> = match self {
            Event::TextDelta(t) => vec![("ev", s("text_delta")), ("text", s(t))],
            Event::ToolCallStart { id, name } => vec![
                ("ev", s("tool_call_start")),
                ("id", s(id)),
                ("name", s(name)),
            ],
            Event::ToolCallDelta(j) => vec![("ev", s("tool_call_delta")), ("json", s(j))],
            Event::ToolCallEnd { id } => vec![("ev", s("tool_call_end")), ("id", s(id))],
            Event::Usage(u) => usage_fields(u),
            Event::Stop(r) => vec![("ev", s("stop")), ("reason", s(r.name()))],
            Event::Error { key, params } => error_fields(key, params),
        };
        f.sort_by_key(|(k, _)| *k);
        f
    }

    /// One JSON object with sorted keys, no newline.
    pub fn to_json(&self) -> String {
        join(&self.fields())
    }
}

fn json_str(t: &str) -> String {
    let mut o = Out::default();
    o.str(t);
    o.0
}

/// The cache writes are written only when non-zero, so recordings and logs without them keep their bytes.
fn usage_fields(u: &Usage) -> Vec<(&'static str, String)> {
    let mut v = vec![
        ("cache_read", u.cache_read.to_string()),
        ("ev", json_str("usage")),
        ("in", u.input.to_string()),
        ("out", u.output.to_string()),
    ];
    if u.cache_write_5m > 0 {
        v.push(("cache_write_5m", u.cache_write_5m.to_string()));
    }
    if u.cache_write_1h > 0 {
        v.push(("cache_write_1h", u.cache_write_1h.to_string()));
    }
    v
}

fn error_fields(key: &str, params: &[(String, String)]) -> Vec<(&'static str, String)> {
    let p: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{}:{}", json_str(k), json_str(v)))
        .collect();
    vec![
        ("ev", json_str("error")),
        ("key", json_str(key)),
        ("params", format!("{{{}}}", p.join(","))),
    ]
}

/// `{"k":v,...}` from fields whose values are already JSON text.
pub(crate) fn join(f: &[(&str, String)]) -> String {
    let parts: Vec<String> = f
        .iter()
        .map(|(k, v)| {
            let mut o = Out::default();
            o.str(k).raw(":").raw(v);
            o.0
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

impl Event {
    pub fn from_value(v: &Value) -> Result<Event> {
        let bad = |w: &str| Error::new("err.llm.replay.format").with("field", w);
        let text = |k: &str| {
            v.get(k)
                .and_then(Value::str)
                .map(str::to_string)
                .ok_or_else(|| bad(k))
        };
        let num = |k: &str| {
            v.get(k)
                .and_then(Value::f64)
                .map(|x| x as u64)
                .ok_or_else(|| bad(k))
        };
        let opt = |k: &str| v.get(k).and_then(Value::f64).map_or(0, |x| x as u64);
        Ok(match v.get("ev").and_then(Value::str) {
            Some("text_delta") => Event::TextDelta(text("text")?),
            Some("tool_call_start") => Event::ToolCallStart {
                id: text("id")?,
                name: text("name")?,
            },
            Some("tool_call_delta") => Event::ToolCallDelta(text("json")?),
            Some("tool_call_end") => Event::ToolCallEnd { id: text("id")? },
            Some("usage") => Event::Usage(Usage {
                input: num("in")?,
                output: num("out")?,
                cache_read: num("cache_read")?,
                cache_write_5m: opt("cache_write_5m"),
                cache_write_1h: opt("cache_write_1h"),
            }),
            Some("stop") => Event::Stop(match text("reason")?.as_str() {
                "end" => StopReason::End,
                "tool" => StopReason::Tool,
                "length" => StopReason::Length,
                "refusal" => StopReason::Refusal,
                _ => return Err(bad("reason")),
            }),
            Some("error") => {
                let params = match v.get("params") {
                    Some(Value::Obj(m)) => m
                        .iter()
                        .filter_map(|(k, x)| x.str().map(|t| (k.clone(), t.to_string())))
                        .collect(),
                    _ => Vec::new(),
                };
                Event::Error {
                    key: text("key")?,
                    params,
                }
            }
            _ => return Err(bad("ev")),
        })
    }
}
