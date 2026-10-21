//! `mzlab-cli llm tools [--out FILE]`: the catalog `strumenti/1` of the assistant tools, generated from the step registry `flow::steps()` and the
//! table of native executors. The catalog itself lives in `mzlab-llm`.

use std::path::PathBuf;

use mzlab_llm::tools::{catalog_json, UI_TOOLS};

use crate::flow::{executable, steps};
use crate::{emit, Outcome};

const HELP: &str = "mzlab-cli llm tools [--out FILE]   the catalog strumenti/1 of the assistant tools (stdout, or FILE)";

/// The catalog text, the one `docs/schemi/strumenti-1.json` holds.
pub fn tools_json() -> String {
    catalog_json(&steps(), UI_TOOLS, &executable)
}

pub(crate) fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    if it.next().as_deref() != Some("tools") {
        return Outcome::fail(&format!("usage: {HELP}"));
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
