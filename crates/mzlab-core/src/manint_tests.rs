//! `manint::table_text` on synthetic SIM / MRM traces sampled every 0.8 s: rows grouped by channel then sample, the three baselines, the S/N, the
//! `sum` row, a channel without Q1, the errors, and a window split at its valleys.

use super::*;

/// A trace of 10 min at 0.8 s on a flat background of 50 with Gaussian peaks `(apex min, height)` of sigma 3.2 s.
pub(super) fn trace(q1: Option<f64>, q3: Option<f64>, name: &str, peaks: &[(f64, f64)]) -> String {
    let (mut t, mut y) = (Vec::new(), Vec::new());
    for i in 0..750 {
        let x = i as f64 * 0.8 / 60.0;
        t.push(format!("{x:.4}"));
        let s: f64 = peaks
            .iter()
            .map(|&(a, h)| h * (-0.5 * ((x - a) * 60.0 / 3.2).powi(2)).exp())
            .sum();
        y.push(format!("{:.1}", 50.0 + (i % 5) as f64 + s));
    }
    let n = |v: Option<f64>| v.map_or("null".to_string(), |q| q.to_string());
    format!(
        "{{\"q1\":{},\"q3\":{},\"name\":\"{name}\",\"rt\":[{}],\"y\":[{}]}}",
        n(q1),
        n(q3),
        t.join(","),
        y.join(",")
    )
}

fn files() -> String {
    let f = |name: &str, h: f64| {
        format!(
            "{{\"name\":\"{name}\",\"transitions\":[{},{},{}]}}",
            trace(Some(237.1), None, "A", &[(3.0, h)]),
            trace(Some(253.1), None, "B", &[(5.0, h / 2.0)]),
            trace(None, None, "", &[(7.0, h / 4.0)])
        )
    };
    format!("[{},{}]", f("s1.mzML", 8e4), f("s2.mzML", 4e4))
}

fn win(file: usize, tr: usize, a: f64, b: f64, base: &str) -> String {
    format!("{{\"file\":{file},\"tr\":{tr},\"a\":{a},\"b\":{b},\"baseline\":\"{base}\"}}")
}

fn rows(t: &str) -> Vec<Vec<String>> {
    t.lines()
        .skip(1)
        .map(|l| l.split(',').map(str::to_string).collect())
        .collect()
}

#[test]
fn rows_grouped_by_channel_then_sample_whatever_the_order_drawn() {
    let w = format!(
        "[{},{},{}]",
        win(1, 1, 4.8, 5.2, "zero"),
        win(1, 0, 2.8, 3.2, "zero"),
        win(0, 0, 2.8, 3.2, "zero")
    );
    let t = table_text(&files(), &w, false).unwrap();
    assert!(t.starts_with(HEAD), "{t}");
    let r = rows(&t);
    let key: Vec<(&str, &str)> = r.iter().map(|x| (x[0].as_str(), x[1].as_str())).collect();
    assert_eq!(key, [("237.1", "s1"), ("237.1", "s2"), ("253.1", "s2")]);
    assert_eq!((r[0][4].as_str(), r[0][5].as_str()), ("2.800", "3.200"));
    assert_eq!(
        r[0].len(),
        13,
        "no column for the method's name of the channel: {t}"
    );
}

#[test]
fn zero_and_line_are_the_measure_of_the_data_view() {
    let w = format!("[{}]", win(0, 0, 2.8, 3.2, "zero"));
    let r = rows(&table_text(&files(), &w, false).unwrap());
    let l = parse(files().as_bytes()).unwrap();
    let tr = &traces(l.arr()[0].get("transitions").unwrap())[0];
    let [area, _, height, apex] = peak_area(&tr.t, &tr.y, 2.8, 3.2, Baseline::Zero).unwrap();
    assert_eq!(r[0][3], sci(area, 5));
    assert_eq!(r[0][7], sci(height, 5));
    assert_eq!(r[0][6], format!("{apex:.3}"));
    assert_eq!(r[0][9], "zero");
    let line = "[{\"file\":0,\"tr\":0,\"a\":2.8,\"b\":3.2,\"baseline\":\"line\",\"line\":[2.8,52,3.2,52]}]";
    let r = rows(&table_text(&files(), line, false).unwrap());
    let want = peak_area(
        &tr.t,
        &tr.y,
        2.8,
        3.2,
        Baseline::Line {
            t0: 2.8,
            y0: 52.0,
            t1: 3.2,
            y1: 52.0,
        },
    )
    .unwrap();
    assert_eq!(
        (r[0][3].as_str(), r[0][9].as_str()),
        (sci(want[0], 5).as_str(), "line")
    );
}

