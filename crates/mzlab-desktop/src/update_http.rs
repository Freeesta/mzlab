//! The published site over HTTPS as a `Source` of `update.rs` (split off to keep that file small).

use super::{safe_rel, Source, UpdateError, SITE};
use std::io::Read;
use std::time::Duration;

/// The published site over HTTPS: no redirects (the host cannot change), time limits, size limit.
pub struct Http {
    agent: ureq::Agent,
    base: String,
}

impl Http {
    pub fn new() -> Self {
        Self::with_base(SITE)
    }
    pub fn with_base(base: &str) -> Self {
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(60))
            .user_agent("mzlab-desktop")
            .build();
        Http {
            agent,
            base: base.to_string(),
        }
    }
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

/// A version fit for a query string: letters, digits, `.`, `-`, `_` only (it comes from the downloaded manifest).
fn plain_token(v: &str) -> bool {
    !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
}

/// `base + rel`, plus `?v=<version>` when the version is a plain token; any other version gives the plain URL.
pub(super) fn versioned_url(base: &str, rel: &str, version: &str) -> String {
    if plain_token(version) {
        format!("{base}{rel}?v={version}")
    } else {
        format!("{base}{rel}")
    }
}

/// Limit of a response with no declared size (manifest, catalogs).
const MAX_BODY: u64 = 600 << 20;

impl Source for Http {
    fn get(&self, rel: &str) -> Result<Vec<u8>, UpdateError> {
        self.fetch(rel, None, MAX_BODY)
    }
    fn get_versioned(&self, rel: &str, version: &str) -> Result<Vec<u8>, UpdateError> {
        self.fetch(rel, Some(version), MAX_BODY)
    }
    fn get_capped(
        &self,
        rel: &str,
        version: Option<&str>,
        max: u64,
    ) -> Result<Vec<u8>, UpdateError> {
        self.fetch(rel, version, max.min(MAX_BODY))
    }
    fn retry_differs(&self, version: &str) -> bool {
        plain_token(version)
    }
}

impl Http {
    fn fetch(&self, rel: &str, version: Option<&str>, max: u64) -> Result<Vec<u8>, UpdateError> {
        safe_rel(rel)?;
        let url = match version {
            Some(v) => versioned_url(&self.base, rel, v),
            None => format!("{}{}", self.base, rel),
        };
        let resp = self.agent.get(&url).call().map_err(|e| match e {
            ureq::Error::Status(code, _) => UpdateError::new("update.error.http", code.to_string()),
            ureq::Error::Transport(t) => UpdateError::new("update.error.offline", t.to_string()),
        })?;
        if resp.status() != 200 {
            return Err(UpdateError::new(
                "update.error.http",
                resp.status().to_string(),
            )); // with no redirects a 3xx comes back as a response
        }
        let mut body = Vec::new();
        resp.into_reader()
            .take(max)
            .read_to_end(&mut body)
            .map_err(|e| UpdateError::new("update.error.offline", e.to_string()))?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::versioned_url;

    #[test]
    fn a_plain_version_is_a_query_and_anything_else_is_not() {
        let u = |v| versioned_url("https://h/", "static/a.js", v);
        assert_eq!(
            u("57d8ccbb.1_x-y"),
            "https://h/static/a.js?v=57d8ccbb.1_x-y"
        );
        for bad in ["", "a#b", "a&b=c", "a/b", "a\r\nb", "a b", "é"] {
            assert_eq!(u(bad), "https://h/static/a.js", "{bad:?}");
        }
    }
}
