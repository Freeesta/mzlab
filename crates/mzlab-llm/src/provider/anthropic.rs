//! The Messages API of Anthropic: the request body and the decoder of its event stream.

use mzlab_core::json::{parse, Out, Value};

use super::{api_error, json_text, plain, CacheTtl, Ctx, Decoder};
use crate::conv::{Part, Role};
use crate::event::{Event, StopReason, Usage};

pub(super) fn body(c: &Ctx) -> String {
    let mut o = Out::default();
    o.raw("{\"model\":")
        .str(&c.model.id)
        .raw(",\"max_tokens\":")
        .int(c.max as i64)
        .raw(",\"stream\":true");
    if c.opts.adaptive_thinking {
        o.raw(",\"thinking\":{\"type\":\"adaptive\"}");
    }
    output_config(&mut o, c);
    if c.opts.cache_auto {
        o.raw(",\"cache_control\":{\"type\":\"ephemeral\"}");
    }
    if !c.system.is_empty() {
        o.raw(",\"system\":");
        match c.opts.cache_ttl {
            None => {
                o.str(&c.system);
            }
            Some(ttl) => {
                o.raw("[{\"type\":\"text\",\"text\":")
                    .str(&c.system)
                    .raw(",\"cache_control\":{\"type\":\"ephemeral\",\"ttl\":\"")
                    .raw(if ttl == CacheTtl::Hour1 { "1h" } else { "5m" })
                    .raw("\"}}]");
            }
        }
    }
    o.raw(",\"messages\":[");
    for (i, m) in c.conv.messages.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        if m.role == Role::System {
            let text: Vec<String> = m.parts.iter().filter_map(plain).collect();
            o.raw("{\"role\":\"system\",\"content\":")
                .str(&text.join("\n"))
                .raw("}");
            continue;
        }
        o.raw(if m.role == Role::Assistant {
            "{\"role\":\"assistant\",\"content\":["
        } else {
            "{\"role\":\"user\",\"content\":["
        });
        for (j, p) in m.parts.iter().enumerate() {
            if j > 0 {
                o.raw(",");
            }
            part(&mut o, p);
        }
        o.raw("]}");
    }
    o.raw("]");
    if !c.tools.is_empty() {
        o.raw(",\"tools\":[");
        for (i, t) in c.tools.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.raw("{\"name\":")
                .str(t.name)
                .raw(",\"description\":")
                .str(t.description)
                .raw(",\"input_schema\":")
                .raw(&t.input_schema);
            if c.opts.strict_tools {
                o.raw(",\"strict\":true");
            }
            o.raw("}");
        }
        o.raw("]");
    }
    o.raw("}");
    o.0
}

/// `output_config` with the effort and the answer schema, when either is asked for.
fn output_config(o: &mut Out, c: &Ctx) {
    if c.opts.effort.is_none() && c.opts.format_schema.is_none() {
        return;
    }
    o.raw(",\"output_config\":{");
    if let Some(e) = c.opts.effort {
        o.raw("\"effort\":\"").raw(e.name()).raw("\"");
    }
    if let Some(f) = &c.opts.format_schema {
        if c.opts.effort.is_some() {
            o.raw(",");
        }
        o.raw("\"format\":{\"type\":\"json_schema\",\"schema\":")
            .raw(f)
            .raw("}");
    }
    o.raw("}");
}

fn part(o: &mut Out, p: &Part) {
    if let Some(t) = plain(p) {
        o.raw("{\"type\":\"text\",\"text\":").str(&t).raw("}");
        return;
    }
    match p {
        Part::Image { png_b64 } => {
            o.raw("{\"type\":\"image\",\"source\":{\"type\":\"base64\",\"media_type\":\"image/png\",\"data\":").str(png_b64).raw("}}");
        }
        Part::ToolCall { id, name, args } => {
            o.raw("{\"type\":\"tool_use\",\"id\":")
                .str(id)
                .raw(",\"name\":")
                .str(name)
                .raw(",\"input\":")
                .raw(&json_text(args))
                .raw("}");
        }
        Part::ToolResult { id, ok, content } => {
            o.raw("{\"type\":\"tool_result\",\"tool_use_id\":")
                .str(id)
                .raw(",\"content\":")
                .str(&json_text(content));
            if !ok {
                o.raw(",\"is_error\":true");
            }
            o.raw("}");
        }
        Part::Text(_) | Part::Context(_) => {}
    }
}

