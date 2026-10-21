//! The LLM side of mzLab, kept apart from the engine: `tools` builds the catalog `strumenti/1` from the step registry; `provider` turns a conversation
//! into a request and a byte stream into events for each vendor; `fake`, `log` and `cost` serve the tests and the transport. `agent` is the loop as a
//! pure state machine, `policy` decides every call by level, `schema` checks the arguments, `plan` checks a proposed plan, `context` builds `contesto/1`
//! and `eval` scores the evaluation cases. Used by `mzlab-cli llm`.
//! No network, no key, no dependency beyond `mzlab-core`.

pub mod agent;
pub mod compact;
pub mod context;
pub mod conv;
pub mod cost;
pub mod eval;
pub mod event;
pub mod fake;
pub mod log;
pub mod plan;
pub mod policy;
pub mod provider;
pub mod schema;
pub mod tools;
