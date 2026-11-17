//! `manint_auto::auto_text` on synthetic SIM traces (a flat background of 50, Gaussian peaks): the peaks of three files grouped by apex, the one with no
//! counterpart marked, the drift tolerance, the log of each file and the windows read back by `table_text`.

use super::tests::trace;
use super::{auto_text, table_text};

fn files(peaks: &[&[(f64, f64)]]) -> String {
    let f = |i: usize, p: &[(f64, f64)]| {
        format!(
            "{{\"name\":\"s{i}.mzML\",\"transitions\":[{}]}}",
            trace(Some(180.1), None, "", p)
        )
    };
    let l: Vec<String> = peaks.iter().enumerate().map(|(i, p)| f(i, p)).collect();
    format!("[{}]", l.join(","))
}

fn num(t: &str, key: &str) -> f64 {
    let at = t.find(&format!("\"{key}\":")).unwrap() + key.len() + 3;
    t[at..].split([',', '}']).next().unwrap().parse().unwrap()
}

// s0 and s1 share the peak at 3.0 min (s1 at 3.05), s2 has none; the peak at 6.0 min is only in s0
fn three() -> String {
    files(&[&[(3.0, 8e4), (6.0, 5e4)], &[(3.05, 6e4)], &[]])
}

#[test]
fn peaks_at_the_same_time_are_one_group_and_one_alone_has_no_counterpart() {
    let t = auto_text(&three(), "{}").unwrap();
    assert_eq!(num(&t, "groups"), 2.0);
    assert_eq!(num(&t, "unmatched"), 1.0);
    assert_eq!(t.matches("\"origin\":\"auto\"").count(), 2);
    assert_eq!(t.matches("\"origin\":\"unmatched\"").count(), 1);
    // the log: every file, with the apexes it found (s2 none)
    assert_eq!(t.matches("\"peaks\":[").count(), 3);
    assert!(t.contains("\"file\":2,\"tr\":0,\"peaks\":[]"), "{t}");
}

#[test]
fn the_tolerance_decides_whether_a_drifted_peak_is_the_same() {
    let f = files(&[&[(3.0, 8e4)], &[(3.5, 6e4)]]);
    let tight = auto_text(&f, "{\"rt_tol\":0.1}").unwrap();
    assert_eq!(
        (num(&tight, "groups"), num(&tight, "unmatched")),
        (2.0, 2.0)
    );
    let wide = auto_text(&f, "{\"rt_tol\":0.6}").unwrap();
    assert_eq!((num(&wide, "groups"), num(&wide, "unmatched")), (1.0, 0.0));
}

#[test]
fn one_file_alone_has_nothing_to_match() {
    let t = auto_text(&files(&[&[(3.0, 8e4)]]), "{}").unwrap();
    assert_eq!((num(&t, "groups"), num(&t, "unmatched")), (1.0, 0.0));
}

#[test]
fn the_windows_are_the_table_rows_grouped_by_peak_then_file() {
    let t = auto_text(&three(), "{}").unwrap();
    let w = &t[t.find("\"windows\":").unwrap() + 10..t.find(",\"log\"").unwrap()];
    let csv = table_text(&three(), w, false).unwrap();
    let rows: Vec<Vec<&str>> = csv
        .lines()
        .skip(1)
        .map(|l| l.split(',').collect())
        .collect();
    assert_eq!(rows.len(), 3, "{csv}");
    // sample, origin: the first peak of s0 then s1, then the peak of s0 alone
    let who: Vec<(&str, &str)> = rows.iter().map(|r| (r[1], r[12])).collect();
    assert_eq!(who, [("s0", "auto"), ("s1", "auto"), ("s0", "unmatched")]);
    assert!(rows.iter().all(|r| r[3].parse::<f64>().unwrap() > 0.0));
}

#[test]
fn the_same_request_gives_the_same_bytes() {
    assert_eq!(
        auto_text(&three(), "{}").unwrap(),
        auto_text(&three(), "").unwrap()
    );
}
