//! The readers seen from outside: the request of a role, the verdict on its answer (accepted, asked again once with the problems, dropped), the phase
//! instruction as a `role: "system"` message in the middle of the conversation, and the readings that go with the next user message.

mod common;

use mzlab_core::json::{parse, Value};
use mzlab_llm::agent::{step, Input, Output, State};
use mzlab_llm::assist::{roles, with_prompts, Prompts, Role};
use mzlab_llm::conv::{Part, Role as Speaker};
use mzlab_llm::policy::{Level, Profile};
use mzlab_llm::provider::{builtin, Provider};
use mzlab_llm::subcheck::{check, request, Verdict, MAX_SUMMARY_TOKENS};

const ROLES: &str = include_str!("../data/assist/assist-ruoli-1.json");
const ROUTER: &str = include_str!("../data/assist/02_router.md");
const META: &str = include_str!("../data/assist/03_metadati.md");
const PHASE: &str = include_str!("../data/assist/06_piano.md");
const INPUT: &str =
    r#"{"ispezione/1":{"files":[{"alias":"f1","polarity":"positive","resolution":"unit"}]}}"#;
const GOOD: &str = r#"{"schema":"lettura/1","kind":"acquisition","findings":[{"id":"F1","topic":"polarity","statement":"f1 is positive","evidence":["ispezione/1.files.f1.polarity"],"confidence":"high","action":{"type":"none"}}],"questions":[],"numbers":[]}"#;

fn provider() -> Provider {
    builtin().into_iter().find(|p| p.id == "anthropic").unwrap()
}

fn role(name: &str) -> Role {
    roles(ROLES)
        .unwrap()
        .into_iter()
        .find(|r| r.role == name)
        .unwrap()
}

fn input() -> Value {
    parse(INPUT.as_bytes()).unwrap()
}

fn verdict(body: &str, attempt: u8) -> Verdict {
    let r = role("metadati");
    check(&provider(), &r, META, &input(), body, attempt, &[]).unwrap()
}

#[test]
fn the_request_is_the_role_of_the_data_with_the_input_as_data() {
    let r = request(&provider(), &role("metadati"), META, &input()).unwrap();
    let v = parse(r.body.as_bytes()).unwrap();
    assert_eq!(
        v.get("model").and_then(Value::str),
        Some("claude-sonnet-5-5")
    );
    assert!(v.get("tools").is_none());
    assert!(r.body.contains("\"format\"") && r.body.contains("lettura/1"));
    assert!(r.body.contains("ispezione/1") && !r.body.contains("temperature"));
    // the chart drawer has no reading schema: its answer is checked by `chart::make`
    let g = request(&provider(), &role("grafici"), META, &input()).unwrap();
    assert!(!g.body.contains("\"format\""));
}

#[test]
fn a_valid_reading_is_the_summary_the_orchestrator_sees() {
    let Verdict::Accepted(s) = verdict(GOOD, 0) else {
        panic!("not accepted")
    };
    assert!(
        s.starts_with("{\"schema\":\"lettura/1\"") && s.len().div_ceil(3) <= MAX_SUMMARY_TOKENS
    );
}

#[test]
fn an_invented_number_goes_back_once_with_the_problems_then_the_reading_is_dropped() {
    let bad = GOOD.replace("f1 is positive", "f1 is positive at 14.3 min");
    let Verdict::Retry { request, problems } = verdict(&bad, 0) else {
        panic!("no retry")
    };
    assert!(problems.contains("err.llm.reading.number"));
    // the retry carries the input, its own wrong answer and the problems, as one JSON object
    let v = parse(request.body.as_bytes()).unwrap();
    let text = format!("{:?}", v.get("messages"));
    assert!(
        text.contains("previous")
            && text.contains("err.llm.reading.number")
            && text.contains("ispezione/1")
    );
    let Verdict::Dropped { problems } = verdict(&bad, 1) else {
        panic!("not dropped")
    };
    assert!(problems.contains("err.llm.reading.number"));
}