#[test]
fn edges_removes_the_background_that_zero_keeps() {
    let w = format!(
        "[{},{}]",
        win(0, 0, 2.8, 3.2, "zero"),
        win(0, 0, 2.8, 3.2, "edges")
    );
    let r = rows(&table_text(&files(), &w, false).unwrap());
    let (z, e): (f64, f64) = (r[0][3].parse().unwrap(), r[1][3].parse().unwrap());
    // 24 s of background at about 52 counts, plus the peak's own tails at the edges (3.75 sigma; the 3-point mean of an edge reaches about 100 counts): edges takes them away, zero keeps them
    let d = z - e;
    assert!(d > 52.0 * 24.0 && d < (52.0 + 120.0) * 24.0, "{z} {e}");
    let peak = 8e4 * 3.2 * (2.0 * std::f64::consts::PI).sqrt();
    assert!((e - peak).abs() / peak < 0.02, "{e} vs {peak}");
}

#[test]
fn signal_to_noise_of_a_peak_and_empty_for_flat_noise() {
    let w = format!(
        "[{},{}]",
        win(0, 0, 2.8, 3.2, "zero"),
        win(0, 0, 8.6, 8.9, "zero")
    );
    let r = rows(&table_text(&files(), &w, false).unwrap());
    assert!(r[0][8].parse::<f64>().unwrap() > 100.0, "{:?}", r[0]);
    assert!(
        r[1][8].is_empty() || r[1][8].parse::<f64>().unwrap() < 10.0,
        "{:?}",
        r[1]
    );
}

#[test]
fn a_sum_row_closes_each_channel() {
    let w = format!(
        "[{},{},{}]",
        win(0, 0, 2.8, 3.2, "zero"),
        win(1, 0, 2.8, 3.2, "zero"),
        win(0, 2, 6.8, 7.2, "zero")
    );
    let t = table_text(&files(), &w, true).unwrap();
    let r = rows(&t);
    assert_eq!(r.len(), 5, "{t}");
    assert_eq!(&r[2][..3], ["237.1", "sum", ""]);
    let s: f64 = r[0][3].parse::<f64>().unwrap() + r[1][3].parse::<f64>().unwrap();
    assert_eq!(r[2][3], sci(s, 5));
    assert_eq!(r[2].len(), 13);
    assert_eq!(
        &r[3][..3],
        ["ion 3", "s1", ""],
        "a channel without Q1 is its place in the list"
    );
    assert_eq!(r[4][1], "sum");
}

#[test]
fn no_window_is_the_header_alone() {
    assert_eq!(table_text(&files(), "[]", true).unwrap(), HEAD);
}

#[test]
fn errors_name_the_channel_or_the_baseline() {
    let e = |w: &str| table_text(&files(), w, false).unwrap_err().key;
    assert_eq!(
        e(&format!("[{}]", win(5, 0, 1.0, 2.0, "zero"))),
        "err.manint.channel"
    );
    assert_eq!(
        e(&format!("[{}]", win(0, 9, 1.0, 2.0, "zero"))),
        "err.manint.channel"
    );
    assert_eq!(
        e(&format!("[{}]", win(0, 0, 1.0, 2.0, "valley"))),
        "err.manint.baseline"
    );
    assert_eq!(
        e("[{\"file\":0,\"tr\":0,\"a\":1,\"b\":2,\"baseline\":\"line\"}]"),
        "err.manint.baseline"
    );
}

#[test]
fn transition_label_and_quoted_sample_without_the_method_name() {
    let f = format!(
        "[{{\"name\":\"a,b.mzML\",\"transitions\":[{}]}}]",
        trace(Some(152.1), Some(110.0), "Quant, 1", &[(3.0, 1e4)])
    );
    let r = table_text(&f, &format!("[{}]", win(0, 0, 2.8, 3.2, "edges")), false).unwrap();
    assert!(
        r.lines().nth(1).unwrap().starts_with("152.1>110,\"a,b\",,"),
        "{r}"
    );
}

