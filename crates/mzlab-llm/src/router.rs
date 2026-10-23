//! The router of the data assistant: a small model that, before every user message, picks the subagent and the program data to attach (`need_data`). It never
//! answers the user and never acts; a wrong route costs one call too many. `request` builds its call from the role of `assist-ruoli-1.json`, `parse` reads
//! its answer against the closed schema. Used by `agent`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse as parse_json, Out, Value};

use crate::assist::Role;
use crate::conv::{Conversation, Message, Part, Role as Speaker};
use crate::provider::{request_with, CacheTtl, Effort, Options, Provider, Request};
use crate::schema::validate;
use crate::tools::value;

/// The schema of the answer (`output_config.format`), the one of `prompt/02`.
const SCHEMA: &str = r#"{"type":"object","additionalProperties":false,"required":["route","need_data","language","suspicious"],"properties":{"route":{"enum":["inspect","chromatography","params","plan","table","chart","answer","chat"]},"need_data":{"type":"array","items":{"enum":["inspection","chromatograms","proposals","plan","tabcheck","rows","views"]}},"language":{"type":"string","pattern":"^[a-z]{2}$"},"suspicious":{"type":"boolean"}}}"#;

/// What the router decided for one user message: an object that satisfies `SCHEMA` (`route`, `need_data`, `language`, `suspicious`).
#[derive(Debug, Clone, PartialEq)]
pub struct Route(pub Value);

impl Route {
    pub fn route(&self) -> &str {
        self.0.get("route").and_then(Value::str).unwrap_or("")
    }

    pub fn need_data(&self) -> Vec<&str> {
        self.0
            .get("need_data")
            .map_or(&[][..], Value::arr)
            .iter()
            .filter_map(Value::str)
            .collect()
    }

    pub fn suspicious(&self) -> bool {
        self.0.get("suspicious").and_then(Value::bool) == Some(true)
    }
}

fn bad(field: &str) -> Error {
    Error::new("err.llm.router").with("field", field)
}

/// The call of the router: its prompt as system text (cached 5 minutes), the state and the message as one JSON object (the message is data, never text of
/// the prompt), the answer forced to `SCHEMA`.
pub fn request(
    provider: &Provider,
    role: &Role,
    prompt: &str,
    state: &Value,
    message: &str,
) -> Result<Request> {
    let mut input = Out::default();
    input.raw("{\"message\":").str(message).raw(",\"state\":");
    value(&mut input, state);
    input.raw("}");
    let conv = Conversation {
        provider: provider.id.clone(),
        model: role.model.clone(),
        system: prompt.to_string(),
        max_output_tokens: Some(role.max_tokens),
        messages: vec![Message {
            role: Speaker::User,
            parts: vec![Part::Text(input.0)],
        }],
    };
    let opts = Options {
        cache_ttl: Some(CacheTtl::Min5),
        effort: Some(Effort::parse(&role.effort).ok_or_else(|| bad("effort"))?),
        format_schema: Some(SCHEMA.to_string()),
        ..Options::default()
    };
    request_with(provider, &conv, &[], &opts)
}

/// The route of an answer; anything outside the schema is `err.llm.router` (the caller then goes on without a route).
pub fn parse(body: &str) -> Result<Route> {
    let v = parse_json(body.trim().as_bytes()).map_err(|_| bad("json"))?;
    let schema = parse_json(SCHEMA.as_bytes()).map_err(|_| bad("schema"))?;
    validate(&schema, &v).map_err(|_| bad("schema"))?;
    let lang = v.get("language").and_then(Value::str).unwrap_or("");
    match lang.len() == 2 && lang.bytes().all(|b| b.is_ascii_lowercase()) {
        true => Ok(Route(v)),
        false => Err(bad("language")),
    }
}
