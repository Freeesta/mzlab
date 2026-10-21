//! The logic of the tutor, from the question to the checked answer: `ask` (the request `tutor-ask/1`), `pick` (choice of the blocks), `answer` (the model
//! request and the passages mode), `checks` (citations, numbers, hidden answers), `quota` (caps in integer micro-dollars), `stat` (counters `tutorstat/1`) and
//! `eval` (the scoring machine and the pipeline the Worker will run), on top of the blocks `blocchi` and their search `cerca`. Used by the `teoria-blocchi`
//! binary at build time, by `mzlab-cli tutor` and by the Worker; pure and deterministic (no clock, no I/O, no network: the transport is passed in).
pub mod answer;
pub mod ask;
pub mod blocchi;
pub mod cerca;
pub mod checks;
pub mod eval;
mod html;
mod pagina;
pub mod pick;
pub mod quota;
pub mod stat;

use mzlab_core::error::Error;
use mzlab_llm::provider::{Adapter, Provider, Request};

/// Schema name written in the blocks file.
pub const SCHEMA: &str = "teoria-blocchi/1";

/// The words of a text for search: normalised like `cerca.js` (lower case, no accents), split on anything but letters, digits and `+/.`,
/// with the dots, slashes and plus signs at the edges of a word dropped ("m/z" and "m+h" stay whole, "risoluzione." becomes "risoluzione").
pub fn words(text: &str) -> Vec<String> {
    mzlab_libro::norm(text)
        .split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '.')))
        .map(|w| w.trim_matches(|c| matches!(c, '+' | '/' | '.')))
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// `err.tutor.request` for a malformed field.
pub(crate) fn bad(field: &str) -> Error {
    Error::new("err.tutor.request").with("field", field)
}

/// Reasoning efforts the Messages API accepts in `output_config.effort`.
const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

/// Adds `output_config.effort` (and adaptive `thinking`) to the body the Anthropic adapter wrote: the adapter has no field for them, the tutor sets them here.
pub(crate) fn with_options(
    mut r: Request,
    p: &Provider,
    thinking: bool,
    effort: &str,
) -> Result<Request, Error> {
    if p.adapter != Adapter::Anthropic {
        return Err(Error::new("err.tutor.provider").with("field", "adapter"));
    }
    if !EFFORTS.contains(&effort) {
        return Err(bad("effort"));
    }
    if r.body.pop() != Some('}') {
        return Err(Error::new("err.tutor.provider").with("field", "body"));
    }
    if thinking {
        r.body.push_str(",\"thinking\":{\"type\":\"adaptive\"}");
    }
    r.body
        .push_str(&format!(",\"output_config\":{{\"effort\":\"{effort}\"}}}}"));
    Ok(r)
}

/// Text that goes into a delimited data block: the angle brackets and the ampersand are escaped so a block can never close its own tag.
pub(crate) fn esc(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
