//! Chat Completions (OpenAI and every compatible server: Mistral, Groq, OpenRouter, Ollama, llama.cpp, LM Studio, vLLM): request body and stream decoder.

use mzlab_core::json::{parse, Out, Value};

use super::{api_error, json_text, plain, Ctx, Decoder};
use crate::conv::{Message, Part, Role};
use crate::event::{Event, StopReason, Usage};

pub(super) fn body(c: &Ctx) -> String {
    let mut o = Out::default();
    o.raw("{\"model\":")
        .str(&c.model.id)
        .raw(",\"max_tokens\":")
        .int(c.max as i64)
        .raw(",\"stream\":true,\"stream_options\":{\"include_usage\":true},\"messages\":[");
    let mut first = true;
    let mut sep = |o: &mut Out| {
        if !std::mem::take(&mut first) {
            o.raw(",");
        }
    };
    if !c.system.is_empty() {
        sep(&mut o);
        o.raw("{\"role\":\"system\",\"content\":")
            .str(&c.system)
            .raw("}");
    }
    for m in &c.conv.messages {
        match m.role {
            Role::User => {
                sep(&mut o);
                user(&mut o, m);
            }
            Role::Assistant => {
                sep(&mut o);
                assistant(&mut o, m);
            }
            Role::Tool => {
                for p in &m.parts {
                    if let Part::ToolResult { id, content, .. } = p {
                        sep(&mut o);
                        o.raw("{\"role\":\"tool\",\"tool_call_id\":")
                            .str(id)
                            .raw(",\"content\":")
                            .str(&json_text(content))
                            .raw("}");
                    }
                }
            }
        }
    }
    o.raw("]");
    if !c.tools.is_empty() {
        o.raw(",\"tools\":[");
        for (i, t) in c.tools.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.raw("{\"type\":\"function\",\"function\":{\"name\":")
                .str(t.name)
                .raw(",\"description\":")
                .str(t.description)
                .raw(",\"parameters\":")
                .raw(&t.input_schema)
                .raw("}}");
        }
        o.raw("]");
    }
    o.raw("}");
    o.0
}

fn user(o: &mut Out, m: &Message) {
    o.raw("{\"role\":\"user\",\"content\":");
    if m.parts.iter().any(|p| matches!(p, Part::Image { .. })) {
        o.raw("[");
        for (i, p) in m.parts.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            match (plain(p), p) {
                (Some(t), _) => {
                    o.raw("{\"type\":\"text\",\"text\":").str(&t).raw("}");
                }
                (None, Part::Image { png_b64 }) => {
                    o.raw("{\"type\":\"image_url\",\"image_url\":{\"url\":")
                        .str(&format!("data:image/png;base64,{png_b64}"))
                        .raw("}}");
                }
                _ => {}
            }
        }
        o.raw("]");
    } else {
        o.str(&texts(m));
    }
    o.raw("}");
}

fn texts(m: &Message) -> String {
    m.parts
        .iter()
        .filter_map(plain)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn assistant(o: &mut Out, m: &Message) {
    let t = texts(m);
    o.raw("{\"role\":\"assistant\",\"content\":");
    if t.is_empty() {
        o.raw("null");
    } else {
        o.str(&t);
    }
    let calls: Vec<&Part> = m
        .parts
        .iter()
        .filter(|p| matches!(p, Part::ToolCall { .. }))
        .collect();
    if !calls.is_empty() {
        o.raw(",\"tool_calls\":[");
        for (i, p) in calls.iter().enumerate() {
            if let Part::ToolCall { id, name, args } = p {
                if i > 0 {
                    o.raw(",");
                }
                o.raw("{\"id\":")
                    .str(id)
                    .raw(",\"type\":\"function\",\"function\":{\"name\":")
                    .str(name)
                    .raw(",\"arguments\":")
                    .str(&json_text(args))
                    .raw("}}");
            }
        }
        o.raw("]");
    }
    o.raw("}");
}

#[derive(Default)]
pub(super) struct Dec {
    reason: Option<StopReason>,
    stopped: bool,
    /// Tool calls that are open: (index in the answer, id).
    open: Vec<(u64, String)>,
}

impl Dec {
    fn close(&mut self, out: &mut Vec<Event>) {
        out.extend(self.open.drain(..).map(|(_, id)| Event::ToolCallEnd { id }));
    }
}

impl Decoder for Dec {
    fn frame(&mut self, _name: &str, data: &str, out: &mut Vec<Event>) {
        if data.trim() == "[DONE]" {
            self.end(out);
            return;
        }
        let Ok(v) = parse(data.as_bytes()) else {
            out.push(Event::error(
                "err.llm.bad_response",
                &[("reason", "json".to_string())],
            ));
            return;
        };
        if let Some(e) = v.get("error") {
            let s = |k: &str| e.get(k).and_then(Value::str).unwrap_or("").to_string();
            out.push(api_error(0, &s("type"), &s("message")));
            return;
        }
        if let Some(c) = v.get("choices").and_then(|c| c.at(0)) {
            let d = c.get("delta");
            if let Some(t) = d
                .and_then(|d| d.get("content"))
                .and_then(Value::str)
                .filter(|t| !t.is_empty())
            {
                out.push(Event::TextDelta(t.to_string()));
            }
            for tc in d
                .and_then(|d| d.get("tool_calls"))
                .map_or(&[][..], Value::arr)
            {
                let index = tc.get("index").and_then(Value::f64).unwrap_or(0.0) as u64;
                let f = tc.get("function");
                if let Some(id) = tc.get("id").and_then(Value::str).filter(|i| !i.is_empty()) {
                    self.open.push((index, id.to_string()));
                    out.push(Event::ToolCallStart {
                        id: id.to_string(),
                        name: f
                            .and_then(|f| f.get("name"))
                            .and_then(Value::str)
                            .unwrap_or("")
                            .to_string(),
                    });
                }
                if let Some(a) = f
                    .and_then(|f| f.get("arguments"))
                    .and_then(Value::str)
                    .filter(|a| !a.is_empty())
                {
                    out.push(Event::ToolCallDelta(a.to_string()));
                }
            }
            if let Some(r) = c.get("finish_reason").and_then(Value::str) {
                self.close(out);
                self.reason = Some(match r {
                    "tool_calls" | "function_call" => StopReason::Tool,
                    "length" => StopReason::Length,
                    "content_filter" => StopReason::Refusal,
                    _ => StopReason::End,
                });
            }
        }
        if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
            let n = |x: Option<&Value>| x.and_then(Value::f64).unwrap_or(0.0) as u64;
            let cached = n(u
                .get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens")));
            out.push(Event::Usage(Usage {
                input: n(u.get("prompt_tokens")).saturating_sub(cached),
                output: n(u.get("completion_tokens")),
                cache_read: cached,
            }));
        }
    }

    /// `[DONE]`, or the end of the bytes after a finish reason.
    fn end(&mut self, out: &mut Vec<Event>) {
        if let (Some(r), false) = (self.reason, self.stopped) {
            self.stopped = true;
            out.push(Event::Stop(r));
        }
    }
}