#[derive(Default)]
pub(super) struct Dec {
    usage: Usage,
    reason: Option<StopReason>,
    /// Tool-use blocks that are open, by block index.
    open: Vec<(u64, String)>,
}

impl Dec {
    fn block_start(&mut self, v: &Value, index: u64, out: &mut Vec<Event>) {
        let b = v.get("content_block");
        if b.and_then(|b| b.get("type")).and_then(Value::str) != Some("tool_use") {
            return;
        }
        let s = |k: &str| {
            b.and_then(|b| b.get(k))
                .and_then(Value::str)
                .unwrap_or("")
                .to_string()
        };
        self.open.push((index, s("id")));
        out.push(Event::ToolCallStart {
            id: s("id"),
            name: s("name"),
        });
    }

    fn block_delta(v: &Value, out: &mut Vec<Event>) {
        let d = v.get("delta");
        let s = |k: &str| {
            d.and_then(|d| d.get(k))
                .and_then(Value::str)
                .unwrap_or("")
                .to_string()
        };
        match d.and_then(|d| d.get("type")).and_then(Value::str) {
            Some("text_delta") => out.push(Event::TextDelta(s("text"))),
            Some("input_json_delta") => out.push(Event::ToolCallDelta(s("partial_json"))),
            _ => {}
        }
    }

    fn message_delta(&mut self, v: &Value, out: &mut Vec<Event>) {
        if let Some(n) = v
            .get("usage")
            .and_then(|u| u.get("output_tokens"))
            .and_then(Value::f64)
        {
            self.usage.output = n as u64;
        }
        self.reason = Some(stop_reason(v));
        out.push(Event::Usage(self.usage));
    }

    fn message_start(&mut self, v: &Value) {
        let u = v.get("message").and_then(|m| m.get("usage"));
        let n = |k: &str| u.and_then(|u| u.get(k)).and_then(Value::f64).unwrap_or(0.0) as u64;
        let detail = u.and_then(|u| u.get("cache_creation"));
        let w = |k: &str| detail.and_then(|d| d.get(k)).and_then(Value::f64);
        // Without the per-lifetime detail the total counts as the 5-minute rate (the default lifetime).
        let (w5, w1) = match (
            w("ephemeral_5m_input_tokens"),
            w("ephemeral_1h_input_tokens"),
        ) {
            (None, None) => (n("cache_creation_input_tokens"), 0),
            (a, b) => (a.unwrap_or(0.0) as u64, b.unwrap_or(0.0) as u64),
        };
        self.usage = Usage {
            input: n("input_tokens"),
            output: n("output_tokens"),
            cache_read: n("cache_read_input_tokens"),
            cache_write_5m: w5,
            cache_write_1h: w1,
        };
    }
}

fn stop_reason(v: &Value) -> StopReason {
    match v
        .get("delta")
        .and_then(|d| d.get("stop_reason"))
        .and_then(Value::str)
    {
        Some("tool_use") => StopReason::Tool,
        Some("max_tokens") => StopReason::Length,
        Some("refusal") => StopReason::Refusal,
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
        let index = v.get("index").and_then(Value::f64).unwrap_or(0.0) as u64;
        match v.get("type").and_then(Value::str) {
            Some("message_start") => self.message_start(&v),
            Some("content_block_start") => self.block_start(&v, index, out),
            Some("content_block_delta") => Dec::block_delta(&v, out),
            Some("content_block_stop") => {
                if let Some(p) = self.open.iter().position(|(i, _)| *i == index) {
                    out.push(Event::ToolCallEnd {
                        id: self.open.remove(p).1,
                    });
                }
            }
            Some("message_delta") => self.message_delta(&v, out),
            Some("message_stop") => out.push(Event::Stop(self.reason.unwrap_or(StopReason::End))),
            Some("error") => {
                let e = v.get("error");
                let s = |k: &str| {
                    e.and_then(|e| e.get(k))
                        .and_then(Value::str)
                        .unwrap_or("")
                        .to_string()
                };
                out.push(api_error(0, &s("type"), &s("message")));
            }
            _ => {}
        }
    }

    fn end(&mut self, _out: &mut Vec<Event>) {}
}
