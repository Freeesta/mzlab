//! The provider of the public demo (`mzlab-demo`, `data/fornitori-1.json`): the requests go to mzLab's proxy without a key. Its base URL is the
//! placeholder `PLACEHOLDER`; `for_send` puts the host of `mzlab/web/llm/proxy-host.js` in its place and adds the session header (random per opening
//! of the panel) and, for the owner only, the passphrase header. The body is never touched: the app and the site send the same bytes.

use mzlab_core::error::{Error, Result};

use super::{Provider, Request};

/// The base URL of the demo provider in `fornitori-1.json` until a host is given.
pub const PLACEHOLDER: &str = "https://proxy.invalid";

/// A session id as the proxy accepts it: `[A-Za-z0-9_-]{8,64}`.
pub fn session_ok(s: &str) -> bool {
    (8..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// A proxy host: `https://` and a host name (an optional port), no path, query or user.
pub fn host_ok(h: &str) -> bool {
    let Some(rest) = h.strip_prefix("https://") else {
        return false;
    };
    let (name, port) = rest.split_once(':').unwrap_or((rest, "443"));
    !name.is_empty()
        && name.len() <= 253
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && !name.starts_with(['.', '-'])
        && !port.is_empty()
        && port.len() <= 5
        && port.bytes().all(|b| b.is_ascii_digit())
}

/// The request of the demo provider as it leaves: the proxy host instead of the placeholder, `session` and (when not empty) `owner` in the
/// headers the descriptor names. Another provider's request comes back unchanged. No host yet → `err.llm.network` (`reason=proxy`).
pub fn for_send(
    p: &Provider,
    mut req: Request,
    host: &str,
    session: &str,
    owner: &str,
) -> Result<Request> {
    let Some((session_header, owner_header)) = &p.session_headers else {
        return Ok(req);
    };
    if !host_ok(host) {
        return Err(Error::new("err.llm.network").with("reason", "proxy"));
    }
    if !session_ok(session) {
        return Err(Error::new("err.llm.conversation").with("field", "session"));
    }
    if owner.len() > 256 || owner.chars().any(char::is_control) {
        return Err(Error::new("err.llm.conversation").with("field", "owner"));
    }
    if let Some(path) = req.url.strip_prefix(PLACEHOLDER) {
        req.url = format!("{host}{path}");
    }
    req.headers
        .push((session_header.clone(), session.to_string()));
    if !owner.is_empty() {
        req.headers.push((owner_header.clone(), owner.to_string()));
    }
    Ok(req)
}
