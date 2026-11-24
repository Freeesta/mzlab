//! In the whole `commands --json` listing an argument says `optional`; in the path listing it says `required` (`catalog/doc.rs`, `args_json`).

use crate::cmd::entries;
use crate::execute;

fn json(args: &[&str]) -> serde_json::Value {
    let o = execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    assert_eq!(o.code, 0, "{args:?}: {}", o.stderr);
    serde_json::from_slice(&o.stdout).unwrap()
}

/// The `args` of a command of the listing: the whole listing leaves out an empty list.
fn args_of(c: &serde_json::Value) -> Vec<serde_json::Value> {
    c["args"].as_array().cloned().unwrap_or_default()
}

/// Every argument is `optional` in the whole listing or `required` in the path listing, never both and never neither.
#[test]
fn compact_optional_is_the_full_listings_missing_required_for_every_argument() {
    let doc = json(&["commands", "--json"]);
    let mut n = 0;
    for (e, c) in entries().zip(doc["commands"].as_array().unwrap()) {
        assert_eq!(c["path"], e.name);
        let mut path = vec!["commands", "--json"];
        path.extend(e.name.split(' '));
        let whole = json(&path);
        let whole = whole["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["path"] == e.name)
            .unwrap()
            .clone();
        let (short, long) = (args_of(c), args_of(&whole));
        assert_eq!(short.len(), long.len(), "{}", e.name);
        for (a, b) in short.iter().zip(&long) {
            let (opt, req) = (a.get("optional").is_some(), b.get("required").is_some());
            assert!(opt != req, "{}: {}", e.name, a["name"]);
            assert!(a.get("required").is_none() && b.get("optional").is_none());
            n += 1;
        }
    }
    assert!(n > 50, "only {n} arguments compared");
}
