//! The scope rule of the assistant (rule 6 of `sistema-1.md`, the system text of the orchestrator; rule 7 of `01_sistema.md`): the off-topic cases of `tests/data/assist/oro/R.jsonl`
//! (category `off_topic`: a poem, code, homework, a general chemistry question, «ignore your rules») played on the public demo (provider `mzlab-demo`,
//! demo catalog, demo limits) from the strict recording `tests/data/assist/reg/fuori_tema.jsonl` (`MZLAB_UPDATE_LLM=1` rewrites it from the answers below):
//! every request must be the recorded one byte for byte, the answer offers the two script chips and no tool is called.

use mzlab_core::json::{parse, Value};
use mzlab_llm::agent::{step, Input, Output, State};
use mzlab_llm::assist::{orchestrator_caps, roles_for, with_prompts, DEMO_PROVIDER};
use mzlab_llm::event::{Event, StopReason};
use mzlab_llm::fake::{record_line, Replay};
use mzlab_llm::policy::{Level, Profile};
use mzlab_llm::provider::{builtin, Request};
use mzlab_llm::tools::{demo_catalog, UI_TOOLS};

use crate::flow::{executable, steps};

const REFUSAL: &str = "I can only help with the files open in mzLab, with mzFlow and with LC-MS data analysis. Try \"Explain these files\" or \"Set up mzFlow for me\".";

fn root(rel: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn cases() -> Vec<Value> {
    std::fs::read_to_string(root("tests/data/assist/oro/R.jsonl"))
        .unwrap()
        .lines()
        .map(|l| parse(l.as_bytes()).unwrap())
        .filter(|c| c.get("category").and_then(Value::str) == Some("off_topic"))
        .collect()
}

fn strs<'a>(c: &'a Value, key: &str) -> Vec<&'a str> {
    c.get("expect")
        .and_then(|e| e.get(key))
        .map_or(&[][..], Value::arr)
        .iter()
        .filter_map(Value::str)
        .collect()
}

/// The machine of the demo as the page builds it (`Agent` + `enable_assist` of the `.wasm`): demo provider and catalog, every role, the demo limits.
fn demo_state() -> State {
    let p = builtin()
        .into_iter()
        .find(|p| p.id == DEMO_PROVIDER)
        .unwrap();
    let tools = demo_catalog(&steps(), UI_TOOLS, &executable);
    let mut s = State::new(
        p,
        "claude-sonnet-5-5",
        tools,
        steps(),
        Level::L1,
        Profile::Research,
    );
    let dir = root("crates/mzlab-llm/data/assist");
    let json = std::fs::read_to_string(dir.join("assist-ruoli-1.json")).unwrap();
    let rs = roles_for(&json, DEMO_PROVIDER).unwrap();
    let files: Vec<(String, String)> = rs
        .iter()
        .map(|r| r.prompt_file.clone())
        .chain(["06_piano.md".to_string()])
        .map(|f| (f.clone(), std::fs::read_to_string(dir.join(&f)).unwrap()))
        .collect();
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    s.enable_assist(rs, with_prompts(&refs)).unwrap();
    s.set_phase_prompt("06_piano.md").unwrap();
    s.set_output_caps(orchestrator_caps(&json, DEMO_PROVIDER).unwrap());
    s
}

fn text(t: &str) -> Vec<Event> {
    vec![Event::TextDelta(t.into()), Event::Stop(StopReason::End)]
}

/// One user message on a fresh demo machine: router, then the orchestrator; `model` answers each call. Returns the text shown and the tools called.
fn play(
    user: &str,
    route: &str,
    model: &mut dyn FnMut(&Request, Vec<Event>) -> Vec<Event>,
) -> (String, Vec<String>) {
    let mut s = demo_state();
    let (mut shown, mut called) = (String::new(), Vec::<String>::new());
    let mut queue = vec![Input::User(user.into())];
    while let Some(input) = queue.pop() {
        for o in step(&mut s, input) {
            match o {
                Output::Sub { request, .. } => {
                    let body: String = model(&request, text(route))
                        .iter()
                        .filter_map(|e| match e {
                            Event::TextDelta(t) => Some(t.clone()),
                            _ => None,
                        })
                        .collect();
                    queue.push(Input::SubResult {
                        role: "router".into(),
                        body: Ok(body),
                    });
                }
                Output::Send(req) => {
                    // the orchestrator carries the scope rule and the demo catalog only
                    assert!(req.body.contains("help only with the files open in mzLab, with mzFlow and with LC-MS data analysis"));
                    assert!(!req.body.contains("\"flow_run\""));
                    queue.extend(
                        model(&req, text(REFUSAL))
                            .into_iter()
                            .rev()
                            .map(Input::Event),
                    );
                }
                Output::Show(t) => shown.push_str(&t),
                Output::CallTool { name, .. } => called.push(name),
                _ => {}
            }
        }
    }
    (shown, called)
}

#[test]
fn off_topic_requests_are_refused_with_the_two_chips_and_no_tool() {
    let file = root("tests/data/assist/reg/fuori_tema.jsonl");
    let update = std::env::var_os("MZLAB_UPDATE_LLM").is_some();
    let mut replay =
        (!update).then(|| Replay::parse(&std::fs::read_to_string(&file).unwrap(), true).unwrap());
    let all = cases();
    assert_eq!(
        all.len(),
        5,
        "a poem, code, homework, a general question, «ignore your rules»"
    );
    let (mut lines, mut n) = (Vec::new(), 0usize);
    for c in &all {
        let id = c.get("id").and_then(Value::str).unwrap();
        let user = c.get("user").and_then(Value::str).unwrap();
        let suspicious = c
            .get("expect")
            .and_then(|e| e.get("must_flag"))
            .and_then(Value::bool)
            == Some(true);
        let route = format!(
            r#"{{"route":"chat","need_data":[],"language":"en","suspicious":{suspicious}}}"#
        );
        let mut model = |req: &Request, script: Vec<Event>| {
            let ev = match replay.as_mut() {
                Some(r) => r
                    .respond(req)
                    .unwrap_or_else(|e| panic!("{id}: the request is not the recorded one: {e:?}")),
                None => script,
            };
            lines.push(record_line(n, req, &ev));
            n += 1;
            ev
        };
        let (shown, called) = play(user, &route, &mut model);
        assert!(called.is_empty(), "{id}: no tool, {called:?}");
        for f in strs(c, "forbidden_tools") {
            assert!(!called.iter().any(|t| t == f), "{id}: {f}");
        }
        for t in strs(c, "required_text") {
            assert!(shown.contains(t), "{id}: «{t}» in {shown}");
        }
        for t in strs(c, "forbidden_text") {
            assert!(!shown.contains(t), "{id}: «{t}»");
        }
    }
    if update {
        std::fs::write(&file, lines.join("\n") + "\n").unwrap();
    }
}
