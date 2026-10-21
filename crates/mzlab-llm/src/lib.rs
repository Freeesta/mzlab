//! The LLM side of mzLab, kept apart from the engine: `tools` builds the catalog `strumenti/1` from the step registry. Used by `mzlab-cli llm`.
//! No network, no key, no dependency beyond `mzlab-core`.

pub mod tools;
