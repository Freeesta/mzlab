//! `flow steps --compact` leaves out what is the default (`algorithms` when empty, `executable_native` when true): putting those two keys back gives
//! the same facts as the full listing (`flow steps --json`), step by step.

use crate::execute;
use serde_json::Value;

fn json(args: &[&str]) -> Value {
    let o = execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    assert_eq!(o.code, 0, "{args:?}: {}", o.stderr);
    serde_json::from_slice(&o.stdout).expect("JSON on stdout")
}

#[test]
fn the_omitted_keys_restore_to_the_full_listing() {
    let (compact, full) = (
        json(&["flow", "steps", "--compact"]),
        json(&["flow", "steps", "--json"]),
    );
    let (c, f) = (
        compact["steps"].as_array().unwrap(),
        full.as_array().unwrap(),
    );
    assert_eq!(c.len(), f.len());
    let (mut empty, mut page_only) = (0, 0);
    for (c, f) in c.iter().zip(f) {
        let id = f["id"].as_str().unwrap();
        assert_eq!(c["id"], f["id"], "{id}");
        let algs = c.get("algorithms").cloned().unwrap_or(Value::Array(vec![]));
        // the full descriptor leaves an empty list out too
        let want: Vec<Value> = f.get("algorithms").map_or(vec![], |a| {
            a.as_array()
                .unwrap()
                .iter()
                .map(|a| a["id"].clone())
                .collect()
        });
        assert_eq!(algs, Value::Array(want.clone()), "{id} algorithms");
        // an empty list is never written, so a present key always has an item
        assert!(
            c.get("algorithms")
                .is_none_or(|a| !a.as_array().unwrap().is_empty()),
            "{id}"
        );
        empty += usize::from(want.is_empty());
        let native = c
            .get("executable_native")
            .map_or(true, |v| v.as_bool().unwrap());
        assert_eq!(
            native,
            f["executable_native"].as_bool().unwrap(),
            "{id} executable_native"
        );
        assert!(
            c.get("executable_native")
                .is_none_or(|v| *v == Value::Bool(false)),
            "{id}: a true value is left out"
        );
        page_only += usize::from(!native);
    }
    // the rule is exercised on both sides: steps without algorithms exist, and so do steps only the page runs
    assert!(
        empty > 0 && page_only > 0,
        "empty={empty} page_only={page_only}"
    );
}
