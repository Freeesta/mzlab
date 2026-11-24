//! The value state and the S/N of every concentration row of `quant.curve@1` (AU-3): `detected`, `below_loq`, `absent` and `na`, and the S/N of the peak.

use super::tests::{exact, line, note_of, run};
use super::*;

/// The `state` and `sn` of a row, the last two fields.
fn state_sn(line: &str) -> (&str, &str) {
    let mut f = line.rsplit(',');
    let sn = f.next().unwrap_or("");
    (f.next().unwrap_or(""), sn)
}

#[test]
fn every_row_has_a_state_and_the_s_n_of_its_peak() {
    let (r, s) = exact();
    let (doc, csv, _) = run(&[], &r, &s);
    assert!(CSV_HEAD.ends_with(",note,state,sn\n"));
    for pt in doc
        .get("curves")
        .unwrap()
        .at(0)
        .unwrap()
        .get("points")
        .unwrap()
        .arr()
    {
        assert_eq!(pt.get("state").and_then(Value::str), Some("detected"));
        let (x, sn) = (
            pt.get("x").and_then(Value::f64).unwrap(),
            pt.get("sn").and_then(Value::f64).unwrap(),
        );
        assert_eq!(sn, 6.0 * x);
    }
    // a number under the range is `below_loq`; the others were measured, and a peak without a value in the S/N column leaves it empty
    assert_eq!(state_sn(line(&csv, "A,b,blank,")), ("below_loq", ""));
    assert_eq!(state_sn(line(&csv, "A,u2,")), ("detected", ""));
    assert_eq!(state_sn(line(&csv, "A,s2,")), ("detected", "12"));
}

#[test]
fn a_sample_without_a_peak_or_without_a_row_has_its_own_state() {
    let (r, mut s) = exact();
    // u3: the integration found no peak (flag no_peak, S/N kept as evidence); u4 is in the standards table but not in the responses
    let mut r: String = r
        .lines()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                format!("{l},peak\n")
            } else {
                format!("{l},ok\n")
            }
        })
        .collect();
    r += "u3,A,,,2.5,no_peak\n";
    s += "u3;;;;sample\nu4;;;;sample\n";
    let (_, csv, _) = run(&[], &r, &s);
    assert_eq!(state_sn(line(&csv, "A,u3,")), ("absent", "2.5"));
    assert_eq!(note_of(line(&csv, "A,u3,")), "no_response");
    assert_eq!(state_sn(line(&csv, "A,u4,")), ("na", ""));
    assert_eq!(note_of(line(&csv, "A,u4,")), "no_response");
}
