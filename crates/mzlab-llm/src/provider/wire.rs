//! `Request` read back from the JSON that `Request::to_json` writes: the calls of the subagents are built by Rust, handed to the page and sent by a transport that
//! only has the request (`llm/send` of the app). Used by `mzlab-desktop` and `mzlab-llm-wasm`.
use mzlab_core::error::Result;
use mzlab_core::json::Value;

use super::{bad, AuthSlot, Request};

impl Request {
    /// The inverse of `to_json`: a request the page got back from Rust and hands to a transport that only has the request (the calls of the subagents).
    pub fn from_value(v: &Value) -> Result<Request> {
        let text = |k: &str| {
            v.get(k)
                .and_then(Value::str)
                .map(str::to_string)
                .ok_or_else(|| bad(k))
        };
        let mut headers = Vec::new();
        for h in v.get("headers").map_or(&[][..], Value::arr) {
            let (k, x) = (
                h.arr().first().and_then(Value::str),
                h.arr().get(1).and_then(Value::str),
            );
            headers.push((
                k.ok_or_else(|| bad("headers"))?.to_string(),
                x.ok_or_else(|| bad("headers"))?.to_string(),
            ));
        }
        let auth = v.get("auth").ok_or_else(|| bad("auth"))?;
        let slot = |k: &str| {
            auth.get(k)
                .and_then(Value::str)
                .map(str::to_string)
                .ok_or_else(|| bad(k))
        };
        Ok(Request {
            url: text("url")?,
            headers,
            body: text("body")?,
            auth: AuthSlot {
                header: slot("header")?,
                prefix: slot("prefix")?,
            },
        })
    }
}
