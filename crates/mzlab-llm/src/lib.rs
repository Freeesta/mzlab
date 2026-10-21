//! The LLM side of mzLab, kept apart from the engine: `tools` builds the catalog `strumenti/1` from the step registry; `provider` turns a conversation
//! into a request and a byte stream into events for each vendor; `fake`, `log` and `cost` serve the tests and the transport. Used by `mzlab-cli llm`.
//! No network, no key, no dependency beyond `mzlab-core`.

pub mod conv;
pub mod cost;
pub mod event;
pub mod fake;
pub mod log;
pub mod provider;
pub mod tools;
