//! One interface for every vendor (`llm-provider/1`): `request` turns a provider, a conversation and the tools into `{url, headers, body}` without the
//! key (the transport puts it in the `AuthSlot`), `Stream` turns the response bytes into normalized events, `http_error` maps a failed status. Pure
//! functions on text; the transport (fetch in the site, `ureq` in the desktop) lives elsewhere.

mod anthropic;
mod errors;
mod gemini;
mod openai;
mod options;
pub mod sse;
mod wire;

pub(crate) use errors::api_error;
pub use errors::{http_error, retry_delay, MAX_RATE_RETRIES};
pub use options::{CacheTtl, Effort, Options};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use crate::conv::{Conversation, Part};
use crate::event::{Event, StopReason};
use crate::tools::{value, Tool};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adapter {
    Anthropic,
    OpenAiCompat,
    Gemini,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolMode {
    Native,
    Prompted,
    None,
}

/// Rates that apply to a whole request whose prompt has more than `above_tokens` tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct PriceOver {
    pub above_tokens: u64,
    pub rates: Price,
}

/// Price per million tokens; 0 = unknown. With a known `input`, the cache rates are those of the file or, when it has none, the usual ones
/// (`cache_read` = `input`, writes 1.25× and 2× `input`). `as_of` is the date the price was checked on the vendor's page.
#[derive(Clone, Debug, PartialEq)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub write_5m: f64,
    pub write_1h: f64,
    pub as_of: String,
    pub currency: String,
    pub over: Option<Box<PriceOver>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub id: String,
    pub context_tokens: u64,
    pub max_output_tokens: u32,
    pub tools: ToolMode,
    pub vision: bool,
    pub price: Price,
}

/// Where the transport puts the key: `header: prefix + key`; an empty header means no key (local servers).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuthSlot {
    pub header: String,
    pub prefix: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Provider {
    pub id: String,
    pub adapter: Adapter,
    pub base_url: String,
    pub auth: AuthSlot,
    pub extra_headers: Vec<(String, String)>,
    pub models: Vec<Model>,
    pub local: bool,
    pub site_allowed: bool,
}

/// What the transport sends. The key is not a field: it goes where `auth` says.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub auth: AuthSlot,
}

fn bad(field: &str) -> Error {
    Error::new("err.llm.provider").with("field", field)
}

impl Request {
    /// `{url, headers: [[name, value]], body, auth: {header, prefix}}`: what the transport sends; the key is added by the transport at `auth`.
    pub fn to_json(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"url\":").str(&self.url).raw(",\"headers\":[");
        for (i, (k, v)) in self.headers.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.raw("[").str(k).raw(",").str(v).raw("]");
        }
        o.raw("],\"body\":").str(&self.body);
        o.raw(",\"auth\":{\"header\":").str(&self.auth.header);
        o.raw(",\"prefix\":").str(&self.auth.prefix).raw("}}");
        o.0
    }
}

impl Provider {
    pub fn from_value(v: &Value) -> Result<Provider> {
        let s = |v: &Value, k: &str| {
            v.get(k)
                .and_then(Value::str)
                .map(str::to_string)
                .ok_or_else(|| bad(k))
        };
        let adapter = match s(v, "adapter")?.as_str() {
            "anthropic" => Adapter::Anthropic,
            "openai_compat" => Adapter::OpenAiCompat,
            "gemini" => Adapter::Gemini,
            _ => return Err(bad("adapter")),
        };
        let auth = v.get("auth").ok_or_else(|| bad("auth"))?;
        let extra_headers = match v.get("extra_headers") {
            Some(Value::Obj(m)) => m
                .iter()
                .filter_map(|(k, x)| x.str().map(|t| (k.clone(), t.to_string())))
                .collect(),
            _ => Vec::new(),
        };
        let models = v
            .get("models")
            .map_or(&[][..], Value::arr)
            .iter()
            .map(model)
            .collect::<Result<Vec<_>>>()?;
        Ok(Provider {
            id: s(v, "id")?,
            adapter,
            base_url: s(v, "base_url")?.trim_end_matches('/').to_string(),
            auth: AuthSlot {
                header: s(auth, "header")?,
                prefix: s(auth, "prefix")?,
            },
            extra_headers,
            models,
            local: v.get("local").and_then(Value::bool).unwrap_or(false),
            site_allowed: v.get("site_allowed").and_then(Value::bool).unwrap_or(false),
        })
    }

    pub fn parse(text: &str) -> Result<Provider> {
        Provider::from_value(&parse(text.as_bytes()).map_err(|_| bad("json"))?)
    }

    pub fn model(&self, id: &str) -> Option<&Model> {
        self.models.iter().find(|m| m.id == id)
    }
}

