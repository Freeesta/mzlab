//! The Anthropic request with its per-role options, the cache writes in the stream and the workspace-limit errors. No network.
//! `MZLAB_UPDATE_LLM=1 cargo test -p mzlab-llm` rewrites the expected files.

mod common;
use common::*;

use mzlab_llm::conv::{Conversation, Part, Role};
use mzlab_llm::event::{Event, Usage};
use mzlab_llm::provider::{
    http_error, request, request_with, retry_delay, Adapter, CacheTtl, Effort, Options,
};

fn assist_options() -> Options {
    Options {
        cache_ttl: Some(CacheTtl::Hour1),
        cache_auto: true,
        effort: Some(Effort::Medium),
        adaptive_thinking: true,
        strict_tools: true,
        format_schema: None,
    }
}

#[test]
fn anthropic_request_with_cache_effort_thinking_and_strict_tools() {
    let p = provider("anthropic");
    let mut c = conversation(&p);
    c.messages.push(mzlab_llm::conv::Message {
        role: Role::System,
        parts: vec![Part::Text("Answer in one line.".into())],
    });
    let r = request_with(&p, &c, &tools(), &assist_options()).unwrap();
    expect_file("req_anthropic_assist.json", &r.body);
    for no in ["temperature", "top_p", "top_k", "x-api-key"] {
        assert!(!r.body.contains(no), "{no}");
    }
    // the explicit breakpoint is on the system block, before the automatic one at the top
    assert!(r.body.contains(
        "\"system\":[{\"type\":\"text\",\"text\":\"You are the mzLab assistant.\",\"cache_control\":{\"type\":\"ephemeral\",\"ttl\":\"1h\"}}]"
    ));
    assert!(r.body.contains("\"thinking\":{\"type\":\"adaptive\"}"));
    assert!(r.body.contains("\"output_config\":{\"effort\":\"medium\"}"));
    assert_eq!(r.body.matches("\"strict\":true").count(), 2);
    assert!(r
        .body
        .contains("{\"role\":\"system\",\"content\":\"Answer in one line.\"}"));
    // the plain request is unchanged by the options existing
    assert_eq!(
        request(&p, &conversation(&p), &tools()).unwrap().body,
        request_with(&p, &conversation(&p), &tools(), &Options::default())
            .unwrap()
            .body
    );
}

#[test]
fn answer_schema_goes_in_output_config_and_only_without_tools() {
    let p = provider("anthropic");
    let c = conversation(&p);
    let o = Options {
        effort: Some(Effort::Low),
        format_schema: Some("{\"type\":\"object\",\"additionalProperties\":false}".into()),
        ..Options::default()
    };
    let r = request_with(&p, &c, &[], &o).unwrap();
    assert!(r.body.contains(
        "\"output_config\":{\"effort\":\"low\",\"format\":{\"type\":\"json_schema\",\"schema\":{\"type\":\"object\",\"additionalProperties\":false}}}"
    ));
    mzlab_core::json::parse(r.body.as_bytes()).unwrap();
    assert_eq!(
        request_with(&p, &c, &tools(), &o).unwrap_err().key,
        "err.llm.conversation"
    );
    let bad = Options {
        format_schema: Some("{not json".into()),
        ..Options::default()
    };
    assert_eq!(
        request_with(&p, &c, &[], &bad).unwrap_err().key,
        "err.llm.conversation"
    );
}

#[test]
fn mid_conversation_system_message_in_the_other_vendors() {
    for id in ["openai", "gemini"] {
        let p = provider(id);
        let mut c = conversation(&p);
        c.messages.push(mzlab_llm::conv::Message {
            role: Role::System,
            parts: vec![Part::Text("Answer in one line.".into())],
        });
        let r = request(&p, &c, &[]).unwrap();
        assert!(r.body.contains("Answer in one line."), "{id}");
    }
    let c = Conversation::parse(&Conversation::to_json(&{
        let p = provider("anthropic");
        let mut c = conversation(&p);
        c.messages.push(mzlab_llm::conv::Message {
            role: Role::System,
            parts: vec![Part::Text("x".into())],
        });
        c
    }))
    .unwrap();
    assert_eq!(c.messages.last().unwrap().role, Role::System);
}

#[test]
fn cache_writes_are_read_from_the_stream_by_lifetime() {
    let bytes = std::fs::read(data("anthropic_cache.sse")).unwrap();
    let whole = run(Adapter::Anthropic, false, std::iter::once(bytes.clone()));
    expect_file("anthropic_cache.events", &lines(&whole));
    for size in [1, 7] {
        let cut = run(
            Adapter::Anthropic,
            false,
            bytes.chunks(size).map(<[u8]>::to_vec),
        );
        assert_eq!(cut, whole, "cut every {size} bytes");
    }
    let usage = |e: &[Event]| {
        e.iter()
            .find_map(|e| match e {
                Event::Usage(u) => Some(*u),
                _ => None,
            })
            .unwrap()
    };
    let u = usage(&whole);
    assert_eq!(
        (u.cache_write_5m, u.cache_write_1h, u.cache_read),
        (100, 24_900, 0)
    );
    // without the detail the total is the 5-minute write
    let plain = b"event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":3,\"output_tokens\":1,\"cache_creation_input_tokens\":50}}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
    let u = usage(&run(
        Adapter::Anthropic,
        false,
        std::iter::once(plain.to_vec()),
    ));
    assert_eq!((u.cache_write_5m, u.cache_write_1h), (50, 0));
    // the recording of an event round-trips with the new fields, and old lines (no writes) keep their bytes
    let e = Event::Usage(Usage {
        input: 1,
        output: 2,
        cache_read: 3,
        cache_write_5m: 4,
        cache_write_1h: 5,
    });
    let v = mzlab_core::json::parse(e.to_json().as_bytes()).unwrap();
    assert_eq!(Event::from_value(&v).unwrap(), e);
    let old = Event::Usage(Usage {
        input: 1,
        output: 2,
        cache_read: 3,
        ..Usage::default()
    });
    assert!(!old.to_json().contains("cache_write"));
}

#[test]
fn workspace_limit_is_its_own_key_and_a_429_without_retry_after_is_final() {
    let key = |s, b: &str| match http_error(s, b) {
        Event::Error { key, .. } => key,
        e => panic!("{e:?}"),
    };
    let msg = "{\"type\":\"error\",\"error\":{\"type\":\"invalid_request_error\",\"message\":\"You have reached your specified workspace API usage limits. You will regain access on 2026-11-01.\"}}";
    assert_eq!(key(400, msg), "err.llm.limit.workspace");
    assert_eq!(
        key(
            429,
            "{\"error\":{\"type\":\"enforced_spend_limit_reached\",\"message\":\"limit\"}}"
        ),
        "err.llm.limit.workspace"
    );
    // a plain rate limit stays a rate limit
    assert_eq!(key(429, &read("anthropic_429.http")[4..]), "err.llm.rate");
    assert_eq!(retry_delay(429, Some(7), 0), Some(7));
    assert_eq!(retry_delay(429, Some(7), 1), Some(7));
    assert_eq!(retry_delay(429, Some(7), 2), None);
    assert_eq!(retry_delay(429, None, 0), None);
    assert_eq!(retry_delay(400, Some(7), 0), None);
}
