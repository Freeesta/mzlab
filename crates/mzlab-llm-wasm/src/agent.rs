//! The loop of the assistant for the pages of the site: `mzlab_llm::agent::State` behind wasm-bindgen. The page feeds it the user's message, the normalized events of
//! the answer, the results of the tools and the confirmations; it gets back JSON lines (`send`, `call`, `ask`, `show`, `log`, `done`). The plans are checked by the page
//! (the registry of steps is not in this module). A loaded recording answers every `send` by itself, so tests run with no transport. Used by `mzlab/web/llm/llm-agente.js`.
use std::collections::VecDeque;

use mzlab_core::error::Error;
use mzlab_core::json::{parse, Out, Value};
use mzlab_llm::agent::{step, Input, Output, State};
use mzlab_llm::event::Event;
use mzlab_llm::fake::Replay;
use mzlab_llm::policy::{Level, Profile};
use mzlab_llm::tools::{parse_catalog, value};
use wasm_bindgen::prelude::*;

use crate::{fail, find};

fn line(o: &Output, outcome: &str) -> String {
    let mut j = Out::default();
    match o {
        Output::Send(r) => j.raw("{\"o\":\"send\",\"request\":").raw(&r.to_json()),
        Output::CallTool { id, name, args } => {
            j.raw("{\"o\":\"call\",\"id\":")
                .str(id)
                .raw(",\"name\":")
                .str(name)
                .raw(",\"args\":");
            value(&mut j, args);
            &mut j
        }
        Output::AskConfirm { id, summary } => j
            .raw("{\"o\":\"ask\",\"id\":")
            .str(id)
            .raw(",\"summary\":")
            .str(summary),
        Output::Show(t) => j.raw("{\"o\":\"show\",\"text\":").str(t),
        Output::Log(l) => j.raw("{\"o\":\"log\",\"line\":").raw(l),
        Output::Done => j.raw("{\"o\":\"done\",\"outcome\":").str(outcome),
    };
    j.raw("}\n");
    j.0
}

/// One conversation of the assistant: the state machine, the level and (in tests) a recording.
#[wasm_bindgen]
pub struct Agent {
    st: State,
    replay: Option<Replay>,
}

#[wasm_bindgen]
impl Agent {
    /// `tools_json`: `strumenti/1`; `level`: `L0|L1|L2`; `research`: false = the profile «corso», where every turn fails with `err.llm.profile.course`.
    #[wasm_bindgen(constructor)]
    pub fn new(
        tools_json: &str,
        provider_id: &str,
        model: &str,
        level: &str,
        research: bool,
    ) -> Result<Agent, String> {
        let level = Level::parse(level)
            .ok_or_else(|| fail(Error::new("err.llm.conversation").with("field", "level")))?;
        let profile = if research {
            Profile::Research
        } else {
            Profile::Course
        };
        let mut st = State::new(
            find(provider_id)?,
            model,
            parse_catalog(tools_json).map_err(fail)?,
            Vec::new(),
            level,
            profile,
        );
        st.external_check = true;
        Ok(Agent { st, replay: None })
    }

    /// A recording `llmreg/1` that answers the requests instead of a transport; returns its number of turns.
    pub fn replay_load(&mut self, text: &str, strict: bool) -> Result<u32, String> {
        let r = Replay::parse(text, strict).map_err(fail)?;
        let n = r.len() as u32;
        self.replay = Some(r);
        Ok(n)
    }

    /// A user message with the optional `contesto/1` object that goes with it.
    pub fn user(&mut self, text: &str, context_json: &str) -> Result<String, String> {
        if !context_json.is_empty() {
            let c = parse(context_json.as_bytes()).map_err(fail)?;
            self.st.add_context(c);
        }
        Ok(self.go(Input::User(text.to_string())))
    }

    /// One normalized event line of the answer (`LlmStream`, `http_error`, the routes of the app).
    pub fn event(&mut self, event_line: &str) -> Result<String, String> {
        let v = parse(event_line.as_bytes()).map_err(fail)?;
        let e = Event::from_value(&v).map_err(fail)?;
        Ok(self.go(Input::Event(e)))
    }

    /// What the page answers to a `call`.
    pub fn result(&mut self, id: &str, ok: bool, content_json: &str) -> Result<String, String> {
        let content = parse(content_json.as_bytes()).unwrap_or(Value::Null);
        Ok(self.go(Input::ToolResult {
            id: id.to_string(),
            ok,
            content,
        }))
    }

    /// What the user answers to an `ask`.
    pub fn confirm(&mut self, id: &str, yes: bool) -> String {
        self.go(Input::Confirm {
            id: id.to_string(),
            yes,
        })
    }

    pub fn stop(&mut self) -> String {
        self.go(Input::Stop)
    }

    /// The conversation as `llm-conv/1`, what the routes of the app take.
    pub fn conversation(&self) -> String {
        self.st.conversation().to_json()
    }

    /// The `llmlog/1` text so far.
    pub fn log(&self) -> String {
        self.st.log().to_string()
    }

    /// `end|length|refusal|stopped|error:<key>`, empty while a turn runs.
    pub fn outcome(&self) -> String {
        self.st.outcome().unwrap_or("").to_string()
    }

    /// Tokens `[in, out]` reported by the provider so far.
    pub fn usage(&self) -> Vec<f64> {
        let (i, o) = self.st.usage;
        vec![i as f64, o as f64]
    }
}

impl Agent {
    fn go(&mut self, input: Input) -> String {
        let out = step(&mut self.st, input);
        self.drive(out)
    }

    /// The lines of the outputs; with a recording, every `send` is answered at once by the next turn of it.
    fn drive(&mut self, first: Vec<Output>) -> String {
        let mut text = String::new();
        let mut work: VecDeque<Output> = first.into();
        while let Some(o) = work.pop_front() {
            text.push_str(&line(&o, self.st.outcome().unwrap_or("")));
            let (Output::Send(req), Some(r)) = (&o, self.replay.as_mut()) else {
                continue;
            };
            let events = r.respond(req).unwrap_or_else(|e| {
                vec![Event::Error {
                    key: e.key.to_string(),
                    params: e
                        .params
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                }]
            });
            for e in events {
                work.extend(step(&mut self.st, Input::Event(e)));
            }
        }
        text
    }
}