#[test]
fn text_that_is_not_json_or_too_long_is_a_problem_not_a_summary() {
    assert!(matches!(verdict("I think so", 1), Verdict::Dropped { .. }));
    let long = format!(
        "{{\"schema\":\"lettura/1\",\"kind\":\"table\",\"findings\":[],\"pad\":\"{}\"}}",
        "x".repeat(4600)
    );
    let Verdict::Dropped { problems } = verdict(&long, 1) else {
        panic!("not dropped")
    };
    assert!(problems.contains("err.llm.reading.size"));
}

fn machine(phase: bool) -> State {
    let mut s = State::new(
        provider(),
        "claude-sonnet-5-5",
        common::tools(),
        Vec::new(),
        Level::L1,
        Profile::Research,
    );
    let book: Prompts = with_prompts(&[
        ("02_router.md", ROUTER),
        ("03_metadati.md", META),
        ("06_piano.md", PHASE),
    ]);
    let all: Vec<Role> = roles(ROLES)
        .unwrap()
        .into_iter()
        .filter(|r| r.role == "router")
        .collect();
    s.enable_assist(all, book).unwrap();
    if phase {
        s.set_phase_prompt("06_piano.md").unwrap();
    }
    s
}

fn routed(s: &mut State, route: &str) -> Vec<Output> {
    step(s, Input::User("go".into()));
    let body = format!(
        "{{\"route\":\"{route}\",\"need_data\":[],\"language\":\"en\",\"suspicious\":false}}"
    );
    step(
        s,
        Input::SubResult {
            role: "router".into(),
            body: Ok(body),
        },
    )
}

fn phase_messages(s: &State) -> usize {
    s.conversation()
        .messages
        .iter()
        .filter(|m| m.role == Speaker::System)
        .count()
}

fn end_turn(s: &mut State) {
    step(
        s,
        Input::Event(mzlab_llm::event::Event::Stop(
            mzlab_llm::event::StopReason::End,
        )),
    );
}

#[test]
fn the_phase_instruction_is_a_system_message_once_per_entry_into_the_plan() {
    let mut s = machine(true);
    routed(&mut s, "table");
    assert_eq!(phase_messages(&s), 0);
    end_turn(&mut s);
    let out = routed(&mut s, "plan");
    let Some(Output::Send(r)) = out.iter().find(|o| matches!(o, Output::Send(_))) else {
        panic!("no send")
    };
    assert_eq!(phase_messages(&s), 1);
    let m = s.conversation().messages.last().unwrap();
    assert!(m.role == Speaker::System && matches!(&m.parts[0], Part::Text(t) if t == PHASE));
    assert!(r.body.contains("\"role\":\"system\"") && r.body.contains("Phase: workflow"));
    assert!(
        s.log().contains("\"ev\":\"phase\"")
            && s.log()
                .contains(&mzlab_core::budget::sha256_hex(PHASE.trim_end().as_bytes()))
    );
    end_turn(&mut s);
    routed(&mut s, "plan");
    assert_eq!(phase_messages(&s), 1, "the same phase is not repeated");
    end_turn(&mut s);
    routed(&mut s, "chart");
    end_turn(&mut s);
    routed(&mut s, "plan");
    assert_eq!(phase_messages(&s), 2, "a new entry gives it again");
}

#[test]
fn without_a_phase_prompt_or_a_valid_route_no_message_is_added() {
    let mut s = machine(false);
    routed(&mut s, "plan");
    assert_eq!(phase_messages(&s), 0);
    assert!(machine(false).set_phase_prompt("missing.md").is_err());
}

#[test]
fn an_accepted_reading_rides_with_the_next_user_message_as_data() {
    let mut s = machine(false);
    s.add_reading("metadati", parse(GOOD.as_bytes()).unwrap());
    let out = step(&mut s, Input::User("what are my files?".into()));
    assert!(out.iter().any(|o| matches!(o, Output::Sub { .. })));
    let m = s.conversation().messages.last().unwrap();
    assert!(m.role == Speaker::User && m.parts.len() == 2);
    assert!(
        matches!(&m.parts[0], Part::Context(c) if c.get("reading").and_then(Value::str) == Some("metadati"))
    );
    // delivered once
    end_turn(&mut s);
    step(
        &mut s,
        Input::SubResult {
            role: "router".into(),
            body: Err("err.llm.network".into()),
        },
    );
    step(&mut s, Input::User("and now?".into()));
    assert_eq!(s.conversation().messages.last().unwrap().parts.len(), 1);
}
