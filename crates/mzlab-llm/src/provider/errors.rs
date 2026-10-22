//! Failed HTTP answers: the `err.llm.*` event of a status and body, the vendor's own words mapped to a key, and when a request may be tried again.

use mzlab_core::json::{parse, Value};

use crate::event::Event;

/// How many times a 429 that carries `retry-after` is tried again.
pub const MAX_RATE_RETRIES: u32 = 2;

/// Seconds to wait before trying a failed request again, or `None` for no retry: only a 429 with a `retry-after` header and fewer than
/// `MAX_RATE_RETRIES` earlier retries. A 429 without `retry-after` is the spend limit being enforced; trying again would only burn requests.
pub fn retry_delay(status: u16, retry_after_secs: Option<u64>, retries_so_far: u32) -> Option<u64> {
    match (status, retry_after_secs) {
        (429, Some(s)) if retries_so_far < MAX_RATE_RETRIES => Some(s),
        _ => None,
    }
}

/// An error from the status and the body of a failed HTTP answer.
pub fn http_error(status: u16, body: &str) -> Event {
    let (kind, msg) = error_fields(body);
    api_error(status, &kind, &msg)
}

fn error_fields(body: &str) -> (String, String) {
    let Ok(v) = parse(body.as_bytes()) else {
        return (String::new(), body.chars().take(200).collect());
    };
    let e = v.get("error").unwrap_or(&v);
    let kind = ["type", "status", "code"]
        .iter()
        .find_map(|k| e.get(k).and_then(Value::str))
        .unwrap_or("");
    let msg = e
        .get("message")
        .or_else(|| v.get("message"))
        .and_then(Value::str)
        .or(e.str())
        .unwrap_or("");
    (kind.to_string(), msg.chars().take(200).collect())
}

/// Maps the HTTP status (0 inside a stream) and the vendor's own words to an `err.llm.*` event. A message that looks like a key is dropped.
pub(crate) fn api_error(status: u16, kind: &str, msg: &str) -> Event {
    let text = format!("{kind} {msg}").to_lowercase();
    let has = |w: &[&str]| w.iter().any(|x| text.contains(x));
    // The workspace spend limit of the Console: a 400 with these words, or a 429 of kind `enforced_spend_limit_reached`. Never retried.
    let key = if has(&[
        "workspace api usage limits",
        "specified workspace",
        "spend_limit",
    ]) {
        "err.llm.limit.workspace"
    } else if status == 401
        || status == 403
        || has(&[
            "authentication",
            "unauthenticated",
            "permission_denied",
            "api key",
            "api_key",
        ])
    {
        "err.llm.auth"
    } else if status == 429
        || status == 529
        || has(&[
            "rate_limit",
            "rate limit",
            "overloaded",
            "resource_exhausted",
            "quota",
        ])
    {
        "err.llm.rate"
    } else if status == 413
        || has(&[
            "context length",
            "context window",
            "too long",
            "too many tokens",
            "token limit",
            "maximum context",
        ])
    {
        "err.llm.context"
    } else if status == 408 || status >= 500 {
        "err.llm.network"
    } else {
        "err.llm.bad_response"
    };
    let mut params = Vec::new();
    if status != 0 {
        params.push(("status", status.to_string()));
    }
    if !msg.is_empty() && !crate::log::has_secret(msg) {
        params.push(("message", msg.to_string()));
    }
    Event::error(key, &params)
}
