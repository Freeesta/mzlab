//! Tests of the wasm bridge of the assistant.

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
