//! The citation of mzLab, written ONCE: `--help`, `--version`, `steps --json`, `llm tools` and the `prov.json` of a run all take it from here.
//! When the DOI exists, set `DOI` below and nothing else needs to change (also add it to `CITATION.cff`, which carries a comment on where).

/// The repository the software is cited by.
pub const URL: &str = "https://github.com/Freeesta/mzlab";
/// The DOI, once there is one (never invent it): `Some("10.xxxx/...")`.
pub const DOI: Option<&str> = None;

/// The version of this build.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `mzLab 1.2.3, https://github.com/Freeesta/mzlab, DOI: to be added`.
pub fn text() -> String {
    format!(
        "mzLab {VERSION}, {URL}, DOI: {}",
        DOI.unwrap_or("to be added")
    )
}

/// The reminder line of `--help` and `--version`.
pub fn reminder() -> String {
    format!("If you use mzLab, please cite it: {}", text())
}

/// `"citation":"..."` as a JSON member (no leading comma).
pub fn member() -> String {
    let mut o = mzlab_core::json::Out::default();
    o.raw("\"citation\":").str(&text());
    o.0
}

/// `json` with the `"citation"` member inserted right after the first `anchor` (which must end with a comma), for the CLI-only outputs;
/// `json` unchanged when the anchor is missing.
pub fn after(json: &str, anchor: &str, indent: &str) -> String {
    match json.split_once(anchor) {
        Some((head, tail)) => format!("{head}{anchor}{indent}{},{tail}", member()),
        None => json.to_string(),
    }
}
