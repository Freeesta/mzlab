//! Tests of the wasm bridge of the assistant.

use super::*;
use crate::agent::Agent;

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

fn recording(path: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../tests/data/llm/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// The `id` of the first `call` line.
fn call_id(lines: &str) -> String {
    let l = lines
        .lines()
        .find(|l| l.contains("\"o\":\"call\""))
        .expect("a call");
    let v = mzlab_core::json::parse(l.as_bytes()).unwrap();
    v.get("id").and_then(|x| x.str()).unwrap().to_string()
}

#[test]
fn agent_runs_a_recorded_turn_and_the_page_answers_the_plan() {
    let mut a = Agent::new(&tools(), "anthropic", "claude-sonnet-5-5", "L0", true).unwrap();
    a.replay_load(&recording("E1.jsonl"), false).unwrap();
    let out = a.user("Find library matches.", "").unwrap();
    // the plan is not checked here: it goes to the page, which answers
    assert!(
        out.contains("\"o\":\"send\"")
            && out.contains("\"o\":\"call\"")
            && out.contains("flow_propose_plan"),
        "{out}"
    );
    let id = call_id(&out);
    let next = a.result(&id, true, "{\"applied\":true}").unwrap();
    // L0 denies the next call (an edit): the model sees the refusal and the recording ends the turn
    assert!(
        next.contains("\"o\":\"done\",\"outcome\":\"end\""),
        "{next}"
    );
    assert_eq!(a.outcome(), "end");
    assert!(a.conversation().contains("\"tool_result\""));
    let conv = a.conversation();
    assert_eq!(
        mzlab_llm::conv::Conversation::parse(&conv)
            .unwrap()
            .to_json(),
        conv
    );
    assert!(a.log().contains("\"ev\":\"decision\""), "{}", a.log());
    assert!(a.usage()[0] > 0.0);
}

#[test]
fn agent_counts_plans_the_page_refuses_and_stops_at_three() {
    let l = Llm::new(&tools()).unwrap();
    let req = l.build(CONV).unwrap();
    let events = |n: u32| {
        use mzlab_llm::event::{Event, StopReason};
        vec![
            Event::ToolCallStart {
                id: format!("c{n}"),
                name: "flow_propose_plan".into(),
            },
            Event::ToolCallDelta("{\"plan\":{\"format\":\"mzflow/2\"}}".into()),
            Event::ToolCallEnd {
                id: format!("c{n}"),
            },
            Event::Stop(StopReason::Tool),
        ]
    };
    let reg: String = (0..4)
        .map(|n| mzlab_llm::fake::record_line(n, &req, &events(n as u32)) + "\n")
        .collect();
    let mut a = Agent::new(&tools(), "anthropic", "claude-sonnet-5-5", "L0", true).unwrap();
    a.replay_load(&reg, false).unwrap();
    let mut out = a.user("plan", "").unwrap();
    for _ in 0..3 {
        if out.contains("\"o\":\"done\"") {
            break;
        }
        out = a
            .result(
                &call_id(&out),
                false,
                "{\"problems\":[{\"error_key\":\"err.flow.port.missing\"}]}",
            )
            .unwrap();
    }
    assert_eq!(a.outcome(), "error:err.llm.plan.tries", "{out}");
}

#[test]
fn agent_in_the_course_profile_does_nothing() {
    let mut a = Agent::new(&tools(), "anthropic", "claude-sonnet-5-5", "L2", false).unwrap();
    let out = a.user("hello", "").unwrap();
    assert_eq!(
        (out.as_str(), a.outcome().as_str()),
        ("", "error:err.llm.profile.course")
    );
}

#[test]
fn context_is_built_from_the_sources_of_the_page() {
    let c = context_build(r#"{"profile":"ricerca","plan":{"sha256":"x","instances":[{"id":"a","step":"features.detect@1"}],"problems":[]}}"#, 2000).unwrap();
    assert!(
        c.starts_with("{\"schema\":\"contesto/1\"") && c.contains("features.detect@1"),
        "{c}"
    );
    assert!(context_build(r#"{"profile":"corso"}"#, 2000)
        .unwrap_err()
        .contains("err.llm.profile.course"));
}
