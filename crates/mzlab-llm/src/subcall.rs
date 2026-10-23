//! The call of a subagent as a request: the prompt of its role as system text (cached 5 minutes), the data as one JSON user message (never part of the
//! prompt), the answer forced to a closed schema when the role has one. Shared by the router and the readers (`subcheck`).

use mzlab_core::error::{Error, Result};

use crate::assist::Role;
use crate::conv::{Conversation, Message, Part, Role as Speaker};
use crate::provider::{request_with, CacheTtl, Effort, Options, Provider, Request};

/// The request of `role` for `input` (JSON text); `schema` is the closed schema of its answer (`output_config.format`).
pub(crate) fn build(
    provider: &Provider,
    role: &Role,
    prompt: &str,
    input: String,
    schema: Option<&str>,
) -> Result<Request> {
    let conv = Conversation {
        provider: provider.id.clone(),
        model: role.model.clone(),
        system: prompt.to_string(),
        max_output_tokens: Some(role.max_tokens),
        messages: vec![Message {
            role: Speaker::User,
            parts: vec![Part::Text(input)],
        }],
    };
    let effort = Effort::parse(&role.effort)
        .ok_or_else(|| Error::new("err.llm.assist.roles").with("field", "effort"))?;
    let opts = Options {
        cache_ttl: Some(CacheTtl::Min5),
        effort: Some(effort),
        format_schema: schema.map(String::from),
        ..Options::default()
    };
    request_with(provider, &conv, &[], &opts)
}
