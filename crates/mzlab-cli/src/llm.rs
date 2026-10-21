//! `mzlab-cli llm tools [--out FILE]`: the catalog `strumenti/1` of the assistant tools, generated from the step registry `flow::steps()` and the
//! table of native executors. `mzlab-cli llm replay REG.jsonl --conv CONV.json`: the events a recording `llmreg/1` gives for a conversation. The
//! catalog and the providers live in `mzlab-llm`.

use std::path::PathBuf;

use mzlab_llm::conv::{Conversation, Role};
use mzlab_llm::fake::Replay;
use mzlab_llm::provider::{builtin, request};
use mzlab_llm::tools::{catalog, catalog_json, UI_TOOLS};

use crate::flow::{executable, steps};
use crate::{emit, Outcome};

const HELP: &str = "mzlab-cli llm tools [--out FILE]   the catalog strumenti/1 of the assistant tools (stdout, or FILE)
mzlab-cli llm replay REG.jsonl --conv CONV.json [--turn N] [--strict]   the normalized events of a recording, one JSON line per event (stdout);
  the turn defaults to the number of assistant messages of the conversation; --strict also compares the SHA-256 of the request
mzlab-cli llm context RUNDIR [--budget N] [--hide-names]   the contesto/1 the assistant gets from a run folder (stdout)
mzlab-cli llm eval CASES_DIR --replay DIR   score the evaluation cases against their recordings DIR/<id>.jsonl (exit 1 if one fails)";

/// The catalog text, the one `docs/schemi/strumenti-1.json` holds.
pub fn tools_json() -> String {
    catalog_json(&steps(), UI_TOOLS, &executable)
}

pub(crate) fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    match it.next().as_deref() {
        Some("tools") => {}
        Some("replay") => return replay(it.collect()),
        Some("context") => return crate::llm_ctx::context(it.collect()),
        Some("eval") => return crate::llm_ctx::eval(it.collect()),
        _ => return Outcome::fail(&format!("usage: {HELP}")),
    }
    let mut out: Option<PathBuf> = None;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--out" => match it.next() {
                Some(p) => out = Some(p.into()),
                None => return Outcome::fail("--out needs a path"),
            },
            x => return Outcome::fail(&format!("unknown option {x}")),
        }
    }
    emit(tools_json(), &out)
}

/// The events of one turn of a recording; errors (a mismatch, a missing turn, a bad file) are the exit code 2 with the key on stderr.
fn replay(args: Vec<String>) -> Outcome {
    let (mut reg, mut conv, mut turn, mut strict) = (None, None, None, false);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--strict" => strict = true,
            "--conv" => conv = it.next(),
            "--turn" => match it.next().and_then(|t| t.parse::<usize>().ok()) {
                Some(t) => turn = Some(t),
                None => return Outcome::fail("--turn needs a number"),
            },
            x if !x.starts_with('-') && reg.is_none() => reg = Some(x.to_string()),
            x => return Outcome::fail(&format!("unknown option {x}")),
        }
    }
    let (Some(reg), Some(conv)) = (reg, conv) else {
        return Outcome::fail(&format!("usage: {HELP}"));
    };
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"));
    let go = || -> Result<String, String> {
        let c = Conversation::parse(&read(&conv)?).map_err(|e| e.to_string())?;
        let replay = Replay::parse(&read(&reg)?, strict).map_err(|e| e.to_string())?;
        let provider = builtin()
            .into_iter()
            .find(|p| p.id == c.provider)
            .ok_or(format!(
                "err.llm.conversation field=provider ({})",
                c.provider
            ))?;
        let tools = catalog(&steps(), UI_TOOLS, &executable);
        let req = request(&provider, &c, &tools).map_err(|e| e.to_string())?;
        let turn = turn.unwrap_or_else(|| {
            c.messages
                .iter()
                .filter(|m| m.role == Role::Assistant)
                .count()
        });
        let events = replay.at(turn, &req).map_err(|e| e.to_string())?;
        Ok(events.iter().map(|e| e.to_json() + "\n").collect())
    };
    go().map_or_else(|e| Outcome::fail(&e), |t| Outcome::ok(t.into_bytes()))
}
