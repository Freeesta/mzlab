//! Tests of the wasm bridge of the assistant.

use super::*;
use crate::agent::Agent;
use crate::assist::Assist;

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

fn doc(path: &str) -> String {
    std::fs::read_to_string(format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// The roles and the prompts as the page hands them over.
fn assist_files() -> (String, String) {
    let dir = "crates/mzlab-llm/data/assist";
    let roles = doc(&format!("{dir}/assist-ruoli-1.json"));
    let pairs: Vec<String> =
        std::fs::read_dir(format!("{}/../../{dir}", env!("CARGO_MANIFEST_DIR")))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.ends_with(".md"))
            .map(|n| {
                let mut o = mzlab_core::json::Out::default();
                o.raw("[")
                    .str(&n)
                    .raw(",")
                    .str(&doc(&format!("{dir}/{n}")))
                    .raw("]");
                o.0
            })
            .collect();
    (roles, format!("[{}]", pairs.join(",")))
}

fn assist() -> Assist {
    let (roles, prompts) = assist_files();
    Assist::new(
        "anthropic",
        &roles,
        &prompts,
        &doc("docs/schemi/steps-1.json"),
    )
    .unwrap()
}

const INPUT: &str = r#"{"ispezione/1":{"files":[{"alias":"f1","polarity":"positive"}]}}"#;
const GOOD: &str = r#"{"schema":"lettura/1","kind":"acquisition","findings":[{"id":"F1","topic":"polarity","statement":"f1 is positive","evidence":["ispezione.files.f1.polarity"],"confidence":"high","action":{"type":"none"}}]}"#;

#[test]
fn assist_asks_a_role_and_gives_the_orchestrator_only_what_passed() {
    let a = assist();
    let r = a.request("metadati", INPUT).unwrap();
    assert!(r.contains("claude-sonnet-5-5") && r.contains("output_config") && !r.contains("sk-"));
    assert!(a.digest("metadati").unwrap().len() == 64);
    assert!(a
        .request("nobody", INPUT)
        .unwrap_err()
        .contains("err.llm.assist.call"));
    let ok = a.check("metadati", INPUT, GOOD, 0).unwrap();
    assert!(
        ok.starts_with("{\"verdict\":\"accepted\",\"summary\":{\"schema\":\"lettura/1\""),
        "{ok}"
    );
    let bad = GOOD.replace("f1 is positive", "f1 is positive at 14.3 min");
    let retry = a.check("metadati", INPUT, &bad, 0).unwrap();
    assert!(
        retry.starts_with("{\"verdict\":\"retry\",\"request\":{\"url\"")
            && retry.contains("err.llm.reading.number"),
        "{retry}"
    );
    let dropped = a.check("metadati", INPUT, &bad, 1).unwrap();
    assert!(dropped.starts_with("{\"verdict\":\"dropped\""), "{dropped}");
}

#[test]
fn assist_refuses_roles_without_their_prompt() {
    let (roles, _) = assist_files();
    let e = Assist::new("anthropic", &roles, "[]", "[]").err().unwrap();
    assert!(e.contains("err.llm.assist.roles"), "{e}");
}

#[test]
fn the_agent_asks_the_router_then_adds_the_phase_message_and_the_readings() {
    let (roles, prompts) = assist_files();
    let mut a = Agent::new(&tools(), "anthropic", "claude-sonnet-5-5", "L1", true).unwrap();
    a.enable_assist(&roles, &prompts, "06_piano.md").unwrap();
    a.set_assist_state(r#"{"phase":"table","files_open":7,"plan":false,"pending_questions":[]}"#)
        .unwrap();
    a.reading("metadati", GOOD).unwrap();
    let out = a.user("build the workflow", "").unwrap();
    assert!(out.contains("\"o\":\"sub\",\"role\":\"router\""), "{out}");
    let route = r#"{"route":"plan","need_data":["plan"],"language":"en","suspicious":false}"#;
    let out = a.sub_result("router", true, route);
    assert!(out.contains("\"o\":\"send\""), "{out}");
    let conv = a.conversation();
    assert!(
        conv.contains("\"role\":\"system\"") && conv.contains("Phase: workflow"),
        "{conv}"
    );
    assert!(conv.contains("\"reading\":\"metadati\""));
    assert!(a.enable_assist(&roles, &prompts, "missing.md").is_err());
}

#[test]
fn the_readers_follow_the_route_once_each_and_a_bad_answer_asks_for_none() {
    let r = r#"{"route":"table","need_data":["tabcheck","rows","inspection","tabcheck","plan"],"language":"en","suspicious":false}"#;
    assert_eq!(Assist::readers(r), r#"["interprete","metadati"]"#);
    assert_eq!(Assist::readers(r#"{"route":"nope"}"#), "[]");
    assert_eq!(Assist::readers("not json"), "[]");
}

#[test]
fn a_request_of_a_subagent_survives_the_trip_through_json() {
    let (roles, prompts) = assist_files();
    let a = Assist::new("anthropic", &roles, &prompts, "[]").unwrap();
    let req = a.request("metadati", INPUT).unwrap();
    let back =
        mzlab_llm::provider::Request::from_value(&mzlab_core::json::parse(req.as_bytes()).unwrap())
            .unwrap();
    assert_eq!(back.to_json(), req);
}
