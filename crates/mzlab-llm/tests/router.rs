//! The router and the subagent calls of the machine: the request of the router (a golden file), its closed answer schema, a turn that waits for the router
//! and goes on without a route when the answer is wrong, and a stop that makes a late answer harmless.

mod common;

use mzlab_core::json::{parse, Value};
use mzlab_llm::agent::{step, Input, Output, State};
use mzlab_llm::assist::{roles, with_prompts};
use mzlab_llm::policy::{Level, Profile};
use mzlab_llm::provider::builtin;
use mzlab_llm::router;

const ROLES: &str = include_str!("../data/assist/assist-ruoli-1.json");
const PROMPT: &str = include_str!("../data/assist/02_router.md");
const GOOD: &str = r#"{"route":"chart","need_data":["views"],"language":"it","suspicious":false}"#;

fn machine() -> State {
    let p = builtin().into_iter().find(|p| p.id == "anthropic").unwrap();
    let mut s = State::new(
        p,
        "claude-sonnet-5-5",
        common::tools(),
        Vec::new(),
        Level::L1,
        Profile::Research,
    );
    let book = with_prompts(&[("02_router.md", PROMPT)]);
    let all: Vec<_> = roles(ROLES)
        .unwrap()
        .into_iter()
        .filter(|r| r.role == "router")
        .collect();
    s.enable_assist(all, book).unwrap();
    s.set_assist_state(
        parse(br#"{"phase":"table","files_open":7,"plan":true,"pending_questions":[]}"#).unwrap(),
    );
    s
}

fn sub(out: Vec<Output>) -> mzlab_llm::provider::Request {
    let mut it = out.into_iter().filter_map(|o| match o {
        Output::Sub { role, request } => {
            assert_eq!(role, "router");
            Some(request)
        }
        Output::Send(_) => panic!("the orchestrator was asked before the router"),
        _ => None,
    });
    let r = it.next().expect("a router call");
    assert!(it.next().is_none());
    r
}

fn sends(out: &[Output]) -> usize {
    out.iter().filter(|o| matches!(o, Output::Send(_))).count()
}

#[test]
fn the_router_request_is_the_role_of_the_data_with_the_message_as_data() {
    let mut s = machine();
    let r = sub(step(
        &mut s,
        Input::User("mostrami i TIC di tutti i file sovrapposti".into()),
    ));
    common::expect_file("req_router.json", &r.body);
    let v = parse(r.body.as_bytes()).unwrap();
    assert_eq!(
        v.get("model").and_then(Value::str),
        Some("claude-haiku-5-5")
    );
    assert!(v.get("tools").is_none());
    assert!(r.body.contains("\"output_config\""));
    assert!(!r.body.contains("temperature"));
    // the user's words travel inside a JSON string, never as part of the prompt
    let sys = v.get("system").map(|x| format!("{x:?}")).unwrap();
    assert!(!sys.contains("mostrami"));
}

#[test]
fn the_route_is_logged_and_the_turn_goes_to_the_orchestrator() {
    let mut s = machine();
    sub(step(&mut s, Input::User("show the TICs".into())));
    let out = step(
        &mut s,
        Input::SubResult {
            role: "router".into(),
            body: Ok(GOOD.into()),
        },
    );
    assert_eq!(sends(&out), 1);
    let r = s.route().unwrap();
    assert_eq!(
        (r.route(), r.need_data(), r.suspicious()),
        ("chart", vec!["views"], false)
    );
    assert!(s.log().contains("\"ev\":\"route\""));
    assert!(s.log().contains("\"ev\":\"sub\""));
    // the next user message asks the router again and forgets the old route
    step(
        &mut s,
        Input::Event(mzlab_llm::event::Event::Stop(
            mzlab_llm::event::StopReason::End,
        )),
    );
    sub(step(&mut s, Input::User("and the blanks?".into())));
    assert!(s.route().is_none());
}

#[test]
fn a_wrong_or_failed_router_answer_costs_the_route_but_not_the_turn() {
    for body in [
        Ok(r#"{"route":"chart"}"#.to_string()),
        Ok(r#"{"route":"hack","need_data":[],"language":"it","suspicious":false}"#.to_string()),
        Ok("not json".to_string()),
        Err("err.llm.network".to_string()),
    ] {
        let mut s = machine();
        sub(step(&mut s, Input::User("hello".into())));
        let out = step(
            &mut s,
            Input::SubResult {
                role: "router".into(),
                body,
            },
        );
        assert_eq!(sends(&out), 1);
        assert!(s.route().is_none());
        assert!(s.log().contains("\"error\":\"err.llm."));
    }
}

#[test]
fn a_stop_ends_the_turn_and_a_late_answer_or_a_stranger_is_ignored() {
    let mut s = machine();
    sub(step(&mut s, Input::User("hello".into())));
    let stranger = step(
        &mut s,
        Input::SubResult {
            role: "grafici".into(),
            body: Ok(GOOD.into()),
        },
    );
    assert!(stranger.is_empty());
    let out = step(&mut s, Input::Stop);
    assert!(out.iter().any(|o| matches!(o, Output::Done)));
    let late = step(
        &mut s,
        Input::SubResult {
            role: "router".into(),
            body: Ok(GOOD.into()),
        },
    );
    assert!(late.is_empty() && s.route().is_none());
    assert_eq!(s.outcome(), Some("stopped"));
}

#[test]
fn the_answer_schema_is_closed() {
    assert!(router::parse(GOOD).is_ok());
    let bad = [
        r#"{"route":"chart","need_data":["views"],"language":"it","suspicious":false,"x":1}"#,
        r#"{"route":"chart","need_data":["files"],"language":"it","suspicious":false}"#,
        r#"{"route":"chart","need_data":[],"language":"ita","suspicious":false}"#,
        r#"{"route":"chart","need_data":[],"language":"IT","suspicious":false}"#,
        r#"{"route":"chart","need_data":[],"language":"it","suspicious":"no"}"#,
        "[]",
    ];
    for b in bad {
        assert!(router::parse(b).is_err(), "{b}");
    }
}

#[test]
fn without_a_router_the_turn_is_as_before() {
    let p = builtin().into_iter().find(|p| p.id == "anthropic").unwrap();
    let mut s = State::new(
        p,
        "claude-sonnet-5-5",
        common::tools(),
        Vec::new(),
        Level::L1,
        Profile::Research,
    );
    assert_eq!(sends(&step(&mut s, Input::User("hi".into()))), 1);
}
