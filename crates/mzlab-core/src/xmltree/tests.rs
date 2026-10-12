#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn nesting_up_to_the_limit_parses_and_deeper_is_refused() {
    let doc = |n: usize| format!("{}x{}", "<a>".repeat(n), "</a>".repeat(n));
    assert!(parse(&doc(MAX_DEPTH)).is_some());
    assert!(parse(&doc(MAX_DEPTH + 1)).is_none());
    // 10 000 levels would exhaust the stack of the WASM if they were followed
    assert!(parse(&doc(10_000)).is_none());
}

#[test]
fn a_method_xml_still_reads() {
    let r = parse("<?xml version=\"1.0\"?><m><p><n>3</n></p></m>").unwrap();
    assert_eq!(r.find("p").and_then(|p| p.findtext("n")), Some("3"));
}
