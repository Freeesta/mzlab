//! The updated folder answers a request of the window only for a path that cannot reach a native file or leave the folder: `override_rel` is the
//! pure rule (tested with every spelling), `serve_override` adds the file-system checks (no symlink, nothing outside `web/`).

use super::*;

/// The path of a request as it may be looked up in the updated folder, or `None` when the embedded copy must answer: an unsafe path, the
/// native files (`NATIVE`) and `check.json` under any spelling the file system could fold onto them (`//`, `/./`, a trailing `/`, case, and on
/// Windows a trailing dot or space and 8.3 short names with `~`), whatever the platform the build runs on.
pub(super) fn override_rel(req_path: &str) -> Option<String> {
    let rel = req_path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    safe_rel(rel).ok()?;
    let parts: Vec<&str> = Path::new(rel)
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    if parts.is_empty()
        || parts
            .iter()
            .any(|c| c.ends_with(['.', ' ']) || c.contains('~'))
    {
        return None;
    }
    let rel = parts.join("/");
    let native =
        rel.eq_ignore_ascii_case(CHECK) || NATIVE.iter().any(|n| rel.eq_ignore_ascii_case(n));
    (!native).then_some(rel)
}

/// The file of the updated folder for a request path of the window (`/` = the page), if it is there. Anything else (and any path that
/// tries to leave the folder, or a symlink) returns `None` and the embedded copy answers.
pub fn serve_override(web: &Path, req_path: &str) -> Option<(Vec<u8>, &'static str)> {
    let rel = override_rel(req_path)?;
    let p: PathBuf = web.join(&rel);
    if !fs::symlink_metadata(&p).ok()?.is_file()
        || !p.canonicalize().ok()?.starts_with(web.canonicalize().ok()?)
    {
        return None;
    }
    let body = fs::read(&p).ok()?;
    Some((body, mime(&rel)))
}

#[cfg(test)]
#[path = "update_serve_tests.rs"]
mod tests;
