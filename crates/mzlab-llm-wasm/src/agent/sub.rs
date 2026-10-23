//! The subagent calls of `Agent` for the page: turning them on, the state the router sees and the readings that passed their check (`Assist.check`).
//! Child of `agent` (it works on the machine behind `Agent`).
use mzlab_core::error::Error;
use mzlab_core::json::parse;
use mzlab_llm::assist::roles as assist_roles;
use mzlab_llm::provider::Request;
use wasm_bindgen::prelude::*;

use super::Agent;
use crate::fail;

#[wasm_bindgen]
impl Agent {
    /// Turns the subagents on: `roles_json` (`assist-ruoli/1`), `prompts_json` (`[["file","text"],...]`) and the file of the phase prompt (`orchestrator.phase_prompt`, may be empty).
    pub fn enable_assist(
        &mut self,
        roles_json: &str,
        prompts_json: &str,
        phase_file: &str,
    ) -> Result<(), String> {
        let roles = assist_roles(roles_json).map_err(fail)?;
        let prompts = crate::assist::prompts_of(prompts_json)?;
        self.st.enable_assist(roles, prompts).map_err(fail)?;
        if phase_file.is_empty() {
            return Ok(());
        }
        self.st.set_phase_prompt(phase_file).map_err(fail)
    }

    /// What the router sees of the session (`phase`, `files_open`, `plan`, `pending_questions`), before a user message.
    pub fn set_assist_state(&mut self, state_json: &str) -> Result<(), String> {
        self.st
            .set_assist_state(parse(state_json.as_bytes()).map_err(fail)?);
        Ok(())
    }

    /// With a recording loaded: the events (lines) it gives for a request of a subagent, taken in order from the same recording as the model's turns.
    pub fn replay_sub(&mut self, request_json: &str) -> Result<String, String> {
        let req =
            Request::from_value(&parse(request_json.as_bytes()).map_err(fail)?).map_err(fail)?;
        let r = self
            .replay
            .as_mut()
            .ok_or_else(|| fail(Error::new("err.llm.replay.exhausted")))?;
        let events = r.respond(&req).map_err(fail)?;
        Ok(events.iter().map(|e| e.to_json() + "\n").collect())
    }

    /// Whether a recording answers the requests (the page then sends nothing over a transport).
    pub fn replaying(&self) -> bool {
        self.replay.is_some()
    }

    /// A reading that `Assist.check` accepted: it goes to the orchestrator with the next user message.
    pub fn reading(&mut self, role: &str, summary_json: &str) -> Result<(), String> {
        self.st
            .add_reading(role, parse(summary_json.as_bytes()).map_err(fail)?);
        Ok(())
    }
}
