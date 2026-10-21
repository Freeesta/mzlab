//! The loop of the assistant as a pure state machine: no I/O, no clock. The transport feeds `Input`s (the user's message, the events of the stream,
//! the results of tools, confirmations, stop) and executes the `Output`s (a request to send, a tool to call, a confirmation to ask). The policy
//! decides every call, the arguments are checked against the schema of the tool, a proposed plan goes through `flow::check`, and every step is a line
//! of `llmlog/1`. Used by the transport of the page and the desktop and by `mzlab-cli llm eval`.

use std::collections::VecDeque;

use mzlab_core::flow::Platform;
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::step::Step;

use crate::conv::Conversation;
use crate::conv::{Message, Part, Role};
use crate::event::{Event, StopReason};
use crate::log::Log;
use crate::plan::problems;
use crate::policy::{decide, Counters, Decision, Level, Profile};
use crate::provider::{request, Provider, Request};
use crate::tools::{value, Class, Tool};

/// The system text, English, versioned by its SHA-256 in the log.
pub const SYSTEM: &str = include_str!("../data/sistema-1.md");
/// A proposed plan that still has problems after this many tries ends the turn.
pub use crate::compact::{compact, estimate};
pub const MAX_PLAN_TRIES: u32 = 3;

pub enum Input {
    User(String),
    Event(Event),
    ToolResult {
        id: String,
        ok: bool,
        content: Value,
    },
    Confirm {
        id: String,
        yes: bool,
    },
    Stop,
}

pub enum Output {
    Send(Request),
    CallTool {
        id: String,
        name: String,
        args: Value,
    },
    AskConfirm {
        id: String,
        summary: String,
    },
    /// A piece of the model's text for the user.
    Show(String),
    /// A complete `llmlog/1` line.
    Log(String),
    Done,
}

#[derive(Clone)]
struct Call {
    id: String,
    name: String,
    args: Value,
}

pub struct State {
    pub platform: Platform,
    provider: Provider,
    tools: Vec<Tool>,
    registry: Vec<Step>,
    level: Level,
    profile: Profile,
    conv: Conversation,
    counters: Counters,
    turn: u32,
    log: Log,
    text: String,
    open: Option<(String, String, String)>,
    calls: Vec<Call>,
    queue: VecDeque<Call>,
    results: Vec<Part>,
    waiting: Option<Call>,
    confirming: Option<Call>,
    tries: u32,
    stopping: bool,
    over: bool,
    outcome: Option<String>,
    context: Option<Value>,
}

fn json(v: &Value) -> String {
    let mut o = Out::default();
    value(&mut o, v);
    o.0
}

impl State {
    pub fn new(
        provider: Provider,
        model: &str,
        tools: Vec<Tool>,
        registry: Vec<Step>,
        level: Level,
        profile: Profile,
    ) -> State {
        let conv = Conversation {
            provider: provider.id.clone(),
            model: model.to_string(),
            system: SYSTEM.to_string(),
            max_output_tokens: None,
            messages: Vec::new(),
        };
        State {
            platform: Platform::Native,
            provider,
            tools,
            registry,
            level,
            profile,
            conv,
            counters: Counters::default(),
            turn: 0,
            log: Log::new(),
            text: String::new(),
            open: None,
            calls: Vec::new(),
            queue: VecDeque::new(),
            results: Vec::new(),
            waiting: None,
            confirming: None,
            tries: 0,
            stopping: false,
            over: true,
            outcome: None,
            context: None,
        }
    }

    /// A `contesto/1` object that goes with the next user message.
    pub fn add_context(&mut self, context: Value) {
        self.context = Some(context);
    }

    /// The whole `llmlog/1` text so far.
    pub fn log(&self) -> &str {
        self.log.text()
    }

    pub fn conversation(&self) -> &Conversation {
        &self.conv
    }

    pub fn counters(&self) -> Counters {
        self.counters
    }

    /// How the last turn ended: `end`, `length`, `refusal`, `stopped` or `error:<key>`; `None` while it runs.
    pub fn outcome(&self) -> Option<&str> {
        self.outcome.as_deref()
    }
}

fn quoted(s: &str) -> String {
    let mut o = Out::default();
    o.str(s);
    o.0
}

fn finish(s: &mut State, out: &mut Vec<Output>) {
    if s.over {
        return;
    }
    s.over = true;
    let pending: Vec<Call> = s
        .waiting
        .take()
        .into_iter()
        .chain(s.confirming.take())
        .chain(s.queue.drain(..))
        .collect();
    for c in &pending {
        reject(s, c, "err.llm.tool.skipped");
    }
    if !s.results.is_empty() {
        let parts = std::mem::take(&mut s.results);
        s.conv.messages.push(Message {
            role: Role::Tool,
            parts,
        });
    }
    out.push(Output::Done);
}

