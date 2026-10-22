//! The per-role knobs of an Anthropic request: cache breakpoints, effort, thinking, strict tools and the answer schema.

/// Lifetime of an explicit cache breakpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheTtl {
    Min5,
    Hour1,
}

/// How hard the model thinks (`output_config.effort`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effort {
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl Effort {
    pub fn name(self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::XHigh => "xhigh",
            Effort::Max => "max",
        }
    }
}

/// The per-role knobs of a request. Only the `anthropic` adapter reads them; the default asks for nothing, so `request` keeps the plain body.
/// `temperature`, `top_p` and `top_k` are not here on purpose: they are never sent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Options {
    /// An explicit cache breakpoint with this lifetime on the system block (after the tools).
    pub cache_ttl: Option<CacheTtl>,
    /// The automatic breakpoint at the top of the request (5 minutes), for the tail of the conversation.
    pub cache_auto: bool,
    pub effort: Option<Effort>,
    /// `thinking: {type: adaptive}`.
    pub adaptive_thinking: bool,
    /// `strict: true` on every tool (the schemas must then be closed with `additionalProperties: false`).
    pub strict_tools: bool,
    /// JSON Schema text of the answer (`output_config.format`); only for a call without tools.
    pub format_schema: Option<String>,
}
