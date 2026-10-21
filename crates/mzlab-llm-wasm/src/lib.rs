//! wasm-bindgen bridge of `mzlab-llm` for the assistant of the site: the catalog in, a conversation in, the request out; the response bytes in, normalized
//! events out (one JSON line each); a recording `llmreg/1` for the transport `replay`. No network and no key here: the iframe `llm/trasporto.html` does the fetch.
//! An error is a JSON text `{"error", "error_key", "params"}` (the same shape as the routes of the engine). Loaded by `mzlab/web/llm/llm-worker.js`.
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

fn fail(e: Error) -> String {
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

fn find(id: &str) -> Result<Provider, String> {
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
mod tests {
    use super::*;

    const CONV: &str = r#"{"schema":"llm-conv/1","provider":"anthropic","model":"claude-sonnet-5-5","system":"s","messages":[{"role":"user","parts":[{"text":"hi"}]}]}"#;

    fn tools() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/schemi/strumenti-1.json"
        ))
        .unwrap()
    }

    #[test]
    fn request_has_no_key_and_names_the_slot() {
        let l = Llm::new(&tools()).unwrap();
        let r = l.request(CONV).unwrap();
        assert!(r.contains("\"url\":\"https://api.anthropic.com"));
        assert!(r.contains("\"auth\":{\"header\":\"x-api-key\",\"prefix\":\"\"}"));
        assert!(!r.contains("sk-"));
    }

    #[test]
    fn unknown_provider_is_a_json_error() {
        let l = Llm::new(&tools()).unwrap();
        let e = l.request(&CONV.replace("anthropic", "nobody")).unwrap_err();
        assert!(e.contains("\"error_key\":\"err.llm.conversation\""), "{e}");
    }

    #[test]
    fn replay_gives_the_recorded_turn_and_strict_checks_the_request() {
        let mut l = Llm::new(&tools()).unwrap();
        let events = [
            mzlab_llm::event::Event::TextDelta("ok".into()),
            mzlab_llm::event::Event::Stop(mzlab_llm::event::StopReason::End),
        ];
        let reg = mzlab_llm::fake::record_line(0, &l.build(CONV).unwrap(), &events);
        assert_eq!(l.replay_load(&reg, true).unwrap(), 1);
        let ev = l.replay_respond(CONV).unwrap();
        assert!(ev.contains("text_delta") && ev.contains("\"ok\""), "{ev}");
        l.replay_reset();
        let other = CONV.replace("hi", "bye");
        assert!(l
            .replay_respond(&other)
            .unwrap_err()
            .contains("err.llm.replay.mismatch"));
    }

    #[test]
    fn stream_survives_any_split() {
        let sse = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        let whole = {
            let mut s = LlmStream::new("anthropic", "claude-sonnet-5-5").unwrap();
            s.push(sse.as_bytes()) + &s.finish()
        };
        let split = {
            let mut s = LlmStream::new("anthropic", "claude-sonnet-5-5").unwrap();
            let mut t = String::new();
            for c in sse.as_bytes().chunks(7) {
                t += &s.push(c);
            }
            t + &s.finish()
        };
        assert!(
            whole.contains("hello") && whole.contains("\"stop\""),
            "{whole}"
        );
        assert_eq!(whole, split);
    }
}