fn model(v: &Value) -> Result<Model> {
    let num = |k: &str| v.get(k).and_then(Value::f64).ok_or_else(|| bad(k));
    let price = v
        .get("price_per_mtok")
        .ok_or_else(|| bad("price_per_mtok"))?;
    let tools = match v.get("tools").and_then(Value::str) {
        Some("native") => ToolMode::Native,
        Some("prompted") => ToolMode::Prompted,
        Some("none") => ToolMode::None,
        _ => return Err(bad("tools")),
    };
    Ok(Model {
        id: v
            .get("id")
            .and_then(Value::str)
            .ok_or_else(|| bad("id"))?
            .to_string(),
        context_tokens: num("context_tokens")? as u64,
        max_output_tokens: num("max_output_tokens")? as u32,
        tools,
        vision: v.get("vision").and_then(Value::bool).unwrap_or(false),
        price: price_of(price),
    })
}

fn price_of(v: &Value) -> Price {
    let p = |k: &str| v.get(k).and_then(Value::f64);
    let input = p("in").unwrap_or(0.0);
    let known = input > 0.0;
    Price {
        input,
        output: p("out").unwrap_or(0.0),
        cache_read: p("cache_read").unwrap_or(if known { input } else { 0.0 }),
        write_5m: p("write_5m").unwrap_or(input * 1.25),
        write_1h: p("write_1h").unwrap_or(input * 2.0),
        as_of: v
            .get("as_of")
            .and_then(Value::str)
            .unwrap_or("")
            .to_string(),
        currency: v
            .get("currency")
            .and_then(Value::str)
            .unwrap_or("USD")
            .to_string(),
        over: v.get("over").map(|o| {
            Box::new(PriceOver {
                above_tokens: o.get("above_tokens").and_then(Value::f64).unwrap_or(0.0) as u64,
                rates: price_of(o),
            })
        }),
    }
}

/// The text of `data/fornitori-1.json`, for a page that lists the providers and models without parsing them again.
pub const BUILTIN_JSON: &str = include_str!("../../data/fornitori-1.json");

/// The providers shipped with the program (`data/fornitori-1.json`).
pub fn builtin() -> Vec<Provider> {
    let v = parse(BUILTIN_JSON.as_bytes()).expect("fornitori-1.json is JSON");
    v.get("providers")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|p| Provider::from_value(p).expect("fornitori-1.json is valid"))
        .collect()
}

/// Builds the request of the next turn. A `prompted` model gets the tool schemas in the system text, a `none` model gets no tools.
pub fn request(p: &Provider, c: &Conversation, tools: &[Tool]) -> Result<Request> {
    request_with(p, c, tools, &Options::default())
}

/// `request` with the per-role knobs of `Options` (cache, effort, thinking, strict tools, answer schema).
pub fn request_with(
    p: &Provider,
    c: &Conversation,
    tools: &[Tool],
    opts: &Options,
) -> Result<Request> {
    let err = |f: &str| Error::new("err.llm.conversation").with("field", f);
    if c.provider != p.id {
        return Err(err("provider"));
    }
    let m = p.model(&c.model).ok_or_else(|| err("model"))?;
    if let Some(f) = &opts.format_schema {
        if !tools.is_empty() || parse(f.as_bytes()).is_err() {
            return Err(err("format"));
        }
    }
    if !m.vision
        && c.messages
            .iter()
            .flat_map(|x| &x.parts)
            .any(|x| matches!(x, Part::Image { .. }))
    {
        return Err(err("image"));
    }
    let native: &[Tool] = if m.tools == ToolMode::Native {
        tools
    } else {
        &[]
    };
    let mut system = c.system.clone();
    if m.tools == ToolMode::Prompted && !tools.is_empty() {
        system = prompted_system(&system, tools);
    }
    let ctx = Ctx {
        model: m,
        conv: c,
        system,
        tools: native,
        max: c.max_output_tokens.unwrap_or(m.max_output_tokens),
        opts,
    };
    let (url, body) = match p.adapter {
        Adapter::Anthropic => (format!("{}/v1/messages", p.base_url), anthropic::body(&ctx)),
        Adapter::OpenAiCompat => (
            format!("{}/chat/completions", p.base_url),
            openai::body(&ctx),
        ),
        Adapter::Gemini => (
            format!(
                "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
                p.base_url, m.id
            ),
            gemini::body(&ctx),
        ),
    };
    let mut headers = vec![
        ("content-type".to_string(), "application/json".to_string()),
        ("accept".to_string(), "text/event-stream".to_string()),
    ];
    headers.extend(p.extra_headers.iter().cloned());
    Ok(Request {
        url,
        headers,
        body,
        auth: p.auth.clone(),
    })
}

