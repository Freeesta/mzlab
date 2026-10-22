//! Helpers shared by the adapter tests: the files in `tests/data`, the expected-file check and a few fixtures.
#![allow(dead_code)]

use std::path::PathBuf;

use mzlab_llm::conv::{Conversation, Part};
use mzlab_llm::event::Event;
use mzlab_llm::provider::{builtin, Adapter, Provider, Stream};
use mzlab_llm::tools::{Class, Tool};

pub fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name)
}

pub fn read(name: &str) -> String {
    std::fs::read_to_string(data(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Compares `got` with the expected file, or rewrites the file when asked.
pub fn expect_file(name: &str, got: &str) {
    if std::env::var("MZLAB_UPDATE_LLM").is_ok() {
        std::fs::write(data(name), got).unwrap();
    }
    assert_eq!(
        got,
        read(name),
        "{name} differs (MZLAB_UPDATE_LLM=1 rewrites it)"
    );
}

pub fn tools() -> Vec<Tool> {
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

pub fn provider(id: &str) -> Provider {
    builtin().into_iter().find(|p| p.id == id).unwrap()
}

pub fn conversation(p: &Provider) -> Conversation {
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

pub fn run(adapter: Adapter, prompted: bool, chunks: impl Iterator<Item = Vec<u8>>) -> Vec<Event> {
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

pub fn lines(e: &[Event]) -> String {
    e.iter().map(|e| e.to_json() + "\n").collect()
}

pub fn adapter(name: &str) -> Adapter {
    match name.split('_').next().unwrap() {
        "anthropic" => Adapter::Anthropic,
        "gemini" => Adapter::Gemini,
        _ => Adapter::OpenAiCompat,
    }
}
