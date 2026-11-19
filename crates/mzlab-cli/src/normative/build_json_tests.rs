//! `db normative build --json`: the `normative-build/1` document carries the lists, the sources with their pins and the stale lists.

use super::tests::{Fake, Work};
use super::*;

/// `--json`: one `normative-build/1` document with the lists, the sources and their pins; a stale list is exit 1 and the document stays on stdout.
#[test]
fn normative_build_json_carries_sources_pins_and_stale_lists() {
    let w = Work::new("json");
    let r = run_with(w.args(&["--update-pins", "--json"]), Fake::standard());
    assert_eq!(r.code, 0, "{}", r.stderr);
    let text = String::from_utf8(r.stdout).unwrap();
    assert!(text.starts_with("{\"schema\":\"normative-build/1\"") && text.ends_with("}\n"));
    let v = mzlab_core::json::parse(text.as_bytes()).unwrap();
    assert_eq!(v.get("pins_updated").and_then(|x| x.bool()), Some(true));
    let lists: Vec<&str> = v
        .get("lists")
        .unwrap()
        .arr()
        .iter()
        .map(|l| l.get("id").unwrap().str().unwrap())
        .collect();
    assert!(lists.contains(&"T_EU_WATER"));
    let sources = v.get("sources").unwrap().arr();
    assert!(!sources.is_empty());
    for s in sources {
        assert_eq!(s.get("sha256").unwrap().str().map(str::len), Some(64));
    }
    // a changed source without --update-pins: the pin check fails before anything is written (exit 3, no document)
    let again = run_with(w.args(&["--json"]), Fake::standard());
    assert_eq!(again.code, 0, "{}", again.stderr);
    let v = mzlab_core::json::parse(&again.stdout).unwrap();
    assert!(v
        .get("sources")
        .unwrap()
        .arr()
        .iter()
        .all(|s| s.get("pin_ok").and_then(|x| x.bool()) != Some(false)));
}
