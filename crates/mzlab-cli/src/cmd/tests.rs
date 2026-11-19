//! The help of every command is the text the program printed before the command table existed (`tests/data/help/help_snapshot.txt`, taken from main), and the table is consistent.

use crate::execute;

const SNAPSHOT: &str = include_str!("../../tests/data/help/help_snapshot.txt");

#[test]
fn every_help_and_usage_error_is_unchanged() {
    let mut n = 0;
    let mut fresh = String::new();
    for chunk in SNAPSHOT.split("=== mzlab-cli ").skip(1) {
        let (line, _) = chunk.split_once('\n').unwrap();
        let args: Vec<String> = line.split_whitespace().map(String::from).collect();
        let o = execute(&args);
        let got = format!(
            "{line}\nexit {}\n--- stdout\n{}--- stderr\n{}\n",
            o.code,
            String::from_utf8_lossy(&o.stdout),
            o.stderr
        );
        // the version is the build's: the snapshot holds a placeholder
        let want = chunk.replace("{VERSION}", env!("CARGO_PKG_VERSION"));
        fresh += &format!(
            "=== mzlab-cli {}",
            got.replace(env!("CARGO_PKG_VERSION"), "{VERSION}")
        );
        if std::env::var_os("MZLAB_UPDATE_HELP").is_none() {
            assert_eq!(got, want, "mzlab-cli {line}");
        }
        n += 1;
    }
    // MZLAB_UPDATE_HELP=1 rewrites the snapshot from the current output (review the diff before committing)
    if std::env::var_os("MZLAB_UPDATE_HELP").is_some() {
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/data/help/help_snapshot.txt"
            ),
            fresh,
        )
        .unwrap();
    }
    assert!(n >= 60, "the snapshot lists {n} invocations");
}

#[test]
fn names_are_unique_and_aliases_point_to_a_command() {
    let mut seen = std::collections::HashSet::new();
    for e in super::entries() {
        assert!(seen.insert(e.name), "{} twice", e.name);
        for a in e.aliases {
            assert_eq!(super::canonical(a), e.name);
            assert!(!seen.contains(a), "alias {a} is also a name");
        }
    }
}

/// The block of `docs/agenti/cli.md` between the two marker lines is what `mzlab-cli help-md` prints (`python3 tools/cli_doc.py` rewrites it).
#[test]
fn the_command_list_of_the_docs_is_current() {
    const DOC: &str = include_str!("../../../../docs/agenti/cli.md");
    let (_, rest) = DOC.split_once("<!--cli-->\n").unwrap();
    let (block, _) = rest.split_once("<!--/cli-->").unwrap();
    assert_eq!(block, super::list_md(), "run python3 tools/cli_doc.py");
}

#[test]
fn the_citation_is_in_help_version_catalogs_and_the_run_record() {
    let text = crate::citation::text();
    assert!(
        text.contains(env!("CARGO_PKG_VERSION"))
            && text.contains("https://github.com/Freeesta/mzlab")
    );
    assert!(
        text.ends_with("DOI: to be added"),
        "no DOI is invented: {text}"
    );
    let out = |a: &[&str]| {
        let o = execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(o.code, 0, "{a:?}");
        String::from_utf8(o.stdout).unwrap()
    };
    for a in [&["--help"][..], &["-h"], &["--version"], &["-V"]] {
        assert!(out(a).contains(&format!("please cite it: {text}")), "{a:?}");
    }
    // machine-readable catalogs carry it as a "citation" field and stay valid JSON
    let tools: serde_json::Value = serde_json::from_str(&out(&["llm", "tools"])).unwrap();
    assert_eq!(tools["citation"], text);
    assert_eq!(tools["schema"], "tools/1");
    let steps: serde_json::Value = serde_json::from_str(&out(&["steps", "--json"])).unwrap();
    assert!(steps
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["citation"] == text));
    let one: serde_json::Value =
        serde_json::from_str(&out(&["steps", "--json", "--id", "features.detect"])).unwrap();
    assert_eq!(one["citation"], text);
    // data stays untouched: plain `steps` has no citation line
    assert!(!out(&["steps"]).contains("cite"));
}
