//! The columns that tie a row to its sample (`sample_id`, `file`) even when samples without channels sit among the others, and `max_shift`: a followed
//! window that would move farther than allowed stays put (`too_far`) with the refused shift written in `shift_min` and in `found_shift`.

use super::tests::{rows_with_time, three};
use super::*;

const HEAD_TAIL: [&str; 3] = ["sample_id", "file", "shift_min"];

fn col(t: &str, name: &str) -> usize {
    t.lines()
        .next()
        .unwrap()
        .split(',')
        .position(|c| c == name)
        .unwrap_or_else(|| panic!("no column {name}"))
}

#[test]
fn the_header_ends_with_the_ids_after_the_origin() {
    let h: Vec<&str> = HEAD.trim_end().split(',').collect();
    assert_eq!(
        &h[h.len() - 4..],
        ["origin", HEAD_TAIL[0], HEAD_TAIL[1], HEAD_TAIL[2]]
    );
}

/// `sample_id` is the place the window names in `file`, empty samples counted: the row of the third sample says 2 although only two samples have rows.
#[test]
fn sample_id_counts_the_samples_without_a_channel_and_file_is_the_name_given() {
    let f = |name: &str, tr: &str| format!("{{\"name\":\"{name}\",\"transitions\":[{tr}]}}");
    let one = super::tests::trace(Some(180.1), None, "", &[(3.0, 8e4)]);
    let files = format!(
        "[{},{},{}]",
        f("blank.mzML", ""),
        f("a.wiff.mzML", &one),
        f("b.mzML", &one)
    );
    let w = r#"[{"file":2,"tr":0,"a":2.8,"b":3.2},{"file":1,"tr":0,"a":2.8,"b":3.2}]"#;
    let t = table_text(&files, w, false).unwrap();
    let (i, n) = (col(&t, "sample_id"), col(&t, "file"));
    let r = rows_with_time(&t);
    let got: Vec<(&str, &str, &str)> = r
        .iter()
        .map(|x| (x[1].as_str(), x[i].as_str(), x[n].as_str()))
        .collect();
    assert_eq!(
        got,
        [("a.wiff", "1", "a.wiff.mzML"), ("b", "2", "b.mzML")],
        "{t}"
    );
}

fn follow(extra: &str) -> String {
    format!(r#"[{{"file":0,"tr":0,"a":2.8,"b":3.2,"follow":true{extra}}}]"#)
}

/// (origin, shift_min, start_min, end_min) of the row of the sample `t3h`, which lies 0.30 min from the source.
fn drifted(t: &str) -> (String, String, String, String) {
    let r = rows_with_time(t);
    let x = r.iter().find(|x| x[1] == "t3h").unwrap();
    let c = |n: &str| x[col(t, n)].clone();
    (c("origin"), c("shift_min"), c("start_min"), c("end_min"))
}

#[test]
fn a_move_longer_than_max_shift_is_refused_and_the_shift_is_the_evidence() {
    let moved = |w: &str| drifted(&table_text(&three(), w, false).unwrap());
    let free = moved(&follow(""));
    assert_eq!(
        (free.0.as_str(), free.1.as_str()),
        ("shifted", "0.307"),
        "{free:?}"
    );
    assert_eq!(moved(&follow(r#","max_shift":0.5"#)).0, "shifted");
    for limit in ["0.2", "0"] {
        let m = moved(&follow(&format!(r#","max_shift":{limit}"#)));
        assert_eq!(
            (m.0.as_str(), m.1.as_str(), m.2.as_str(), m.3.as_str()),
            ("too_far", "0.307", "2.800", "3.200"),
            "limit {limit}: the window stays, the refused shift is written"
        );
    }
}

#[test]
fn the_default_of_the_document_applies_to_a_window_without_its_own() {
    let doc =
        |limit: &str, own: &str| format!(r#"{{"windows":{},"max_shift":{limit}}}"#, follow(own));
    let m = |w: &str| drifted(&table_text(&three(), w, false).unwrap()).0;
    assert_eq!(m(&doc("0.2", "")), "too_far");
    assert_eq!(
        m(&doc("0.2", r#","max_shift":0.5"#)),
        "shifted",
        "the window's own wins"
    );
    assert_eq!(m(&doc("0.5", r#","max_shift":0.1"#)), "too_far");
}

#[test]
fn a_bad_limit_is_an_error_never_ignored() {
    for bad in ["-1", "\"x\"", "true"] {
        let e =
            table_text(&three(), &follow(&format!(r#","max_shift":{bad}"#)), false).unwrap_err();
        assert_eq!(e.key, "err.manint.maxShift", "{bad}");
    }
}

#[test]
fn the_shifts_document_carries_the_found_shift_and_the_refusal() {
    let w = |extra: &str| format!(r#"{{"file":0,"tr":0,"a":2.8,"b":3.2{extra}}}"#);
    let j = |extra: &str| follow_text(&three(), &w(extra)).unwrap();
    let free = parse(j("").as_bytes()).unwrap();
    let (a, b) = (&free.arr()[0], &free.arr()[1]);
    assert_eq!(a.get("origin").and_then(Value::str), Some("shifted"));
    assert!((a.get("found_shift").and_then(Value::f64).unwrap() - 0.3).abs() < 0.02);
    assert!(
        b.get("found_shift").is_some_and(Value::is_null),
        "no peak: no shift found"
    );
    let hard = parse(j(r#","max_shift":0.1"#).as_bytes()).unwrap();
    let a = &hard.arr()[0];
    assert_eq!(a.get("origin").and_then(Value::str), Some("too_far"));
    assert_eq!(
        (
            a.get("shift").and_then(Value::f64),
            a.get("a").and_then(Value::f64)
        ),
        (Some(0.0), Some(2.8))
    );
    assert!((a.get("found_shift").and_then(Value::f64).unwrap() - 0.3).abs() < 0.02);
}
