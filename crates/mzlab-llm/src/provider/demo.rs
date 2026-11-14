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

/// The origin (`scheme://host[:port]`, default port dropped, lower case) of an http(s) URL, or `None` for anything else, a user part or a backslash included
/// (`https://api.x.com@evil.org` is `evil.org`, so it is refused rather than parsed).
fn origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    let default = match scheme.as_str() {
        "https" => "443",
        "http" => "80",
        _ => return None,
    };
    let auth = rest.split(['/', '?', '#']).next()?;
    if auth.is_empty() || auth.contains(['@', '\\']) {
        return None;
    }
    let auth = auth.to_ascii_lowercase();
    let (host, port) = auth.rsplit_once(':').unwrap_or((&auth, default));
    (!host.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
        .then(|| format!("{scheme}://{host}:{port}"))
}

/// Whether `url` may carry the key of `p`: its origin is the one of the provider's `base_url`; for the demo provider also the checked proxy host
/// (`proxy_host`, empty before one is known), whose placeholder is the base URL until `for_send`. The page never decides the origin.
pub fn url_belongs(p: &Provider, url: &str, proxy_host: &str) -> bool {
    let Some(o) = origin(url) else { return false };
    if origin(&p.base_url) == Some(o.clone()) {
        return true;
    }
    p.session_headers.is_some() && host_ok(proxy_host) && origin(proxy_host) == Some(o)
}

/// The spend of the session the proxy reports on a 2xx answer (`x-mzlab-spend: <spent>/<cap>` in micro-dollars, `x-mzlab-soft` once past 80 % of
/// the cap) as `{"spent":<µ$>,"cap":<µ$>,"soft":<bool>}`, or `None` for a header that is not two whole numbers with a cap above zero.
pub fn spend_json(header: &str, soft: bool) -> Option<String> {
    let (a, b) = header.trim().split_once('/')?;
    let num = |s: &str| {
        (!s.is_empty() && s.len() <= 15 && s.bytes().all(|c| c.is_ascii_digit()))
            .then(|| s.parse::<u64>().ok())
            .flatten()
    };
    let (spent, cap) = (num(a)?, num(b)?);
    (cap > 0).then(|| format!("{{\"spent\":{spent},\"cap\":{cap},\"soft\":{soft}}}"))
}