fn fail(s: &mut State, out: &mut Vec<Output>, key: &str) {
    s.outcome = Some(format!("error:{key}"));
    finish(s, out);
}

fn note(s: &mut State, out: &mut Vec<Output>, kind: &str, fields: &[(&'static str, String)]) {
    match s.log.record(s.turn, kind, fields) {
        Ok(()) => out.push(Output::Log(s.log.last().to_string())),
        Err(e) => fail(s, out, e.key),
    }
}

fn send(s: &mut State, out: &mut Vec<Output>) {
    if s.over {
        return;
    }
    let limit = s
        .provider
        .model(&s.conv.model)
        .map_or(0, |m| m.context_tokens);
    compact(&mut s.conv, &s.tools, limit);
    match request(&s.provider, &s.conv, &s.tools) {
        Ok(r) => out.push(Output::Send(r)),
        Err(e) => fail(s, out, e.key),
    }
}

fn user(s: &mut State, text: &str, out: &mut Vec<Output>) {
    if s.profile == Profile::Course {
        return fail(s, out, "err.llm.profile.course");
    }
    s.turn += 1;
    s.counters = Counters::default();
    s.tries = 0;
    s.stopping = false;
    s.over = false;
    s.outcome = None;
    s.queue.clear();
    s.waiting = None;
    s.confirming = None;
    let mut parts: Vec<Part> = s.context.take().into_iter().map(Part::Context).collect();
    parts.push(Part::Text(text.to_string()));
    s.conv.messages.push(Message {
        role: Role::User,
        parts,
    });
    note(s, out, "user", &[("bytes", text.len().to_string())]);
    send(s, out);
}

fn close_call(s: &mut State) {
    if let Some((id, name, args)) = s.open.take() {
        let args = if args.trim().is_empty() { "{}" } else { &args };
        let args = parse(args.as_bytes()).unwrap_or(Value::Null);
        s.calls.push(Call { id, name, args });
    }
}

fn event(s: &mut State, e: Event, out: &mut Vec<Output>) {
    if s.over {
        return;
    }
    if let Err(err) = s.log.event(s.turn, &e) {
        return fail(s, out, err.key);
    }
    out.push(Output::Log(s.log.last().to_string()));
    match e {
        Event::TextDelta(t) => {
            s.text.push_str(&t);
            out.push(Output::Show(t));
        }
        Event::ToolCallStart { id, name } => s.open = Some((id, name, String::new())),
        Event::ToolCallDelta(j) => {
            if let Some(o) = &mut s.open {
                o.2.push_str(&j);
            }
        }
        Event::ToolCallEnd { .. } => close_call(s),
        Event::Usage(_) => {}
        Event::Stop(r) => stopped(s, r, out),
        Event::Error { key, .. } => fail(s, out, &key),
    }
}

fn stopped(s: &mut State, r: StopReason, out: &mut Vec<Output>) {
    close_call(s);
    let mut parts = Vec::new();
    if !s.text.is_empty() {
        parts.push(Part::Text(std::mem::take(&mut s.text)));
    }
    let calls = std::mem::take(&mut s.calls);
    for c in &calls {
        parts.push(Part::ToolCall {
            id: c.id.clone(),
            name: c.name.clone(),
            args: c.args.clone(),
        });
    }
    if !parts.is_empty() {
        s.conv.messages.push(Message {
            role: Role::Assistant,
            parts,
        });
    }
    if calls.is_empty() {
        s.outcome = Some(r.name().to_string());
        return finish(s, out);
    }
    s.queue.extend(calls);
    advance(s, out);
}

fn advance(s: &mut State, out: &mut Vec<Output>) {
    while s.waiting.is_none() && s.confirming.is_none() && !s.over {
        match s.queue.pop_front() {
            Some(c) => dispatch(s, c, out),
            None => return flush(s, out),
        }
    }
}

fn flush(s: &mut State, out: &mut Vec<Output>) {
    if !s.results.is_empty() {
        let parts = std::mem::take(&mut s.results);
        s.conv.messages.push(Message {
            role: Role::Tool,
            parts,
        });
    }
    if s.stopping {
        finish(s, out);
    } else {
        send(s, out);
    }
}

/// The result of a call that did not run: an error key (and what the tool said) goes back to the model.
fn answer(s: &mut State, c: &Call, ok: bool, content: String) {
    let content = parse(content.as_bytes()).unwrap_or(Value::Null);
    s.results.push(Part::ToolResult {
        id: c.id.clone(),
        ok,
        content,
    });
}

fn reject(s: &mut State, c: &Call, key: &str) {
    answer(s, c, false, format!("{{\"error_key\":{}}}", quoted(key)));
}

fn dispatch(s: &mut State, c: Call, out: &mut Vec<Output>) {
    note(
        s,
        out,
        "call",
        &[
            ("args", json(&c.args)),
            ("id", quoted(&c.id)),
            ("name", quoted(&c.name)),
        ],
    );
    let Some(tool) = s.tools.iter().find(|t| t.name == c.name).cloned() else {
        return reject(s, &c, "err.llm.tool.unknown");
    };
    let d = decide(&tool, &c.args, s.level, &s.counters, s.profile);
    let (word, key) = match &d {
        Decision::Allow => ("allow", ""),
        Decision::Confirm => ("confirm", ""),
        Decision::Deny { key } => ("deny", *key),
    };
    note(
        s,
        out,
        "decision",
        &[
            ("decision", quoted(word)),
            ("id", quoted(&c.id)),
            ("key", quoted(key)),
            ("tool", quoted(&c.name)),
        ],
    );
    match d {
        Decision::Allow => run_call(s, c, &tool, out),
        Decision::Confirm => {
            out.push(Output::AskConfirm {
                id: c.id.clone(),
                summary: format!("{} {}", c.name, json(&c.args)),
            });
            s.confirming = Some(c);
        }
        Decision::Deny { key }
            if key == "err.llm.limit.calls" || key == "err.llm.profile.course" =>
        {
            reject(s, &c, key);
            fail(s, out, key);
        }
        Decision::Deny { key } => reject(s, &c, key),
    }
}

/// `flow_check` and `flow_propose_plan` are answered here (the plan is checked by `flow::check`); every other allowed call is the page's.
fn run_call(s: &mut State, c: Call, tool: &Tool, out: &mut Vec<Output>) {
    s.counters.calls += 1;
    if tool.class == Class::Run {
        s.counters.runs += 1;
    }
    if c.name != "flow_check" && c.name != "flow_propose_plan" {
        return call_page(s, c, out);
    }
    match problems(&c.args, &s.registry, s.platform) {
        Err(key) => reject(s, &c, &key),
        Ok((_, text)) if c.name == "flow_check" => answer(s, &c, true, text),
        Ok((0, _)) => call_page(s, c, out),
        Ok((_, text)) => {
            s.tries += 1;
            answer(s, &c, false, text);
            if s.tries >= MAX_PLAN_TRIES {
                fail(s, out, "err.llm.plan.tries");
            }
        }
    }
}

fn call_page(s: &mut State, c: Call, out: &mut Vec<Output>) {
    out.push(Output::CallTool {
        id: c.id.clone(),
        name: c.name.clone(),
        args: c.args.clone(),
    });
    s.waiting = Some(c);
}

fn result(s: &mut State, id: &str, ok: bool, content: Value, out: &mut Vec<Output>) {
    if s.waiting.as_ref().is_none_or(|c| c.id != id) {
        return;
    }
    s.waiting = None;
    s.results.push(Part::ToolResult {
        id: id.to_string(),
        ok,
        content,
    });
    note(
        s,
        out,
        "result",
        &[("id", quoted(id)), ("ok", ok.to_string())],
    );
    advance(s, out);
}

fn confirm(s: &mut State, id: &str, yes: bool, out: &mut Vec<Output>) {
    if s.confirming.as_ref().is_none_or(|c| c.id != id) {
        return;
    }
    let Some(c) = s.confirming.take() else { return };
    note(
        s,
        out,
        "confirm",
        &[("id", quoted(id)), ("yes", yes.to_string())],
    );
    let tool = s.tools.iter().find(|t| t.name == c.name).cloned();
    match (yes, tool) {
        (true, Some(t)) => run_call(s, c, &t, out),
        _ => reject(s, &c, "err.llm.tool.declined"),
    }
    advance(s, out);
}

fn stop(s: &mut State, out: &mut Vec<Output>) {
    s.stopping = true;
    s.outcome = Some("stopped".to_string());
    if s.waiting.is_none() {
        flush(s, out);
    }
}

/// One move of the machine.
pub fn step(s: &mut State, input: Input) -> Vec<Output> {
    let mut out = Vec::new();
    match input {
        Input::User(t) => user(s, &t, &mut out),
        Input::Event(e) => event(s, e, &mut out),
        Input::ToolResult { id, ok, content } => result(s, &id, ok, content, &mut out),
        Input::Confirm { id, yes } => confirm(s, &id, yes, &mut out),
        Input::Stop => stop(s, &mut out),
    }
    out
}
