//! The LLM side of mzLab, kept apart from the engine: `tools` builds the catalog `strumenti/1` from the step registry; `provider` turns a conversation
//! into a request and a byte stream into events for each vendor; `fake`, `log`, `cost` and `spend` (micro-dollar prices, cap and meter) serve the tests and the transport. `agent` is the loop as a
//! pure state machine, `policy` decides every call by level, `schema` checks the arguments, `plan` checks a proposed plan, `context` builds `contesto/1`,
//! `inspect` builds `ispezione/1` (what each file is, and the QC and blank checks of the series) and `eval` scores the evaluation cases. Used by `mzlab-cli llm`.
//! No network, no key, no dependency beyond `mzlab-core`.

pub mod agent;
pub mod assist;
pub mod chart;
mod chart_data;
pub mod chart_views;
pub mod compact;
pub mod context;
pub mod conv;
pub mod cost;
pub mod eval;
pub mod event;
pub mod fake;
pub mod inspect;
mod inspect_facts;
mod inspect_qc;
mod inspect_scan;
pub mod log;
mod moves;
pub mod params_rules;
mod params_rules_det;
mod params_rules_tab;
pub mod plan;
pub mod policy;
pub mod provider;
pub mod reading;
pub mod schema;
pub mod spend;
mod text;
pub mod tools;
