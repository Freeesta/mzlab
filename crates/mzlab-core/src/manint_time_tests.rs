//! The `time_min` column of `manint::table_text`: the time point of each sample, read from its name or given by the list, empty when unknown.

use super::tests::trace;
use super::*;

/// One file per `(name, extra JSON fields)`, each with the same single trace and a peak at 3 min.
fn files(list: &[(&str, &str)]) -> String {
    let f = |(name, extra): &(&str, &str)| {
        format!(
            "{{\"name\":\"{name}.mzML\"{extra},\"transitions\":[{}]}}",
            trace(Some(237.1), None, "A", &[(3.0, 8e4)])
        )
    };
    format!("[{}]", list.iter().map(f).collect::<Vec<_>>().join(","))
}

/// `(sample, time_min)` of every row of the table of one window per file.
fn times(list: &[(&str, &str)], sums: bool) -> Vec<(String, String)> {
    let w: Vec<String> = (0..list.len())
        .map(|i| format!("{{\"file\":{i},\"tr\":0,\"a\":2.8,\"b\":3.2,\"baseline\":\"zero\"}}"))
        .collect();
    let t = table_text(&files(list), &format!("[{}]", w.join(",")), sums).unwrap();
    assert!(t.starts_with(HEAD), "{t}");
    t.lines()
        .skip(1)
        .map(|l| {
            let c: Vec<&str> = l.split(',').collect();
            assert_eq!(c.len(), 14, "{l}");
            (c[1].to_string(), c[2].to_string())
        })
        .collect()
}

#[test]
fn the_time_comes_from_the_name_and_is_right_after_the_sample() {
    assert_eq!(
        HEAD.split(',').take(3).collect::<Vec<_>>(),
        ["channel", "sample", "time_min"]
    );
    let list = [
        ("RET t0 (2)", ""),
        ("RET 3h_2", ""),
        ("RET 1 h 30", ""),
        ("RET 90 s", ""),
        ("PERM 1.5h", ""),
        ("blk 90-10", ""),
        ("blk 3h", ""),
        ("H20 demi", ""),
    ];
    let got = times(&list, false);
    let want = [
        ("RET t0 (2)", "0"),
        ("RET 3h_2", "180"),
        ("RET 1 h 30", "90"),
        ("RET 90 s", "1.5"),
        ("PERM 1.5h", "90"),
        ("blk 90-10", ""),
        ("blk 3h", ""),
        ("H20 demi", ""),
    ];
    let want: Vec<(String, String)> = want
        .iter()
        .map(|(s, t)| (s.to_string(), t.to_string()))
        .collect();
    assert_eq!(got, want);
}

#[test]
fn the_time_of_the_list_wins_and_null_means_none() {
    let got = times(
        &[
            ("RET 3h", ",\"time\":45"),
            ("RET 3h ", ",\"time\":null"),
            ("S1", ",\"time\":0.25"),
            ("S2", ",\"time\":-5"),
        ],
        false,
    );
    let t: Vec<&str> = got.iter().map(|x| x.1.as_str()).collect();
    assert_eq!(t, ["45", "", "0.25", ""], "{got:?}");
}

#[test]
fn a_sum_row_has_no_time() {
    let got = times(&[("RET 3h", ""), ("RET 8h", "")], true);
    assert_eq!(got.len(), 3);
    assert_eq!(got[2], ("sum".into(), "".into()));
}
