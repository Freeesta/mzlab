//! The adapters without network: request bodies against the files in `tests/data`, synthetic streams (whole and cut at every byte) against their
//! expected events, the recordings, the log and the cost. `MZLAB_UPDATE_LLM=1 cargo test -p mzlab-llm` rewrites the expected files.

use std::path::PathBuf;

use mzlab_llm::conv::{Conversation, Part};
use mzlab_llm::cost::estimate;
use mzlab_llm::event::{Event, StopReason, Usage};
use mzlab_llm::fake::{record_line, request_sha256, Replay};
use mzlab_llm::log::Log;
use mzlab_llm::provider::{builtin, http_error, request, Adapter, Provider, Stream};
use mzlab_llm::tools::{Class, Tool};

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name)
}

fn read(name: &str) -> String {
    std::fs::read_to_string(data(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Compares `got` with the expected file, or rewrites the file when asked.
fn expect_file(name: &str, got: &str) {
    if std::env::var("MZLAB_UPDATE_LLM").is_ok() {
        std::fs::write(data(name), got).unwrap();
    }
    assert_eq!(
        got,
        read(name),
        "{name} differs (MZLAB_UPDATE_LLM=1 rewrites it)"
    );
}

fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "flow_check",
            class: Class::Read,
            description: "Check the plan.",
            input_schema: "{\"type\":\"object\",\"properties\":{\"plan\":{\"type\":\"string\"}}}"
                .into(),
        },
        Tool {
            name: "flow_run",
            class: Class::Run,
            description: "Run the plan.",
            input_schema: "{\"type\":\"object\"}".into(),
        },
    ]
}

fn provider(id: &str) -> Provider {
    builtin().into_iter().find(|p| p.id == id).unwrap()
}

fn conversation(p: &Provider) -> Conversation {
    let mut c = Conversation::parse(&read("conv.json")).unwrap();
    c.provider = p.id.clone();
    c.model = p.models[0].id.clone();
    if !p.models[0].vision {
        c.messages[0]
            .parts
            .retain(|x| !matches!(x, Part::Image { .. }));
    }
    c
}

#[test]
fn request_bodies_match_the_files() {
    for id in ["anthropic", "openai", "gemini", "ollama"] {
        let p = provider(id);
        let r = request(&p, &conversation(&p), &tools()).unwrap();
        expect_file(&format!("req_{id}.json"), &r.body);
        assert!(!r.body.contains("x-api-key") && !r.body.to_lowercase().contains("bearer"));
        assert_eq!(r.auth, p.auth);
    }
    let r = request(
        &provider("anthropic"),
        &conversation(&provider("anthropic")),
        &[],
    )
    .unwrap();
    assert!(!r.body.contains("\"tools\""));
    assert!(r
        .headers
        .contains(&("anthropic-version".to_string(), "2023-06-01".to_string())));
}

#[test]
fn urls_and_modes() {
    let g = provider("gemini");
    assert_eq!(request(&g, &conversation(&g), &[]).unwrap().url, "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:streamGenerateContent?alt=sse");
    let o = provider("ollama");
    let r = request(&o, &conversation(&o), &tools()).unwrap();
    assert_eq!(r.url, "http://localhost:11434/v1/chat/completions");
    assert!(r.auth.header.is_empty());
    // prompted: the schemas are in the system text, not in a tools field
    assert!(r.body.contains("Run the plan.") && !r.body.contains("\"tools\""));
    // none: nothing about tools at all
    let mut n = o.clone();
    n.models[0].tools = mzlab_llm::provider::ToolMode::None;
    let r = request(&n, &conversation(&n), &tools()).unwrap();
    assert!(!r.body.contains("Run the plan."));
    // an image for a model without vision is refused
    let mut c = conversation(&o);
    c.messages[0].parts.push(Part::Image {
        png_b64: "x".into(),
    });
    assert_eq!(
        request(&o, &c, &[]).unwrap_err().key,
        "err.llm.conversation"
    );
    // a conversation for another provider or an unknown model is refused
    let mut c = conversation(&o);
    c.model = "nope".into();
    assert_eq!(
        request(&o, &c, &[]).unwrap_err().key,
        "err.llm.conversation"
    );
    c.model = o.models[0].id.clone();
    c.provider = "x".into();
    assert_eq!(
        request(&o, &c, &[]).unwrap_err().key,
        "err.llm.conversation"
    );
}

