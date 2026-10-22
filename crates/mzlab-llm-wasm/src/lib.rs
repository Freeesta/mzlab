//! wasm-bindgen bridge of `mzlab-llm` for the assistant of the site: the catalog in, a conversation in, the request out; the response bytes in, normalized
//! events out (one JSON line each); a recording `llmreg/1` for the transport `replay`. No network and no key here: the iframe `llm/trasporto.html` does the fetch.
//! An error is a JSON text `{"error", "error_key", "params"}` (the same shape as the routes of the engine). Loaded by `mzlab/web/llm/llm-worker.js`.
mod agent;

use mzlab_core::error::Error;
use mzlab_llm::conv::Conversation;
use mzlab_llm::fake::Replay;
use mzlab_llm::provider::{self, Provider, Request, Stream, ToolMode};
use mzlab_llm::tools::{parse_catalog, Tool};
use wasm_bindgen::prelude::*;

/// `mzlab-llm/1`: the page checks that the loaded `.wasm` is the one it was built with.
#[wasm_bindgen]
pub fn version() -> String {
    "mzlab-llm/1".to_string()
}

/// `fornitori-1.json` as shipped (`llm-providers/1`): the providers and models the settings list.
#[wasm_bindgen]
pub fn providers() -> String {
    provider::BUILTIN_JSON.to_string()
}

/// The event line of a failed HTTP status (`err.llm.*`), for the transport that got an answer without a stream.
#[wasm_bindgen]
pub fn http_error(status: u16, body: &str) -> String {
    provider::http_error(status, body).to_json()
}

/// The `contesto/1` text for sources collected by the page (`mzlab_llm::context::build_text`), within `budget_tokens`.
#[wasm_bindgen]
pub fn context_build(sources_json: &str, budget_tokens: u32) -> Result<String, String> {
    mzlab_llm::context::build_text(sources_json, u64::from(budget_tokens)).map_err(fail)
}

/// The `ispezione/1` text (what each file is, QC and blank checks) for the sources the page collected (`mzlab_llm::inspect::build_text`), within `budget_tokens`.
#[wasm_bindgen]
pub fn inspect_build(sources_json: &str, budget_tokens: u32) -> Result<String, String> {
    mzlab_llm::inspect::build_text(sources_json, u64::from(budget_tokens)).map_err(fail)
}

pub(crate) fn fail(e: Error) -> String {
    let mut o = mzlab_core::json::Out::default();
    o.raw("{\"error\":")
        .str(e.key)
        .raw(",\"error_key\":")
        .str(e.key)
        .raw(",\"params\":{");
    for (i, (k, v)) in e.params.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(k).raw(":").str(v);
    }
    o.raw("}}");
    o.0
}

pub(crate) fn find(id: &str) -> Result<Provider, String> {
    provider::builtin()
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| fail(Error::new("err.llm.conversation").with("field", "provider")))
}

fn lines(events: &[mzlab_llm::event::Event]) -> String {
    events.iter().map(|e| e.to_json() + "\n").collect()
}

/// The state of the assistant: the tool catalog the page handed over and the recording of the transport `replay`, if one is loaded.
#[wasm_bindgen]
pub struct Llm {
    tools: Vec<Tool>,
    replay: Option<Replay>,
}

#[wasm_bindgen]
impl Llm {
    /// `tools_json`: the catalog `strumenti/1` (`docs/schemi/strumenti-1.json`, shipped beside the page). Call it once per page.
    #[wasm_bindgen(constructor)]
    pub fn new(tools_json: &str) -> Result<Llm, String> {
        Ok(Llm {
            tools: parse_catalog(tools_json).map_err(fail)?,
            replay: None,
        })
    }

    fn build(&self, conv_json: &str) -> Result<Request, String> {
        let conv = Conversation::parse(conv_json).map_err(fail)?;
        let p = find(&conv.provider)?;
        provider::request(&p, &conv, &self.tools).map_err(fail)
    }

    /// The request of the next turn (`{url, headers, body, auth}`, never the key) for a conversation `llm-conv/1`; also the text of «What will be sent».
    pub fn request(&self, conv_json: &str) -> Result<String, String> {
        Ok(self.build(conv_json)?.to_json())
    }

    /// Loads a recording `llmreg/1`; `strict` also compares the SHA-256 of each request.
    pub fn replay_load(&mut self, text: &str, strict: bool) -> Result<u32, String> {
        let r = Replay::parse(text, strict).map_err(fail)?;
        let n = r.len() as u32;
        self.replay = Some(r);
        Ok(n)
    }

    /// The events (one JSON line each) the recording gives for the next turn of the conversation.
    pub fn replay_respond(&mut self, conv_json: &str) -> Result<String, String> {
        let req = self.build(conv_json)?;
        let r = self
            .replay
            .as_mut()
            .ok_or_else(|| fail(Error::new("err.llm.replay.format")))?;
        Ok(lines(&r.respond(&req).map_err(fail)?))
    }

    pub fn replay_reset(&mut self) {
        if let Some(r) = self.replay.as_mut() {
            r.reset();
        }
    }
}

/// The decoder of one answer: the bytes of the response in (any split), event lines out.
#[wasm_bindgen]
pub struct LlmStream {
    inner: Stream,
}

#[wasm_bindgen]
impl LlmStream {
    /// For `provider` and `model` (a `prompted` model reads its tool call from the text).
    #[wasm_bindgen(constructor)]
    pub fn new(provider_id: &str, model: &str) -> Result<LlmStream, String> {
        let p = find(provider_id)?;
        let inner = Stream::new(p.adapter);
        let prompted = p
            .model(model)
            .is_some_and(|m| m.tools == ToolMode::Prompted);
        Ok(LlmStream {
            inner: if prompted { inner.prompted() } else { inner },
        })
    }

    pub fn push(&mut self, bytes: &[u8]) -> String {
        lines(&self.inner.push(bytes))
    }

    pub fn finish(&mut self) -> String {
        lines(&self.inner.finish())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
