//! The readers of the data assistant seen from outside: the request of a role for the data it may see, and the verdict on its answer. A `lettura/1` or
//! `param-scelte/1` that `reading::validate` accepts goes to the orchestrator as a summary (at most `MAX_SUMMARY_TOKENS`, never the input of the reader);
//! one with problems goes back to the reader once with the problems, and a second failure drops it. Used by the wasm class `Assist` and the tests.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::step::Step;

use crate::assist::Role;
use crate::provider::{Provider, Request};
use crate::reading::{problems_json, validate};
use crate::subcall;
use crate::tools::value;

/// What the orchestrator may see of one reader: `ceil(bytes / 3)` tokens, the estimate of `compact`.
pub const MAX_SUMMARY_TOKENS: usize = 1500;

const ACTION: &str = r#"{"type":"object","additionalProperties":false,"required":["type"],"properties":{"type":{"enum":["none","ask","set_param","add_step","split_series","chart"]},"ref":{"type":"string"},"step":{"type":"string"},"algorithm":{"type":"string"},"name":{"type":"string"},"value":{"anyOf":[{"type":"number"},{"type":"string"},{"type":"boolean"},{"type":"array","items":{"type":"number"}}]}}}"#;
const QUESTIONS: &str = r#"{"type":"array","items":{"type":"object","additionalProperties":false,"required":["id","text","options"],"properties":{"id":{"type":"string"},"text":{"type":"string"},"options":{"type":"array","items":{"type":"string"}}}}}"#;
const NUMBERS: &str = r#"{"type":"array","items":{"anyOf":[{"type":"string"},{"type":"number"}]}}"#;

/// The structure `output_config.format` forces; the limits and the cross-checks are `reading::validate`'s.
fn format(schema: &str) -> Option<String> {
    let finding = format!(
        r#"{{"type":"object","additionalProperties":false,"required":["id","topic","statement","evidence","confidence","action"],"properties":{{"id":{{"type":"string"}},"topic":{{"type":"string"}},"statement":{{"type":"string"}},"evidence":{{"type":"array","items":{{"type":"string"}}}},"confidence":{{"enum":["high","medium","low"]}},"action":{ACTION}}}}}"#
    );
    let choice = format!(
        r#"{{"type":"object","additionalProperties":false,"required":["step","name","value","source","reason"],"properties":{{"step":{{"type":"string"}},"algorithm":{{"type":"string"}},"name":{{"type":"string"}},"value":{},"source":{{"type":"string"}},"reason":{{"type":"string"}}}}}}"#,
        r#"{"anyOf":[{"type":"number"},{"type":"string"},{"type":"boolean"},{"type":"array","items":{"type":"number"}}]}"#
    );
    match schema {
        "lettura/1" => Some(format!(
            r#"{{"type":"object","additionalProperties":false,"required":["schema","kind","findings"],"properties":{{"schema":{{"enum":["lettura/1"]}},"kind":{{"enum":["acquisition","chromatography","table"]}},"findings":{{"type":"array","items":{finding}}},"questions":{QUESTIONS},"numbers":{NUMBERS}}}}}"#
        )),
        "param-scelte/1" => Some(format!(
            r#"{{"type":"object","additionalProperties":false,"required":["schema","choices"],"properties":{{"schema":{{"enum":["param-scelte/1"]}},"choices":{{"type":"array","items":{choice}}},"questions":{QUESTIONS},"numbers":{NUMBERS}}}}}"#
        )),
        _ => None,
    }
}

fn bad(field: &str) -> Error {
    Error::new("err.llm.assist.call").with("field", field)
}

/// The call of `role` for `input`, the data blocks it may see keyed by source (`{"ispezione/1": ...}`).
pub fn request(provider: &Provider, role: &Role, prompt: &str, input: &Value) -> Result<Request> {
    let mut o = Out::default();
    value(&mut o, input);
    subcall::build(
        provider,
        role,
        prompt,
        o.0,
        format(&role.output_schema).as_deref(),
    )
}

/// What to do with the answer of a reader.
pub enum Verdict {
    /// The summary for the orchestrator: the reading as compact JSON.
    Accepted(String),
    /// The reader is asked again with its own answer and the problems (`problems_json`); the request is ready to send.
    Retry { request: Request, problems: String },
    /// The second answer is wrong too: the reading is dropped and the orchestrator says so.
    Dropped { problems: String },
}

/// The verdict on `body`, the first (`attempt` 0) or the second answer of `role` over `input`. A role without a reading schema (the chart drawer) is
/// accepted as JSON and checked by `chart::make`; a body that is not JSON or is too long for the orchestrator counts as a problem.
pub fn check(
    provider: &Provider,
    role: &Role,
    prompt: &str,
    input: &Value,
    body: &str,
    attempt: u8,
    registry: &[Step],
) -> Result<Verdict> {
    let found = match parse(body.trim().as_bytes()) {
        Err(_) => Err(vec![problem("json")]),
        Ok(reading) => {
            let mut o = Out::default();
            value(&mut o, &reading);
            let readable = matches!(role.output_schema.as_str(), "lettura/1" | "param-scelte/1");
            match (readable, o.0.len().div_ceil(3) > MAX_SUMMARY_TOKENS) {
                (_, true) => Err(vec![problem("size")]),
                (true, _) => validate(&reading, input, registry).map(|()| o.0),
                _ => Ok(o.0),
            }
        }
    };
    match (found, attempt) {
        (Ok(summary), _) => Ok(Verdict::Accepted(summary)),
        (Err(p), 0) => {
            let problems = problems_json(&p);
            let mut o = Out::default();
            o.raw("{\"input\":");
            value(&mut o, input);
            o.raw(",\"previous\":")
                .str(body)
                .raw(",\"problems\":")
                .raw(&problems)
                .raw("}");
            let request = subcall::build(
                provider,
                role,
                prompt,
                o.0,
                format(&role.output_schema).as_deref(),
            )
            .map_err(|_| bad("retry"))?;
            Ok(Verdict::Retry { request, problems })
        }
        (Err(p), _) => Ok(Verdict::Dropped {
            problems: problems_json(&p),
        }),
    }
}

fn problem(reason: &str) -> crate::reading::Problem {
    crate::reading::Problem {
        code: format!("err.llm.reading.{reason}"),
        at: "body".to_string(),
    }
}
