//! Keeps a conversation inside the context window: a token estimate (`ceil(bytes / 3)`, no tokenizer is shipped) and `compact`, which replaces the
//! oldest tool results and then the oldest context parts by short markers. Used by `agent` before every request.

use mzlab_core::json::{Out, Value};

use crate::conv::{Conversation, Part};
use crate::tools::{value, Tool};

fn json(v: &Value) -> String {
    let mut o = Out::default();
    value(&mut o, v);
    o.0
}

fn part_bytes(p: &Part) -> usize {
    match p {
        Part::Text(t) => t.len(),
        Part::Context(v) => json(v).len(),
        Part::Image { png_b64 } => png_b64.len(),
        Part::ToolCall { name, args, .. } => name.len() + json(args).len(),
        Part::ToolResult { content, .. } => json(content).len(),
    }
}

/// A prudent token estimate, `ceil(bytes / 3)`: no tokenizer is shipped.
pub fn estimate(c: &Conversation, tools: &[Tool]) -> u64 {
    let parts: usize = c
        .messages
        .iter()
        .flat_map(|m| &m.parts)
        .map(part_bytes)
        .sum();
    let schemas: usize = tools
        .iter()
        .map(|t| t.input_schema.len() + t.description.len())
        .sum();
    ((parts + schemas + c.system.len()) as u64).div_ceil(3)
}

fn tool_name(c: &Conversation, id: &str) -> Option<String> {
    c.messages
        .iter()
        .flat_map(|m| &m.parts)
        .find_map(|p| match p {
            Part::ToolCall { id: i, name, .. } if i == id => Some(name.clone()),
            _ => None,
        })
}

/// The id of the most recent result of `flow_get_plan`: the current plan stays whole.
fn current_plan(c: &Conversation) -> Option<String> {
    c.messages
        .iter()
        .flat_map(|m| &m.parts)
        .rev()
        .find_map(|p| match p {
            Part::ToolResult { id, .. } if tool_name(c, id).as_deref() == Some("flow_get_plan") => {
                Some(id.clone())
            }
            _ => None,
        })
}

fn summarize_one(c: &mut Conversation, keep: Option<&str>) -> bool {
    let names: Vec<(String, Option<String>)> = c
        .messages
        .iter()
        .flat_map(|m| &m.parts)
        .filter_map(|p| match p {
            Part::ToolResult { id, .. } => Some((id.clone(), tool_name(c, id))),
            _ => None,
        })
        .collect();
    for m in &mut c.messages {
        for p in &mut m.parts {
            let Part::ToolResult { id, ok, content } = p else {
                continue;
            };
            if content.get("summarized").is_some() || keep == Some(id.as_str()) {
                continue;
            }
            let name = names
                .iter()
                .find(|(i, _)| i == id)
                .and_then(|(_, n)| n.clone())
                .unwrap_or_default();
            *content = Value::Obj(vec![
                ("summarized".into(), Value::Bool(true)),
                ("tool".into(), Value::Str(name)),
                ("ok".into(), Value::Bool(*ok)),
            ]);
            return true;
        }
    }
    false
}

fn drop_old_context(c: &mut Conversation) -> bool {
    let last = c
        .messages
        .iter()
        .rposition(|m| m.parts.iter().any(|p| matches!(p, Part::Context(_))));
    for (k, m) in c.messages.iter_mut().enumerate() {
        if Some(k) == last {
            continue;
        }
        if let Some(p) = m.parts.iter_mut().find(|p| matches!(p, Part::Context(_))) {
            *p = Part::Text("[older context summarized]".to_string());
            return true;
        }
    }
    false
}

/// Above 70 % of the context the oldest results of tools become one line each, then older `contesto/1` blocks; the current plan and the last
/// `contesto/1` stay whole. Returns how many parts were replaced.
pub fn compact(c: &mut Conversation, tools: &[Tool], context_tokens: u64) -> usize {
    let mut n = 0;
    if context_tokens == 0 {
        return 0;
    }
    let keep = current_plan(c);
    while estimate(c, tools) > context_tokens * 7 / 10
        && (summarize_one(c, keep.as_deref()) || drop_old_context(c))
    {
        n += 1;
    }
    n
}