#[test]
fn the_name_of_a_window_is_the_third_column_cleaned_and_cut() {
    let long = "x".repeat(LABEL_MAX + 5);
    let w = format!(
        "[{{\"file\":0,\"tr\":0,\"a\":2.8,\"b\":3.2,\"label\":\"  Peak 1, left\\n \"}},{{\"file\":1,\"tr\":0,\"a\":2.8,\"b\":3.2,\"label\":\"{long}\"}},{}]",
        win(0, 2, 6.8, 7.2, "zero")
    );
    let t = table_text(&files(), &w, true).unwrap();
    let l: Vec<&str> = t.lines().collect();
    assert!(l[1].starts_with("237.1,s1,\"Peak 1, left\","), "{t}");
    assert!(
        l[2].starts_with(&format!("237.1,s2,{},", "x".repeat(LABEL_MAX))),
        "{t}"
    );
    assert!(
        l[3].starts_with("237.1,sum,,"),
        "a sum row has no name: {t}"
    );
    assert!(
        l[4].starts_with("ion 3,s1,,"),
        "a window without a name: {t}"
    );
}

/// Two peaks of sample 1 on channel A that overlap: apexes 3.0 and 3.2 min, the second of half height.
fn pair() -> String {
    format!(
        "[{{\"name\":\"s1.mzML\",\"transitions\":[{}]}}]",
        trace(Some(237.1), None, "A", &[(3.0, 8e4), (3.2, 4e4)])
    )
}

#[test]
fn a_window_split_at_its_valley_gives_a_row_per_part_that_add_up() {
    for base in ["zero", "edges"] {
        let one = format!("{{\"file\":0,\"tr\":0,\"a\":2.7,\"b\":3.5,\"baseline\":\"{base}\"}}");
        let v = valleys_text(&pair(), &one, valley::DEPTH).unwrap();
        let at: f64 = v
            .strip_prefix("{\"splits\":[")
            .and_then(|r| r.strip_suffix("]}"))
            .unwrap()
            .parse()
            .unwrap();
        assert!(at > 3.05 && at < 3.2, "{v}");
        let split = format!("[{}]", one.replace('}', &format!(",\"splits\":[{at}]}}")));
        let t = table_text(&pair(), &split, false).unwrap();
        let r = rows(&t);
        assert_eq!(r.len(), 2, "{t}");
        assert_eq!((r[0][4].as_str(), r[1][5].as_str()), ("2.700", "3.500"));
        assert_eq!(r[0][5], r[1][4], "the parts meet at the drop line");
        assert_eq!((r[0][10].as_str(), r[1][10].as_str()), ("drop", "drop"));
        let pv: Vec<f64> = r.iter().map(|x| x[11].parse().unwrap()).collect();
        assert!(pv[0] > pv[1] && pv[1] > 1.0, "{pv:?}");
        assert!((r[0][6].parse::<f64>().unwrap() - 3.0).abs() < 0.02);
        // the parts add up to the window measured against the same line
        let t0 = &samples_and_keys(&pair()).unwrap().0[0].1[0];
        let (lo, hi) = (near_idx(&t0.t, 2.7), near_idx(&t0.t, 3.5));
        let line = drop_line(
            baseline_of(&parse(one.as_bytes()).unwrap(), 0.0).unwrap(),
            t0,
            lo,
            hi,
        );
        let whole =
            crate::baseline::integrate(&t0.t, &t0.y, lo, hi, |x| line.at(x).unwrap()).area * 60.0;
        let sum: f64 = valley::parts(&t0.t, &t0.y, lo, hi, &[near_idx(&t0.t, at)], |x| {
            line.at(x).unwrap()
        })
        .iter()
        .map(|p| p.area * 60.0)
        .sum();
        assert!(
            (sum - whole).abs() <= 1e-9 * whole,
            "{base}: {sum} vs {whole}"
        );
        let printed: f64 = r.iter().map(|x| x[3].parse::<f64>().unwrap()).sum();
        assert!(
            (printed - whole).abs() <= 1e-4 * whole,
            "{base}: {printed} vs {whole}"
        );
    }
}

