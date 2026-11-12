//! `structure.moiety` on the synthetic table of `tests/dati/moiety.json`: the rows kept, their order, the cells written back and the errors.

use super::moiety_text;
use mzlab_core::json::{self, Value};

fn fixture() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/dati/moiety.json");
    json::parse(&std::fs::read(p).unwrap()).unwrap()
}

fn params(c: &Value) -> Value {
    json::parse(c.get("params").and_then(|p| p.str()).unwrap().as_bytes()).unwrap()
}

#[test]
fn the_fixture_gives_its_tables() {
    let f = fixture();
    let table = f.get("table").and_then(|t| t.str()).unwrap();
    for c in f.get("cases").unwrap().arr() {
        let got = moiety_text(&params(c), table.as_bytes()).unwrap();
        assert_eq!(
            got,
            c.get("out").and_then(|o| o.str()).unwrap(),
            "{:?}",
            params(c)
        );
    }
}

#[test]
fn wrong_requests_give_their_key() {
    let f = fixture();
    let table = f.get("table").and_then(|t| t.str()).unwrap();
    for c in f.get("errors").unwrap().arr() {
        let e = moiety_text(&params(c), table.as_bytes()).unwrap_err();
        assert_eq!(
            e.key,
            c.get("key").and_then(|k| k.str()).unwrap(),
            "{:?}",
            params(c)
        );
    }
    assert_eq!(
        moiety_text(&json::parse(br#"{"query":"CO"}"#).unwrap(), b"")
            .unwrap_err()
            .key,
        "error.sub.db"
    );
}
