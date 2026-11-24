//! The rule of `flag_help_note` on flags (`catalog/doc.rs`, `implied`): the compact `commands --json` leaves `--json` and an optional `--out` out of
//! `flags`, the synopsis shows them; `flags_of` puts them back, and putting them back gives the flags of the full listing.

use std::collections::BTreeSet;

/// The flags of a compact command with the ones `flag_help_note` leaves out (`--json`, an optional `--out`) put back from the synopsis, in its order.
pub(crate) fn flags_of(c: &serde_json::Value) -> Vec<serde_json::Value> {
    let listed = c["flags"].as_array().cloned().unwrap_or_default();
    let mut seen = BTreeSet::new();
    let words = c["synopsis"]
        .as_str()
        .unwrap()
        .split(|ch: char| ch.is_whitespace() || "[]|()".contains(ch));
    let flags: Vec<serde_json::Value> = words
        .filter(|w| w.starts_with("--") && seen.insert(w.to_string()))
        .map(|w| match listed.iter().find(|f| f["name"] == w) {
            Some(f) => f.clone(),
            None => {
                assert!(
                    w == "--json" || w == "--out",
                    "{}: {w} is in the synopsis only",
                    c["path"]
                );
                serde_json::json!({ "name": w })
            }
        })
        .collect();
    assert!(
        listed.iter().all(|f| f["name"] != "--json"
                // a bare `--out` is listed only when it is required (written outside brackets, so `required` is read from the synopsis)
                && (f != &serde_json::json!({"name": "--out"})
                    || crate::catalog_restore_tests::is_required(c, "--out"))),
        "{}",
        c["path"]
    );
    flags
}

#[test]
fn the_flags_put_back_from_the_synopsis_are_the_full_ones() {
    let run = |a: &[&str]| -> serde_json::Value {
        let o = crate::execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        serde_json::from_slice(&o.stdout).unwrap()
    };
    let doc = run(&["commands", "--json"]);
    let mut put_back = 0;
    for c in doc["commands"].as_array().unwrap() {
        let mut path = vec!["commands", "--json"];
        path.extend(c["path"].as_str().unwrap().split(' '));
        let whole = run(&path);
        let names =
            |v: &[serde_json::Value]| v.iter().map(|f| f["name"].clone()).collect::<Vec<_>>();
        let full = whole["commands"][0]["flags"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let back = flags_of(c);
        assert_eq!(names(&back), names(&full), "{}", c["path"]);
        put_back += back.len() - c["flags"].as_array().map_or(0, Vec::len);
    }
    assert!(put_back > 30, "only {put_back} flags left out");
}
