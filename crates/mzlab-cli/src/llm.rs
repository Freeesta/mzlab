//! `mzlab-cli llm tools [--out FILE]`: the catalog `tools/1` of the assistant tools, generated from the step registry `flow::steps()` and the
//! table of native executors. `mzlab-cli llm replay REG.jsonl --conv CONV.json`: the events a recording `llmreg/1` gives for a conversation. The
//! catalog and the providers live in `mzlab-llm`. `llm inspect|params DIR`: the facts without a model (`llm_facts.rs`).

use std::path::PathBuf;

use mzlab_llm::conv::{Conversation, Role};
use mzlab_llm::fake::Replay;
use mzlab_llm::provider::{builtin, request};
use mzlab_llm::tools::{catalog, catalog_json, demo_catalog_json, UI_TOOLS};

use crate::flow::{executable, steps};
use crate::{emit, Outcome};

// `llm inspect` and `llm params` (the crate root is at its line limit)
#[path = "llm_facts.rs"]
pub(crate) mod facts;

const HELP: &str = "mzlab-cli llm tools [--demo] [--compact] [--out FILE]   the catalog tools/1 of the assistant tools (stdout, or FILE); --demo: the subset of the public demo;
  --compact: tools-compact/1, per tool the name, class and the names of its inputs, no prose
mzlab-cli llm replay REG.jsonl --conv CONV.json [--turn N] [--strict]   the normalized events of a recording, one JSON line per event (stdout);
  the turn defaults to the number of assistant messages of the conversation; --strict also compares the SHA-256 of the request
mzlab-cli llm context RUNDIR [--budget N] [--hide-names]   the context/1 the assistant gets from a run folder (stdout)
mzlab-cli llm eval CASES_DIR --replay DIR   score the evaluation cases against their recordings DIR/<id>.jsonl (exit 1 if one fails)
mzlab-cli llm inspect DIR [--budget N] [--hide-names]   the inspection/1 of the mzML files of a folder, without a model (stdout)
mzlab-cli llm params DIR [--answers FILE] [--budget N] [--hide-names]   the param-proposal/1 of the rules for the same files (stdout)";

/// The catalog text, the one `docs/schemi/tools-1.json` holds.
pub fn tools_json() -> String {
    catalog_json(&steps(), UI_TOOLS, &executable)
}

/// The demo catalog text, the one `docs/schemi/tools-demo-1.json` holds.
pub fn demo_tools_json() -> String {
    demo_catalog_json(&steps(), UI_TOOLS, &executable)
}

pub(crate) fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    match it.next().as_deref() {
        Some("tools") => {}
        Some("replay") => return replay(it.collect()),
        Some("context") => return crate::llm_ctx::context(it.collect()),
        Some("eval") => return crate::llm_ctx::eval(it.collect()),
        Some("inspect") => return facts::inspect(it.collect()),
        Some("params") => return facts::params(it.collect()),
        _ => return Outcome::fail(&format!("usage: {HELP}")),
    }
    let mut out: Option<PathBuf> = None;
    let mut demo = false;
    let mut compact = false;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--demo" => demo = true,
            "--compact" => compact = true,
            "--out" => match it.next() {
                Some(p) => out = Some(p.into()),
                None => return Outcome::fail("--out needs a path"),
            },
            x => return Outcome::fail(&format!("unknown option {x}")),
        }
    }
    if compact {
        return emit(compact_json(demo), &out);
    }
    // the citation is a CLI-only key: the files of docs/schemi and the catalog of the site stay as they are
    let text = if demo {
        demo_tools_json()
    } else {
        tools_json()
    };
    // `cli_catalog` is CLI-only too: the catalog of the commands beside the catalog of the tools, so the agent catalogs agree
    let text = crate::citation::after(&text, "\"schema\":\"tools/1\",", "\n  ");
    let text = text.replacen(
        "\"schema\":\"tools/1\",",
        "\"schema\":\"tools/1\",\n  \"cli_catalog\":\"mzlab-cli commands --json\",",
        1,
    );
    emit(text, &out)
}

/// `tools-compact/1`: the tools of the catalog by name and class with the names of their inputs; the full text stays one call away.
fn compact_json(demo: bool) -> String {
    use mzlab_core::json::{parse, Value};
    use mzlab_core::jsonb::{text, write, Obj};
    let src = if demo {
        demo_tools_json()
    } else {
        tools_json()
    };
    let doc = parse(src.as_bytes()).unwrap_or(Value::Null);
    let tools = doc
        .get("tools")
        .map(|t| t.arr().to_vec())
        .unwrap_or_default();
    let items = tools
        .iter()
        .map(|t| {
            let s = |k: &str| text(t.get(k).and_then(Value::str).unwrap_or(""));
            let inputs = match t.get("input_schema").and_then(|i| i.get("properties")) {
                Some(Value::Obj(o)) => o.iter().map(|(k, _)| text(k)).collect(),
                _ => Vec::new(),
            };
            Obj::new()
                .put("name", s("name"))
                .put("class", s("class"))
                .put("inputs", Value::Arr(inputs))
                .done()
        })
        .collect();
    let doc = Obj::new()
        .put("schema", text("tools-compact/1"))
        .put("citation", text(&crate::citation::text()))
        .put("catalog", text("mzlab-cli llm tools"))
        .put("tools", Value::Arr(items))
        .done();
    write(&doc) + "\n"
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