fn run(adapter: Adapter, prompted: bool, chunks: impl Iterator<Item = Vec<u8>>) -> Vec<Event> {
    let mut s = Stream::new(adapter);
    if prompted {
        s = s.prompted();
    }
    let mut out = Vec::new();
    for c in chunks {
        out.extend(s.push(&c));
    }
    out.extend(s.finish());
    out
}

fn lines(e: &[Event]) -> String {
    e.iter().map(|e| e.to_json() + "\n").collect()
}

fn adapter(name: &str) -> Adapter {
    match name.split('_').next().unwrap() {
        "anthropic" => Adapter::Anthropic,
        "gemini" => Adapter::Gemini,
        _ => Adapter::OpenAiCompat,
    }
}

#[test]
fn streams_give_the_expected_events_even_cut_at_every_byte() {
    let cases = [
        "anthropic_text",
        "anthropic_tool",
        "anthropic_truncated",
        "anthropic_error",
        "openai_text",
        "openai_tool",
        "openai_truncated",
        "openai_error",
        "gemini_text",
        "gemini_tool",
        "gemini_truncated",
        "gemini_error",
        "ollama_prompted",
    ];
    for name in cases {
        let bytes = std::fs::read(data(&format!("{name}.sse"))).unwrap();
        let prompted = name == "ollama_prompted";
        let whole = run(adapter(name), prompted, std::iter::once(bytes.clone()));
        expect_file(&format!("{name}.events"), &lines(&whole));
        for size in [1, 2, 7] {
            let cut = run(
                adapter(name),
                prompted,
                bytes.chunks(size).map(<[u8]>::to_vec),
            );
            assert_eq!(cut, whole, "{name} cut every {size} bytes");
        }
    }
}

#[test]
fn http_errors_map_to_keys() {
    for name in [
        "anthropic_401",
        "anthropic_429",
        "openai_401",
        "openai_429",
        "gemini_401",
        "gemini_429",
    ] {
        let text = read(&format!("{name}.http"));
        let (status, body) = text.split_once('\n').unwrap();
        expect_file(
            &format!("{name}.events"),
            &lines(&[http_error(status.parse().unwrap(), body)]),
        );
    }
    let key = |s, b| match http_error(s, b) {
        Event::Error { key, .. } => key,
        e => panic!("{e:?}"),
    };
    assert_eq!(
        key(
            400,
            "{\"error\":{\"message\":\"prompt is too long: 300000 tokens\"}}"
        ),
        "err.llm.context"
    );
    assert_eq!(key(413, "x"), "err.llm.context");
    assert_eq!(key(500, "oops"), "err.llm.network");
    assert_eq!(key(0, ""), "err.llm.bad_response");
    assert_eq!(key(418, "{}"), "err.llm.bad_response");
    // an echoed key never travels in the event
    let text = read("openai_401.http");
    let e = http_error(401, text.split_once('\n').unwrap().1);
    assert!(!e.to_json().contains("abc123"));
}

#[test]
fn prompted_without_a_block_stays_text() {
    let sse = b"data: {\"choices\":[{\"delta\":{\"content\":\"Just words\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    let e = run(Adapter::OpenAiCompat, true, std::iter::once(sse.to_vec()));
    assert_eq!(
        e,
        vec![
            Event::TextDelta("Just words".into()),
            Event::Stop(StopReason::End)
        ]
    );
}