#[test]
fn splits_outside_the_window_leave_one_row() {
    let w = r#"[{"file":0,"tr":0,"a":2.7,"b":3.5,"splits":[1.0,2.7,9.0,null]}]"#;
    let r = rows(&table_text(&pair(), w, false).unwrap());
    assert_eq!(r.len(), 1);
    assert_eq!((r[0][10].as_str(), r[0][11].as_str()), ("", ""));
    let one = r#"{"file":0,"tr":0,"a":2.7,"b":3.5}"#;
    assert_eq!(valleys_text(&pair(), one, 1.0).unwrap(), "{\"splits\":[]}");
    assert_eq!(
        valleys_text(&pair(), r#"{"file":3,"tr":0,"a":1,"b":2}"#, 0.1)
            .unwrap_err()
            .key,
        "err.manint.channel"
    );
}

// two transitions closer than 0.005 Da in one file are two channels, each with its own rows, never folded into the first
#[test]
fn transitions_closer_than_the_rounding_stay_two_channels() {
    let f = format!(
        "[{{\"name\":\"s1.mzML\",\"transitions\":[{},{}]}}]",
        trace(Some(300.001), Some(150.0), "", &[(3.0, 8e4)]),
        trace(Some(300.004), Some(150.0), "", &[(5.0, 4e4)])
    );
    let w = format!(
        "[{},{}]",
        win(0, 0, 2.8, 3.2, "zero"),
        win(0, 1, 4.8, 5.2, "zero")
    );
    let r = rows(&table_text(&f, &w, true).unwrap());
    let ch: Vec<&str> = r.iter().map(|x| x[0].as_str()).collect();
    assert_eq!(
        ch,
        ["300.001>150", "300.001>150", "300.004>150", "300.004>150"]
    );
    assert_eq!((r[1][1].as_str(), r[3][1].as_str()), ("sum", "sum"));
}

// three samples with one peak at 3.0 min: s1 exactly there, s2 drifted by +0.30 min (outside a ±0.2 window), s3 with no peak at all
fn three() -> String {
    let f = |name: &str, peaks: &[(f64, f64)]| {
        format!(
            "{{\"name\":\"{name}.mzML\",\"transitions\":[{}]}}",
            trace(Some(180.1), None, "", peaks)
        )
    };
    format!(
        "[{},{},{}]",
        f("t0", &[(3.0, 8e4)]),
        f("t3h", &[(3.3, 6e4)]),
        f("t8h", &[])
    )
}

fn follow_win(label: &str) -> String {
    format!("[{{\"file\":0,\"tr\":0,\"a\":2.8,\"b\":3.2,\"follow\":true,\"label\":\"{label}\"}}]")
}

#[test]
fn a_window_applied_to_the_other_samples_follows_the_apex_and_marks_no_peak() {
    let r = rows(&table_text(&three(), &follow_win("Cpd"), false).unwrap());
    assert_eq!(r.len(), 3);
    let col = |i: usize| r.iter().map(|x| x[i].as_str()).collect::<Vec<_>>();
    assert_eq!(
        col(1),
        ["t0", "t3h", "t8h"],
        "one row per sample, in the order of the samples"
    );
    assert_eq!(
        col(2),
        ["Cpd", "Cpd", "Cpd"],
        "the name is the window's, in every file"
    );
    assert_eq!(col(12), ["manual", "shifted", "no_peak"]);
    assert_eq!((col(4)[0], col(5)[0]), ("2.800", "3.200"));
    // the drifted peak: the edges move by the apex offset (3.3 - 3.0), so the whole peak is integrated
    let (a, b): (f64, f64) = (col(4)[1].parse().unwrap(), col(5)[1].parse().unwrap());
    assert!(
        (a - 3.1).abs() < 0.02 && ((b - a) - 0.4).abs() < 1e-6,
        "{a} {b}"
    );
    assert!(
        (r[1][6].parse::<f64>().unwrap() - 3.3).abs() < 0.02,
        "{r:?}"
    );
    // no peak: the same edges, still integrated, never hidden
    assert_eq!((col(4)[2], col(5)[2]), ("2.800", "3.200"));
    assert!(r[2][3].parse::<f64>().unwrap() > 0.0);
}

#[test]
fn an_apex_inside_the_window_keeps_the_edges() {
    // the apex of s2 lies at 3.1, inside 2.8..3.2: applied, not shifted
    let f = format!(
        "[{{\"name\":\"a.mzML\",\"transitions\":[{}]}},{{\"name\":\"b.mzML\",\"transitions\":[{}]}}]",
        trace(Some(180.1), None, "", &[(3.0, 8e4)]),
        trace(Some(180.1), None, "", &[(3.1, 6e4)])
    );
    let r = rows(&table_text(&f, &follow_win(""), false).unwrap());
    assert_eq!(
        (r[1][4].as_str(), r[1][5].as_str(), r[1][12].as_str()),
        ("2.800", "3.200", "applied")
    );
}

#[test]
fn a_window_of_its_own_in_a_sample_overrides_the_applied_one_there_only() {
    let w = "[{\"file\":0,\"tr\":0,\"a\":2.8,\"b\":3.2,\"follow\":true},{\"file\":1,\"tr\":0,\"a\":3.2,\"b\":3.4,\"label\":\"mine\"}]";
    let r = rows(&table_text(&three(), w, false).unwrap());
    let c = |i: usize| {
        r.iter()
            .map(|x| (x[1].as_str(), x[12].as_str()))
            .nth(i)
            .unwrap()
    };
    assert_eq!(r.len(), 3, "{r:?}");
    assert_eq!(
        (c(0), c(1), c(2)),
        (("t0", "manual"), ("t3h", "manual"), ("t8h", "no_peak"))
    );
    assert_eq!((r[1][2].as_str(), r[1][4].as_str()), ("mine", "3.200"));
}

#[test]
fn a_name_given_to_one_sample_alone_keeps_it() {
    let w =
        r#"[{"file":0,"tr":0,"a":2.8,"b":3.2,"follow":true,"label":"Cpd","names":{"2":"odd"}}]"#;
    let r = rows(&table_text(&three(), w, false).unwrap());
    assert_eq!(
        r.iter().map(|x| x[2].as_str()).collect::<Vec<_>>(),
        ["Cpd", "Cpd", "odd"]
    );
}

#[test]
fn rows_are_ordered_by_peak_then_sample() {
    // two peaks (3.0 and 5.0 min) each applied to both samples: peak 3.0 in s1, s2, then peak 5.0 in s1, s2
    let w = "[{\"file\":0,\"tr\":0,\"a\":4.8,\"b\":5.2,\"follow\":true},{\"file\":0,\"tr\":0,\"a\":2.8,\"b\":3.2,\"follow\":true}]";
    let one = |n: &str| {
        format!(
            "{{\"name\":\"{n}.mzML\",\"transitions\":[{}]}}",
            trace(Some(180.1), None, "", &[(3.0, 8e4), (5.0, 8e4)])
        )
    };
    let f = format!("[{},{}]", one("s1"), one("s2"));
    let r = rows(&table_text(&f, w, false).unwrap());
    let key: Vec<(&str, &str)> = r.iter().map(|x| (x[4].as_str(), x[1].as_str())).collect();
    assert_eq!(
        key,
        [
            ("2.800", "s1"),
            ("2.800", "s2"),
            ("4.800", "s1"),
            ("4.800", "s2")
        ]
    );
}

#[test]
fn follow_text_gives_the_windows_the_table_uses() {
    let w = r#"{"file":0,"tr":0,"a":2.8,"b":3.2}"#;
    let t = follow_text(&three(), w).unwrap();
    let v = parse(t.as_bytes()).unwrap();
    let o = |i: usize, k: &str| v.arr()[i].get(k).unwrap().clone();
    assert_eq!(v.arr().len(), 2);
    assert_eq!(o(0, "origin").str(), Some("shifted"));
    assert!((o(0, "shift").f64().unwrap() - 0.3).abs() < 0.02);
    assert_eq!(
        (o(1, "origin").str(), o(1, "a").f64()),
        (Some("no_peak"), Some(2.8))
    );
    // the table gives the same edges to the same rows
    let r = rows(&table_text(&three(), &follow_win(""), false).unwrap());
    assert_eq!(
        r[1][4].parse::<f64>().unwrap(),
        (o(0, "a").f64().unwrap() * 1e3).round() / 1e3
    );
}
