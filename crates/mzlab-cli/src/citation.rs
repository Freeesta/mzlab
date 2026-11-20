//! The citation of mzLab, written ONCE: `--help`, `--version`, `steps --json`, `llm tools` and the `prov.json` of a run all take it from here.
//! When the DOI exists, set `DOI` below and nothing else needs to change (also add it to `CITATION.cff`, which carries a comment on where).

/// The repository the software is cited by.
pub const URL: &str = "https://github.com/Freeesta/mzlab";
/// The DOI, once there is one (never invent it): `Some("10.xxxx/...")`.
pub const DOI: Option<&str> = None;

/// The version of this build.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The full commit this binary was built from (`build.rs`: `MZLAB_COMMIT`, else git); `None` when the build knew neither.
pub fn commit() -> Option<&'static str> {
    Some(env!("MZLAB_CLI_COMMIT")).filter(|c| !c.is_empty())
}

/// The first 7 characters of the commit, as `git rev-parse --short` writes it.
pub fn commit_short() -> Option<&'static str> {
    commit().map(|c| &c[..7.min(c.len())])
}

/// The answer of `--version`: the version and the commit, so a build can be told from another with the same version. There is no build date:
/// a date would make two builds of one commit differ.
pub fn version_line() -> String {
    format!(
        "mzlab-cli {VERSION} (commit {})",
        commit_short().unwrap_or("unknown")
    )
}

/// `--version --json` (`version/1`): the same facts with stable keys; `commit` and `commit_short` are `null` for a build that knew no commit.
pub fn version_json() -> String {
    let mut o = mzlab_core::json::Out::default();
    o.raw("{\"schema\":\"version/1\",\"name\":\"mzlab-cli\",\"version\":")
        .str(VERSION);
    for (key, v) in [("commit", commit()), ("commit_short", commit_short())] {
        o.raw(&format!(",\"{key}\":"));
        match v {
            Some(v) => o.str(v),
            None => o.raw("null"),
        };
    }
    o.raw(",").raw(&member()).raw("}\n");
    o.0
}

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

#[cfg(test)]
mod tests {
    use crate::execute;

    fn run(a: &[&str]) -> String {
        let o = execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(o.code, 0, "{a:?}");
        String::from_utf8(o.stdout).unwrap()
    }

    #[test]
    fn version_names_the_build_in_text_and_in_json() {
        let text = run(&["--version"]);
        let first = text.lines().next().unwrap();
        assert!(
            first.starts_with(&format!("mzlab-cli {} (commit ", super::VERSION)),
            "{first}"
        );
        let v: serde_json::Value = serde_json::from_str(&run(&["--version", "--json"])).unwrap();
        assert_eq!(v["schema"], "version/1");
        assert_eq!(v["name"], "mzlab-cli");
        assert_eq!(v["version"], super::VERSION);
        assert_eq!(v["citation"], super::text());
        // the keys are the ones of the schema, and the commit is the one the line shows
        let schema: serde_json::Value =
            serde_json::from_str(&run(&["schema", "version/1"])).unwrap();
        let mut keys: Vec<&String> = v.as_object().unwrap().keys().collect();
        let mut want: Vec<String> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap().to_string())
            .collect();
        keys.sort();
        want.sort();
        assert_eq!(keys, want.iter().collect::<Vec<_>>());
        match super::commit() {
            Some(c) => {
                assert_eq!(c.len(), 40);
                assert_eq!(v["commit"], c);
                assert_eq!(v["commit_short"], &c[..7]);
                assert!(first.ends_with(&format!("(commit {})", &c[..7])));
            }
            None => {
                assert!(v["commit"].is_null() && v["commit_short"].is_null());
                assert!(first.ends_with("(commit unknown)"));
            }
        }
        // two runs of one binary answer alike: no date, no clock
        assert_eq!(run(&["--version"]), text);
        assert_eq!(run(&["-V", "--json"]), run(&["--version", "--json"]));
    }
}