/// What an adapter needs to write a body.
pub(crate) struct Ctx<'a> {
    pub model: &'a Model,
    pub conv: &'a Conversation,
    pub system: String,
    pub tools: &'a [Tool],
    pub max: u32,
    pub opts: &'a Options,
}

pub(crate) fn json_text(v: &Value) -> String {
    let mut o = Out::default();
    value(&mut o, v);
    o.0
}

/// The text of a part that travels as plain text (text, context); `None` for the others.
pub(crate) fn plain(p: &Part) -> Option<String> {
    match p {
        Part::Text(t) => Some(t.clone()),
        Part::Context(c) => Some(json_text(c)),
        _ => None,
    }
}

fn prompted_system(base: &str, tools: &[Tool]) -> String {
    let mut s = base.to_string();
    if !s.is_empty() {
        s.push_str("\n\n");
    }
    s.push_str("You may call one tool per answer. To call it, end your answer with exactly one block ```json {\"tool\":NAME,\"args\":{...}} ``` (the block on its own lines); the result comes in the next message. Tools:\n");
    for t in tools {
        s.push_str(&format!(
            "- {}: {} Arguments (JSON Schema): {}\n",
            t.name, t.description, t.input_schema
        ));
    }
    s
}

/// The decoder of one vendor: frames in, events out.
pub(crate) trait Decoder {
    fn frame(&mut self, name: &str, data: &str, out: &mut Vec<Event>);
    /// The end of the bytes: events a vendor leaves implicit (usage, stop).
    fn end(&mut self, out: &mut Vec<Event>);
}

/// The events of a response, fed with the bytes as they arrive (cut anywhere).
pub struct Stream {
    sse: sse::Sse,
    dec: Box<dyn Decoder>,
    prompted: Option<String>,
    finished: bool,
}

impl Stream {
    pub fn new(adapter: Adapter) -> Stream {
        let dec: Box<dyn Decoder> = match adapter {
            Adapter::Anthropic => Box::<anthropic::Dec>::default(),
            Adapter::OpenAiCompat => Box::<openai::Dec>::default(),
            Adapter::Gemini => Box::<gemini::Dec>::default(),
        };
        Stream {
            sse: sse::Sse::default(),
            dec,
            prompted: None,
            finished: false,
        }
    }

    /// For a `prompted` model: the first ```json block of the answer becomes a tool call, and the stop reason `tool`.
    pub fn prompted(mut self) -> Stream {
        self.prompted = Some(String::new());
        self
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<Event> {
        let mut out = Vec::new();
        for f in self.sse.push(bytes) {
            self.dec.frame(&f.name, &f.data, &mut out);
        }
        self.post(out)
    }

    /// The end of the bytes. A stream that ended without a stop reason is a network failure.
    pub fn finish(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        for f in self.sse.finish() {
            self.dec.frame(&f.name, &f.data, &mut out);
        }
        self.dec.end(&mut out);
        let closed = self.finished
            || out
                .iter()
                .any(|e| matches!(e, Event::Stop(_) | Event::Error { .. }));
        if !closed {
            out.push(Event::error(
                "err.llm.network",
                &[("reason", "truncated".to_string())],
            ));
        }
        self.post(out)
    }
}

/// `{"tool":NAME,"args":{...}}` in the first ```json block of `text`, as (name, args JSON text).
fn first_json_call(text: &str) -> Option<(String, String)> {
    let start = text.find("```json")? + "```json".len();
    let block = &text[start..start + text[start..].find("```")?];
    let v = parse(block.trim().as_bytes()).ok()?;
    let name = v.get("tool")?.str()?.to_string();
    let args = v.get("args").cloned().unwrap_or(Value::Obj(Vec::new()));
    Some((name, json_text(&args)))
}

impl Stream {
    fn post(&mut self, events: Vec<Event>) -> Vec<Event> {
        let mut out = Vec::new();
        for e in events {
            match (&e, self.prompted.as_mut()) {
                (Event::TextDelta(t), Some(acc)) => acc.push_str(t),
                (Event::Stop(StopReason::End), Some(acc)) => {
                    if let Some((name, args)) = first_json_call(acc) {
                        out.push(Event::ToolCallStart {
                            id: "prompted-1".into(),
                            name,
                        });
                        out.push(Event::ToolCallDelta(args));
                        out.push(Event::ToolCallEnd {
                            id: "prompted-1".into(),
                        });
                        out.push(Event::Stop(StopReason::Tool));
                        self.finished = true;
                        continue;
                    }
                }
                _ => {}
            }
            if matches!(e, Event::Stop(_) | Event::Error { .. }) {
                self.finished = true;
            }
            out.push(e);
        }
        out
    }
}
