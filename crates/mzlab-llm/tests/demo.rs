//! The public demo: the provider `mzlab-demo` (mzLab's proxy, no key, placeholder host), the request as it leaves (`provider::demo::for_send`: host,
//! session and owner headers, body untouched) and the `demo_*` output limits of `assist-ruoli-1.json` the proxy enforces per role.
mod common;

use mzlab_core::json::parse;
use mzlab_llm::agent::{step, Input, Output, State};
use mzlab_llm::assist::{orchestrator_caps, roles, roles_for, with_prompts, Caps, DEMO_PROVIDER};
use mzlab_llm::policy::{Level, Profile};
use mzlab_llm::provider::demo::{for_send, host_ok, session_ok, PLACEHOLDER};
use mzlab_llm::provider::{request, Request};

const ROLES: &str = include_str!("../data/assist/assist-ruoli-1.json");
const ROUTER: &str = include_str!("../data/assist/02_router.md");
const PLAN: &str = include_str!("../data/assist/06_piano.md");
const HOST: &str = "https://assist.example.org";
const SESSION: &str = "s3ss10n_AbC-0123";

fn demo_request() -> Request {
    let p = common::provider(DEMO_PROVIDER);
    let c = common::conversation(&p);
    request(&p, &c, &common::tools()).unwrap()
}

#[test]
fn the_demo_provider_is_the_proxy_without_a_key_and_only_sonnet_and_haiku() {
    let p = common::provider(DEMO_PROVIDER);
    assert_eq!(p.base_url, PLACEHOLDER);
    assert!(p.auth.header.is_empty(), "no key slot");
    assert!(
        p.extra_headers.is_empty(),
        "the proxy adds anthropic-version"
    );
    assert_eq!(
        p.session_headers,
        Some(("x-mzlab-session".into(), "x-mzlab-owner".into()))
    );
    let ids: Vec<&str> = p.models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["claude-sonnet-5-5", "claude-haiku-5-5"]);
    assert!(p
        .models
        .iter()
        .all(|m| !m.vision && m.price.input > 0.0 && !m.price.as_of.is_empty()));
    let r = demo_request();
    assert_eq!(r.url, format!("{PLACEHOLDER}/v1/messages"));
}

#[test]
fn the_request_leaves_with_the_host_and_the_session_and_the_same_body() {
    let p = common::provider(DEMO_PROVIDER);
    let before = demo_request();
    let r = for_send(&p, before.clone(), HOST, SESSION, "").unwrap();
    assert_eq!(r.url, format!("{HOST}/v1/messages"));
    assert_eq!(r.body, before.body);
    assert_eq!(
        r.headers.last().unwrap(),
        &("x-mzlab-session".to_string(), SESSION.to_string())
    );
    assert!(!r.headers.iter().any(|(k, _)| k == "x-mzlab-owner"));
    let owner = for_send(&p, before.clone(), HOST, SESSION, "a phrase").unwrap();
    assert_eq!(
        owner.headers.last().unwrap(),
        &("x-mzlab-owner".to_string(), "a phrase".to_string())
    );
    assert_eq!(owner.body, before.body);
}

#[test]
fn no_host_a_bad_session_or_owner_stop_the_request() {
    let p = common::provider(DEMO_PROVIDER);
    let key =
        |h: &str, s: &str, o: &str| for_send(&p, demo_request(), h, s, o).err().map(|e| e.key);
    assert_eq!(key("", SESSION, ""), Some("err.llm.network"));
    assert_eq!(
        key("http://assist.example.org", SESSION, ""),
        Some("err.llm.network")
    );
    assert_eq!(
        key("https://assist.example.org/x", SESSION, ""),
        Some("err.llm.network")
    );
    assert_eq!(key(HOST, "short", ""), Some("err.llm.conversation"));
    assert_eq!(
        key(HOST, "has space in it", ""),
        Some("err.llm.conversation")
    );
    assert_eq!(
        key(HOST, SESSION, "line\nbreak"),
        Some("err.llm.conversation")
    );
    assert!(host_ok("https://mzlab-assist.someone.workers.dev") && host_ok("https://a.b:8443"));
    assert!(!host_ok("https://") && !host_ok("https://a.b:") && !host_ok("https://A.B"));
    assert!(session_ok(&"a".repeat(64)) && !session_ok(&"a".repeat(65)));
}

#[test]
fn another_provider_leaves_unchanged() {
    let p = common::provider("anthropic");
    let r = request(&p, &common::conversation(&p), &common::tools()).unwrap();
    assert_eq!(for_send(&p, r.clone(), "", "", "").unwrap(), r);
}

