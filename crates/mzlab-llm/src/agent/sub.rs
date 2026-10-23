//! The calls of the machine to its subagents (`assist::Role`): today the router, which runs before every user message. Child of `agent` (it works on the
//! fields of `State`); the roles and the prompts come from the page, never from the program.

use mzlab_core::error::Result;
use mzlab_core::json::Value;

use super::{json, note, quoted, send, Output, State};
use crate::assist::{Prompts, Role};
use crate::router::{self, Route};

/// The subagents the machine may call: their roles, their prompts (handed over by the page) and the state the router sees.
pub(super) struct Assist {
    roles: Vec<Role>,
    prompts: Prompts,
    state: Value,
}

impl State {
    /// Turns the router on: before every user message the machine asks it for the route and the data to attach. Fails when a prompt file is missing.
    pub fn enable_assist(&mut self, roles: Vec<Role>, prompts: Prompts) -> Result<()> {
        prompts.check(&roles)?;
        self.assist = Some(Assist {
            roles,
            prompts,
            state: Value::Null,
        });
        Ok(())
    }

    /// What the router sees of the session (`phase`, `files_open`, `plan`, `pending_questions`), set by the page before a user message.
    pub fn set_assist_state(&mut self, state: Value) {
        if let Some(a) = &mut self.assist {
            a.state = state;
        }
    }

    /// The route chosen for the last user message; `None` without a router or when its answer was not valid.
    pub fn route(&self) -> Option<&Route> {
        self.route.as_ref()
    }
}

/// Asks the router for the route of `text`; false when there is no router (the turn goes straight to the orchestrator).
pub(super) fn ask_router(s: &mut State, text: &str, out: &mut Vec<Output>) -> bool {
    let Some(a) = &s.assist else { return false };
    let Some(role) = a.roles.iter().find(|r| r.role == "router") else {
        return false;
    };
    let prompt = a.prompts.prompt(&role.prompt_file).unwrap_or("");
    match router::request(&s.provider, role, prompt, &a.state, text) {
        Ok(request) => {
            let sha = a.prompts.digest(&role.prompt_file).unwrap_or_default();
            s.sub = Some("router".to_string());
            let fields = [("prompt_sha256", quoted(&sha)), ("role", quoted("router"))];
            note(s, out, "sub", &fields);
            let role = "router".to_string();
            out.push(Output::Sub { role, request });
            true
        }
        Err(e) => {
            note(s, out, "route", &[("error", quoted(e.key))]);
            false
        }
    }
}

/// The answer of a subagent. A router answer that is missing or outside the schema is logged and the turn goes on without a route.
pub(super) fn sub_result(
    s: &mut State,
    role: &str,
    body: std::result::Result<String, String>,
    out: &mut Vec<Output>,
) {
    if s.over || s.sub.as_deref() != Some(role) {
        return;
    }
    s.sub = None;
    match body.and_then(|b| router::parse(&b).map_err(|e| e.key.to_string())) {
        Ok(r) => {
            note(s, out, "route", &[("answer", json(&r.0))]);
            s.route = Some(r);
        }
        Err(key) => note(s, out, "route", &[("error", quoted(&key))]),
    }
    send(s, out);
}
