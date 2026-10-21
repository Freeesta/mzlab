//! The vendor-neutral conversation (`llm-conv/1`) the adapters read: roles, parts (text, context, image, tool call, tool result) and the system text.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Text(String),
    /// A `contesto/1` object, sent as its JSON text.
    Context(Value),
    Image {
        png_b64: String,
    },
    ToolCall {
        id: String,
        name: String,
        args: Value,
    },
    ToolResult {
        id: String,
        ok: bool,
        content: Value,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Conversation {
    pub provider: String,
    pub model: String,
    /// The system text; empty = none.
    pub system: String,
    /// `None` = the model's own limit.
    pub max_output_tokens: Option<u32>,
    pub messages: Vec<Message>,
}

fn bad(field: &str) -> Error {
    Error::new("err.llm.conversation").with("field", field)
}

impl Conversation {
    pub fn parse(text: &str) -> Result<Conversation> {
        Conversation::from_value(&parse(text.as_bytes()).map_err(|_| bad("json"))?)
    }

    pub fn from_value(v: &Value) -> Result<Conversation> {
        if v.get("schema").and_then(Value::str) != Some("llm-conv/1") {
            return Err(bad("schema"));
        }
        let text = |k: &str| v.get(k).and_then(Value::str).map(str::to_string);
        let mut messages = Vec::new();
        for m in v.get("messages").map_or(&[][..], Value::arr) {
            let role = match m.get("role").and_then(Value::str) {
                Some("user") => Role::User,
                Some("assistant") => Role::Assistant,
                Some("tool") => Role::Tool,
                _ => return Err(bad("role")),
            };
            let parts = m
                .get("parts")
                .map_or(&[][..], Value::arr)
                .iter()
                .map(part)
                .collect::<Result<Vec<_>>>()?;
            messages.push(Message { role, parts });
        }
        Ok(Conversation {
            provider: text("provider").ok_or_else(|| bad("provider"))?,
            model: text("model").ok_or_else(|| bad("model"))?,
            system: text("system").unwrap_or_default(),
            max_output_tokens: v
                .get("max_output_tokens")
                .and_then(Value::f64)
                .map(|x| x as u32),
            messages,
        })
    }
}

fn part(p: &Value) -> Result<Part> {
    let s = |v: &Value, k: &str| {
        v.get(k)
            .and_then(Value::str)
            .map(str::to_string)
            .ok_or_else(|| bad(k))
    };
    if let Some(t) = p.get("text").and_then(Value::str) {
        return Ok(Part::Text(t.to_string()));
    }
    if let Some(c) = p.get("context") {
        return Ok(Part::Context(c.clone()));
    }
    if let Some(i) = p.get("image") {
        return Ok(Part::Image {
            png_b64: s(i, "png_b64")?,
        });
    }
    if let Some(c) = p.get("tool_call") {
        return Ok(Part::ToolCall {
            id: s(c, "id")?,
            name: s(c, "name")?,
            args: c.get("args").cloned().unwrap_or(Value::Obj(Vec::new())),
        });
    }
    if let Some(r) = p.get("tool_result") {
        let ok = r.get("ok").and_then(Value::bool).unwrap_or(true);
        return Ok(Part::ToolResult {
            id: s(r, "id")?,
            ok,
            content: r.get("content").cloned().unwrap_or(Value::Null),
        });
    }
    Err(bad("part"))
}