#[test]
fn the_demo_limits_are_those_of_the_proxy_and_never_above_the_full_ones() {
    let want = [
        ("router", 200),
        ("metadati", 2000),
        ("cromatografia", 2000),
        ("parametri", 2000),
        ("interprete", 2000),
        ("grafici", 800),
    ];
    let full = roles(ROLES).unwrap();
    let demo = roles_for(ROLES, DEMO_PROVIDER).unwrap();
    for (role, n) in want {
        let d = demo.iter().find(|r| r.role == role).unwrap();
        let f = full.iter().find(|r| r.role == role).unwrap();
        assert_eq!(d.max_tokens, n, "{role}");
        assert!(d.max_tokens <= f.max_tokens, "{role}");
        let haiku = matches!(role, "router" | "grafici");
        assert_eq!(
            d.model,
            if haiku {
                "claude-haiku-5-5"
            } else {
                "claude-sonnet-5-5"
            },
            "{role}"
        );
        assert_eq!(d.effort, if haiku { "low" } else { "medium" }, "{role}");
    }
    assert_eq!(roles_for(ROLES, "anthropic").unwrap(), full);
    assert_eq!(
        orchestrator_caps(ROLES, DEMO_PROVIDER).unwrap(),
        Caps {
            turn: 1500,
            plan: 3000
        }
    );
    assert_eq!(
        orchestrator_caps(ROLES, "anthropic").unwrap(),
        Caps {
            turn: 3000,
            plan: 6000
        }
    );
    assert!(roles(&ROLES.replacen("\"demo_max_tokens\":200,", "", 1)).is_err());
}

#[test]
fn the_orchestrator_of_the_demo_asks_for_1500_tokens_and_3000_after_the_plan_prompt() {
    let p = common::provider(DEMO_PROVIDER);
    let mut s = State::new(
        p,
        "claude-sonnet-5-5",
        common::tools(),
        Vec::new(),
        Level::L1,
        Profile::Research,
    );
    let book = with_prompts(&[("02_router.md", ROUTER), ("06_piano.md", PLAN)]);
    let router: Vec<_> = roles_for(ROLES, DEMO_PROVIDER)
        .unwrap()
        .into_iter()
        .filter(|r| r.role == "router")
        .collect();
    s.enable_assist(router, book).unwrap();
    s.set_phase_prompt("06_piano.md").unwrap();
    s.set_output_caps(orchestrator_caps(ROLES, DEMO_PROVIDER).unwrap());
    assert_eq!(s.conversation().max_output_tokens, Some(1500));
    let out = step(&mut s, Input::User("Set up mzFlow for me".into()));
    let sub = out.iter().find_map(|o| match o {
        Output::Sub { request, .. } => Some(request.clone()),
        _ => None,
    });
    let body = parse(sub.expect("the router first").body.as_bytes()).unwrap();
    assert_eq!(body.get("max_tokens").and_then(|v| v.f64()), Some(200.0));
    let route = r#"{"route":"plan","need_data":[],"language":"en","suspicious":false}"#;
    let out = step(
        &mut s,
        Input::SubResult {
            role: "router".into(),
            body: Ok(route.into()),
        },
    );
    let sent = out.iter().find_map(|o| match o {
        Output::Send(r) => Some(r.clone()),
        _ => None,
    });
    let body = parse(sent.expect("the orchestrator").body.as_bytes()).unwrap();
    assert_eq!(body.get("max_tokens").and_then(|v| v.f64()), Some(3000.0));
}

#[test]
fn the_keys_of_the_proxy_reach_the_page_with_only_their_short_params() {
    use mzlab_llm::provider::http_error;
    let line = |s: u16, b: &str| http_error(s, b).to_json();
    let closed = line(
        403,
        r#"{"error":"err.llm.proxy.closed","error_key":"err.llm.proxy.closed","params":{}}"#,
    );
    assert!(closed.contains("\"err.llm.proxy.closed\""), "{closed}");
    let budget = line(
        503,
        r#"{"error_key":"err.llm.proxy.budget","params":{"limit":"day"}}"#,
    );
    assert!(
        budget.contains("\"err.llm.proxy.budget\"") && budget.contains("\"limit\":\"day\""),
        "{budget}"
    );
    let spend = line(
        429,
        r#"{"error_key":"err.llm.limit.spend","params":{"limit":"session","cap":"500000"}}"#,
    );
    assert!(
        spend.contains("\"err.llm.limit.spend\"")
            && spend.contains("\"limit\":\"session\"")
            && !spend.contains("500000"),
        "{spend}"
    );
    for k in ["off", "body", "origin"] {
        let b = format!(r#"{{"error_key":"err.llm.proxy.{k}","params":{{"field":"x y"}}}}"#);
        let l = line(400, &b);
        assert!(
            l.contains(&format!("\"err.llm.proxy.{k}\"")) && !l.contains("x y"),
            "{l}"
        );
    }
    // a vendor's 403 and an unknown key stay what they were
    assert!(line(403, r#"{"error_key":"err.llm.proxy.other"}"#).contains("\"err.llm.auth\""));
    assert!(line(
        403,
        r#"{"error":{"type":"permission_error","message":"no"}}"#
    )
    .contains("\"err.llm.auth\""));
}

#[test]
fn the_spend_header_is_two_whole_numbers_with_a_cap() {
    use mzlab_llm::provider::demo::spend_json;
    assert_eq!(
        spend_json("120000/500000", false).as_deref(),
        Some(r#"{"spent":120000,"cap":500000,"soft":false}"#)
    );
    assert_eq!(
        spend_json(" 400000/500000 ", true).as_deref(),
        Some(r#"{"spent":400000,"cap":500000,"soft":true}"#)
    );
    for bad in ["", "1/0", "a/2", "1.5/2", "-1/2", "1/2/3", "1"] {
        assert_eq!(spend_json(bad, false), None, "{bad}");
    }
}
