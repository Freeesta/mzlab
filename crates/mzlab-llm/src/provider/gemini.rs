//! `streamGenerateContent` of Gemini: request body and stream decoder. The tool schema goes in `parametersJsonSchema` (full JSON Schema).

use mzlab_core::json::{parse, Out, Value};

use super::{api_error, json_text, plain, Ctx, Decoder};
use crate::conv::{Part, Role};
use crate::event::{Event, StopReason, Usage};

pub(super) fn body(c: &Ctx) -> String {
    let mut o = Out::default();
    o.raw("{");
    if !c.system.is_empty() {
        o.raw("\"systemInstruction\":{\"parts\":[{\"text\":")
            .str(&c.system)
            .raw("}]},");
    }
    o.raw("\"contents\":[");
    for (i, m) in c.conv.messages.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.raw(if m.role == Role::Assistant {
            "{\"role\":\"model\",\"parts\":["
        } else {
            "{\"role\":\"user\",\"parts\":["
        });
        for (j, p) in m.parts.iter().enumerate() {
            if j > 0 {
                o.raw(",");
            }
            part(&mut o, p, c);
        }
        o.raw("]}");
    }
    o.raw("],\"generationConfig\":{\"maxOutputTokens\":")
        .int(c.max as i64)
        .raw("}");
    if !c.tools.is_empty() {
        o.raw(",\"tools\":[{\"functionDeclarations\":[");
        for (i, t) in c.tools.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.raw("{\"name\":")
                .str(t.name)
                .raw(",\"description\":")
                .str(t.description)
                .raw(",\"parametersJsonSchema\":")
                .raw(&t.input_schema)
                .raw("}");
        }
        o.raw("]}]");
    }
    o.raw("}");
    o.0
}

fn part(o: &mut Out, p: &Part, c: &Ctx) {
    if let Some(t) = plain(p) {
        o.raw("{\"text\":").str(&t).raw("}");
        return;
    }
    match p {
        Part::Image { png_b64 } => {
            o.raw("{\"inlineData\":{\"mimeType\":\"image/png\",\"data\":")
                .str(png_b64)
                .raw("}}");
        }
        Part::ToolCall { name, args, .. } => {
            o.raw("{\"functionCall\":{\"name\":")
                .str(name)
                .raw(",\"args\":")
                .raw(&json_text(args))
                .raw("}}");
        }
        Part::ToolResult { id, ok, content } => {
            // Gemini matches a result by function name, the conversation by call id.
            let name = c
                .conv
                .messages
                .iter()
                .flat_map(|m| &m.parts)
                .find_map(|q| match q {
                    Part::ToolCall { id: i, name, .. } if i == id => Some(name.as_str()),
                    _ => None,
                })
                .unwrap_or(id);
            let key = if *ok { "output" } else { "error" };
            o.raw("{\"functionResponse\":{\"name\":")
                .str(name)
                .raw(",\"response\":{")
                .str(key)
                .raw(":")
                .raw(&json_text(content))
                .raw("}}}");
        }
        Part::Text(_) | Part::Context(_) => {}
    }
}

#[derive(Default)]
pub(super) struct Dec {
    usage: Option<Usage>,
    reason: Option<StopReason>,
    calls: usize,
    stopped: bool,
}

impl Dec {
    fn parts(&mut self, c: &Value, out: &mut Vec<Event>) {
        for p in c
            .get("content")
            .and_then(|c| c.get("parts"))
            .map_or(&[][..], Value::arr)
        {
            if let Some(t) = p.get("text").and_then(Value::str).filter(|t| !t.is_empty()) {
                out.push(Event::TextDelta(t.to_string()));
            }
            if let Some(f) = p.get("functionCall") {
                self.calls += 1;
                let id = format!("gemini-{}", self.calls);
                out.push(Event::ToolCallStart {
                    id: id.clone(),
                    name: f.get("name").and_then(Value::str).unwrap_or("").to_string(),
                });
                out.push(Event::ToolCallDelta(
                    f.get("args").map_or_else(|| "{}".to_string(), json_text),
                ));
                out.push(Event::ToolCallEnd { id });
            }
        }
    }
}

fn stop_reason(r: &str) -> StopReason {
    match r {
        "MAX_TOKENS" => StopReason::Length,
        "SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII" => {
            StopReason::Refusal
        }
        _ => StopReason::End,
    }
}

impl Decoder for Dec {
    fn frame(&mut self, _name: &str, data: &str, out: &mut Vec<Event>) {
        let Ok(v) = parse(data.as_bytes()) else {
            out.push(Event::error(
                "err.llm.bad_response",
                &[("reason", "json".to_string())],
            ));
            return;
        };
        if let Some(e) = v.get("error") {
            let s = |k: &str| e.get(k).and_then(Value::str).unwrap_or("").to_string();
            out.push(api_error(0, &s("status"), &s("message")));
            self.stopped = true;
            return;
        }
        if let Some(c) = v.get("candidates").and_then(|c| c.at(0)) {
            self.parts(c, out);
            if let Some(r) = c.get("finishReason").and_then(Value::str) {
                self.reason = Some(stop_reason(r));
            }
        }
        if let Some(u) = v.get("usageMetadata") {
            let n = |k: &str| u.get(k).and_then(Value::f64).unwrap_or(0.0) as u64;
            let cached = n("cachedContentTokenCount");
            self.usage = Some(Usage {
                input: n("promptTokenCount").saturating_sub(cached),
                output: n("candidatesTokenCount"),
                cache_read: cached,
                ..Usage::default()
            });
        }
    }

    /// Gemini has no end marker: the usage and the stop come when the bytes end.
    fn end(&mut self, out: &mut Vec<Event>) {
        if self.stopped {
            return;
        }
        if let Some(r) = self.reason {
            self.stopped = true;
            out.extend(self.usage.take().map(Event::Usage));
            out.push(Event::Stop(if self.calls > 0 && r == StopReason::End {
                StopReason::Tool
            } else {
                r
            }));
        }
    }
}
