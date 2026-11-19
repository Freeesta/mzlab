//! The `db` group (reference databases and candidate packs) and the `llm` group (the catalogs and recordings of the assistant).

use super::{pre, Entry};
use crate::Outcome;

// In the wasm build these commands exist in the help but run nowhere.
#[cfg(not(target_family = "wasm"))]
fn normative(a: Vec<String>) -> Outcome {
    crate::normative::run(a)
}
#[cfg(not(target_family = "wasm"))]
fn insilico(a: Vec<String>) -> Outcome {
    crate::insilico_pack::run(pre("pack", a))
}
#[cfg(target_family = "wasm")]
fn normative(_: Vec<String>) -> Outcome {
    Outcome::fail("unknown command db normative")
}
#[cfg(target_family = "wasm")]
fn insilico(_: Vec<String>) -> Outcome {
    Outcome::fail("unknown command db insilico")
}

pub(crate) static TABLE: &[Entry] = &[
    Entry::new("db normative build", "mzlab-cli db normative build --cache DIR --out DIR   build the EU and US regulatory lists from the official sources (native only)", |a| normative(pre("build", a))),
    Entry::new("db normative pack", "mzlab-cli db normative pack ...   pack the regulatory lists (native only)", |a| normative(pre("pack", a))),
    Entry::new("db normative listings", "mzlab-cli db normative listings ...   the regulatory lists built (native only)", |a| normative(pre("listings", a))),
    Entry::new("db normative import-official", "mzlab-cli db normative import-official ...   import the official lists the user owns (native only)", |a| normative(pre("import-official", a))),
    Entry::new("db insilico pack", "mzlab-cli db insilico pack CSV OUT   the spectra.insilico candidate package (native only)", insilico),
    Entry::new("db ref-list", "mzlab-cli db ref-list FILE.csv [--name N] [--match X]   read a CSV list of reference ions as JSON (reflist/1) with line-numbered row errors; exit 1 if rows are wrong", crate::plain::ref_list_cmd::run),
    Entry::new("llm tools", "mzlab-cli llm tools [--out FILE] [--demo] [--compact]   the catalog tools/1 of the assistant tools", |a| crate::llm::run(pre("tools", a))).examples(&["mzlab-cli llm tools --compact"]),
    Entry::new("llm inspect", "mzlab-cli llm inspect FILE...   the facts of a file the assistant reads", |a| crate::llm::run(pre("inspect", a))).schema("inspection/1"),
    Entry::new("llm params", "mzlab-cli llm params ...   the parameters the assistant proposes", |a| crate::llm::run(pre("params", a))).schema("param-proposal/1"),
    Entry::new("llm context", "mzlab-cli llm context RUNDIR [--budget N] [--hide-names]   the context/1 of a run folder", |a| crate::llm::run(pre("context", a))).schema("context/1"),
    Entry::new("llm replay", "mzlab-cli llm replay REG.jsonl --conv CONV.json [--turn N] [--strict]   the events a recording gives for a conversation, one per line", |a| crate::llm::run(pre("replay", a))),
    Entry::new("llm eval", "mzlab-cli llm eval CASES_DIR --replay DIR   score the cases", |a| crate::llm::run(pre("eval", a))),
];
