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
    Entry::new("db normative build", "mzlab-cli db normative build --cache DIR --out DIR   the EU and US regulatory lists from official sources (native only)", |a| normative(pre("build", a))).schema("normative-build/1").exits(&[0, 1, 2, 3, 4, 6]),
    Entry::new("db normative pack", "mzlab-cli db normative pack ...   pack the regulatory lists (native only)", |a| normative(pre("pack", a))).exits(&[0, 2, 4, 6]),
    Entry::new("db normative listings", "mzlab-cli db normative listings ...   the regulatory lists built (native only)", |a| normative(pre("listings", a))).exits(&[0, 2, 4]),
    Entry::new("db normative import-official", "mzlab-cli db normative import-official ...   import the user's official lists (native only)", |a| normative(pre("import-official", a))).exits(&[0, 2, 3, 4, 6]),
    Entry::new("db insilico pack", "mzlab-cli db insilico pack CSV OUT   the spectra.insilico candidate package (native only)", insilico).schema("insilico/1").exits(&[0, 1, 2, 4, 6]),
    Entry::new("db ref-list", "mzlab-cli db ref-list FILE.csv [--name N] [--match X]   a CSV of reference ions as JSON (reflist/1); exit 1 on row errors", crate::plain::ref_list_cmd::run).schema("reflist/1"),
    Entry::new("llm tools", "mzlab-cli llm tools [--out FILE] [--demo] [--compact]   the assistant tools (tools/1)", |a| crate::llm::run(pre("tools", a))).examples(&["mzlab-cli llm tools --compact"]).schema("tools/1"),
    Entry::new("llm inspect", "mzlab-cli llm inspect FILE|DIR... [--budget N] [--hide-names]   inspect mzML files", |a| crate::llm::run(pre("inspect", a))).schema("inspection/1").exits(&[0, 2, 4]),
    Entry::new("llm params", "mzlab-cli llm params [FILE|DIR...] [--inspection FILE|-] [--answers FILE] [--budget N] [--hide-names]   propose parameters by rules", |a| crate::llm::run(pre("params", a))).schema("param-proposal/1").exits(&[0, 2, 4]),
    Entry::new("llm context", "mzlab-cli llm context RUNDIR [--budget N] [--hide-names]   a run folder's context", |a| crate::llm::run(pre("context", a))).schema("context/1").exits(&[0, 2, 4]),
    Entry::new("llm replay", "mzlab-cli llm replay REG.jsonl --conv CONV.json [--turn N] [--strict]   replay a recording", |a| crate::llm::run(pre("replay", a))).schema("replay/1").exits(&[0, 1, 2, 3, 4]),
    Entry::new("llm eval", "mzlab-cli llm eval CASES_DIR --replay DIR   score the cases", |a| crate::llm::run(pre("eval", a))).schema("evalrun/1").exits(&[0, 1, 2, 3, 4]),
];
