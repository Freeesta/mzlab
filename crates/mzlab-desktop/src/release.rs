//! Notice of a newer desktop release: asks GitHub for the latest published release of the project and compares its tag with the
//! version of this binary. The request carries nothing of the user (no cookies, no identifiers) and any failure is just «no news».

use std::io::Read;
use std::time::Duration;

use serde::Deserialize;

/// Public API: the latest release that is neither a draft nor a prerelease.
pub const API: &str = "https://api.github.com/repos/Freeesta/mzlab/releases/latest";
/// Page where the installers of the latest release are found.
pub const PAGE: &str = "https://github.com/Freeesta/mzlab/releases/latest";

#[derive(Deserialize)]
struct Latest {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// `v1.2.3` or `1.2.3` → `[1, 2, 3]`; anything else (suffix such as `-rc1`, missing or non-numeric part) → `None`.
pub fn parse_version(s: &str) -> Option<[u64; 3]> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let mut it = s.split('.');
    let mut out = [0u64; 3];
    for slot in out.iter_mut() {
        let part = it.next()?;
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        *slot = part.parse().ok()?;
    }
    if it.next().is_some() {
        return None;
    }
    Some(out)
}

/// The version to announce: the tag of the release if it is a stable one and strictly newer than `current`.
pub fn newer_tag(body: &[u8], current: &str) -> Option<String> {
    let r: Latest = serde_json::from_slice(body).ok()?;
    if r.draft || r.prerelease {
        return None;
    }
    let tag = parse_version(&r.tag_name)?;
    let cur = parse_version(current)?;
    (tag > cur).then(|| r.tag_name.trim().trim_start_matches(['v', 'V']).to_string())
}

/// One short request (5 s in all, no redirects, 256 KB at most); `None` on any error.
pub fn fetch_latest(url: &str) -> Option<Vec<u8>> {
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(Duration::from_secs(5))
        .user_agent("mzlab-desktop")
        .build();
    let resp = agent
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .ok()?;
    if resp.status() != 200 {
        return None;
    }
    let mut body = Vec::new();
    resp.into_reader()
        .take(256 << 10)
        .read_to_end(&mut body)
        .ok()?;
    Some(body)
}

/// JSON for the page: `{"status":"newer","version":"X","url":PAGE}` or `{"status":"none"}`.
pub fn check(url: &str, current: &str) -> String {
    match fetch_latest(url).and_then(|b| newer_tag(&b, current)) {
        Some(v) => serde_json::json!({ "status": "newer", "version": v, "url": PAGE }).to_string(),
        None => r#"{"status":"none"}"#.to_string(),
    }
}
