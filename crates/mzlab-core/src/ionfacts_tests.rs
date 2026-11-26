//! `Facts::from_method` and `read_json`: the dwell of SIM/MRM transitions is read, the other scan types read nothing, differing values give none.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;

fn doc(exps: &str) -> Value {
    parse(
        format!(r#"{{"schema":"method/1","ms":{{"periods":[{{"experiments":[{exps}]}}]}}}}"#)
            .as_bytes(),
    )
    .unwrap()
}

const SIM: &str = r#"{"index":1,"type":"MRM","transitions":[{"q1":241.3,"q3":100.1,"dwell_ms":5},{"q1":300.2,"q3":90,"dwell_ms":5}]}"#;

#[test]
fn equal_dwells_are_read_with_their_experiment() {
    let (f, r) = Facts::from_method(&doc(SIM), Some(ScanKind::Sim));
    assert_eq!(
        f,
        Facts {
            dwell_ms: Some(5.0),
            ..Facts::default()
        }
    );
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].field, "dwell_ms");
    assert!(
        r[0].from.contains("ms.periods[0].experiments[0]"),
        "{}",
        r[0].from
    );
    assert_eq!(f.query(), "dwell_ms=5");
}

#[test]
fn every_scan_type_asked_reads_only_what_it_uses() {
    // a trap scan and a Q1 scan need labels the method does not name yet: nothing is read, nothing guessed
    for k in [ScanKind::Ems, ScanKind::Epi, ScanKind::Q1, ScanKind::Other] {
        let (f, r) = Facts::from_method(&doc(SIM), Some(k));
        assert_eq!((f, r.len()), (Facts::default(), 0), "{k:?}");
    }
    let (f, _) = Facts::from_method(&doc(SIM), None);
    assert_eq!(f.dwell_ms, Some(5.0));
}

#[test]
fn different_dwells_give_none_and_say_where() {
    let two = format!(r#"{SIM},{{"index":2,"transitions":[{{"q1":1,"dwell_ms":10.5}}]}}"#);
    let (f, r) = Facts::from_method(&doc(&two), Some(ScanKind::Sim));
    assert_eq!(f.dwell_ms, None);
    assert_eq!(r[0].value, None);
    assert!(
        r[0].from.contains("5 ms in ms.periods[0].experiments[0]"),
        "{}",
        r[0].from
    );
    assert!(
        r[0].from
            .contains("10.5 ms in ms.periods[0].experiments[1]"),
        "{}",
        r[0].from
    );
}

#[test]
fn a_method_without_a_dwell_reads_nothing() {
    for d in [
        doc(r#"{"index":1,"parameters":[]}"#),
        doc(r#"{"transitions":[{"q1":1,"dwell_ms":0}]}"#),
        parse(b"{}").unwrap(),
    ] {
        let (f, r) = Facts::from_method(&d, None);
        assert_eq!((f, r.len()), (Facts::default(), 0));
    }
}

#[test]
fn the_document_is_stable_and_refuses_what_it_cannot_read() {
    let m = format!(r#"{{"ms":{{"periods":[{{"experiments":[{SIM}]}}]}}}}"#);
    let j = read_json(&m, "sim").unwrap();
    let v = parse(j.as_bytes()).unwrap();
    assert_eq!(v.get("query").unwrap().str(), Some("dwell_ms=5"));
    let r = &v.get("read").unwrap().arr()[0];
    assert_eq!(r.get("value").unwrap().f64(), Some(5.0));
    assert_eq!(read_json("", "").unwrap(), r#"{"query":"","read":[]}"#);
    assert_eq!(read_json(&m, "ems").unwrap(), r#"{"query":"","read":[]}"#);
    assert_eq!(read_json(&m, "ms").unwrap_err().key, "err.raw.xic.param");
    assert!(read_json("{", "").is_err());
}