fn recording() -> (Provider, Conversation, String) {
    let p = provider("anthropic");
    let c = conversation(&p);
    let r = request(&p, &c, &tools()).unwrap();
    let ev = run(
        Adapter::Anthropic,
        false,
        std::iter::once(std::fs::read(data("anthropic_tool.sse")).unwrap()),
    );
    let text = format!(
        "{}\n{}\n",
        record_line(0, &r, &ev),
        record_line(
            1,
            &r,
            &[Event::TextDelta("ok".into()), Event::Stop(StopReason::End)]
        )
    );
    (p, c, text)
}

#[test]
fn replay_repeats_and_strict_sees_a_changed_request() {
    let (p, c, text) = recording();
    let r = request(&p, &c, &tools()).unwrap();
    let mut a = Replay::parse(&text, true).unwrap();
    let first = a.respond(&r).unwrap();
    assert_eq!(a.respond(&r).unwrap().len(), 2);
    assert_eq!(a.respond(&r).unwrap_err().key, "err.llm.replay.exhausted");
    a.reset();
    assert_eq!(a.respond(&r).unwrap(), first);
    assert_eq!(
        Replay::parse(&text, false).unwrap().at(0, &r).unwrap(),
        first
    );
    // another system text = another body
    let mut changed = c.clone();
    changed.system.push('!');
    let r2 = request(&p, &changed, &tools()).unwrap();
    assert_ne!(request_sha256(&r), request_sha256(&r2));
    assert_eq!(
        Replay::parse(&text, true)
            .unwrap()
            .respond(&r2)
            .unwrap_err()
            .key,
        "err.llm.replay.mismatch"
    );
    assert_eq!(
        Replay::parse(&text, false).unwrap().respond(&r2).unwrap(),
        first
    );
    assert_eq!(
        Replay::parse("{\"schema\":\"x\"}", false)
            .err()
            .unwrap()
            .key,
        "err.llm.replay.format"
    );
}

fn fake_key(prefix: &str) -> String {
    format!("{prefix}FAKEKEY1234567890")
}

#[test]
fn log_has_sorted_keys_and_refuses_keys() {
    let mut log = Log::new();
    log.event(
        0,
        &Event::Usage(Usage {
            input: 1,
            output: 2,
            cache_read: 3,
        }),
    )
    .unwrap();
    log.event(0, &Event::TextDelta("task-force and ask-me".into()))
        .unwrap();
    assert_eq!(
        log.text(),
        "{\"cache_read\":3,\"ev\":\"usage\",\"in\":1,\"out\":2,\"schema\":\"llmlog/1\",\"seq\":0,\"turn\":0}\n{\"ev\":\"text_delta\",\"schema\":\"llmlog/1\",\"seq\":1,\"text\":\"task-force and ask-me\",\"turn\":0}\n"
    );
    for prefix in [
        ["s", "k-"].concat(),
        ["s", "k-ant-"].concat(),
        ["AI", "za"].concat(),
    ] {
        let e = log
            .event(
                0,
                &Event::TextDelta(format!("my key is {}", fake_key(&prefix))),
            )
            .unwrap_err();
        assert_eq!(e.key, "err.llm.log.secret");
    }
    assert_eq!(log.text().lines().count(), 2);
}

#[test]
fn cost_is_none_without_a_price() {
    let u = Usage {
        input: 1_000_000,
        output: 500_000,
        cache_read: 0,
    };
    let mut m = provider("openai").models[0].clone();
    assert_eq!(estimate(&u, &m), None);
    m.price.input = 2.0;
    m.price.output = 8.0;
    assert_eq!(estimate(&u, &m), Some(6.0));
}

#[test]
fn shipped_data_has_no_invented_price_and_schemas_are_json() {
    for p in builtin() {
        assert!(
            p.models
                .iter()
                .all(|m| m.price.input == 0.0 && m.price.output == 0.0),
            "{}",
            p.id
        );
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/schemi");
    for n in ["llm-provider-1", "llm-conv-1", "llmlog-1", "llmreg-1"] {
        let t =
            std::fs::read(root.join(format!("{n}.json"))).unwrap_or_else(|e| panic!("{n}: {e}"));
        mzlab_core::json::parse(&t).unwrap_or_else(|_| panic!("{n} is not JSON"));
    }
}
