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

impl Source for Http {
    fn get(&self, rel: &str) -> Result<Vec<u8>, UpdateError> {
        self.fetch(rel, None)
    }
    fn get_versioned(&self, rel: &str, version: &str) -> Result<Vec<u8>, UpdateError> {
        self.fetch(rel, Some(version))
    }
}

impl Http {
    fn fetch(&self, rel: &str, version: Option<&str>) -> Result<Vec<u8>, UpdateError> {
        safe_rel(rel)?;
        let mut url = format!("{}{}", self.base, rel);
        // only a plain token goes into the query: the version comes from the downloaded manifest
        if let Some(v) = version.filter(|v| {
            !v.is_empty()
                && v.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        }) {
            url.push_str("?v=");
            url.push_str(v);
        }
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
            .take(600 << 20)
            .read_to_end(&mut body)
            .map_err(|e| UpdateError::new("update.error.offline", e.to_string()))?;
        Ok(body)
    }
}
