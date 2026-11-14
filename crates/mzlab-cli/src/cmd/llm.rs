//! The `llm` family: the catalogs and recordings of the assistant.

use super::Entry;

pub(crate) static TABLE: &[Entry] = &[
    Entry { name: "llm", aliases: &[], help: "mzlab-cli llm tools [--out FILE]   the catalog strumenti/1 of the assistant tools (llm tools --help)\n       mzlab-cli llm replay REG.jsonl --conv CONV.json [--turn N] [--strict]   the events a recording gives for a conversation, one per line\n       mzlab-cli llm context RUNDIR [--budget N] [--hide-names]   the contesto/1 of a run folder; llm eval CASES_DIR --replay DIR scores the cases", run: crate::llm::run },
];
