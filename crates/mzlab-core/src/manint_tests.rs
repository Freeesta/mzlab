//! `manint::table_text` on synthetic SIM / MRM traces sampled every 0.8 s: rows grouped by channel then sample, the three baselines, the S/N, the
//! `sum` row, a channel without Q1, and the errors.

use super::*;

/// A trace of 10 min at 0.8 s on a flat background of 50 with Gaussian peaks `(apex min, height)` of sigma 3.2 s.
fn trace(q1: Option<f64>, q3: Option<f64>, name: &str, peaks: &[(f64, f64)]) -> String {
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
    let key: Vec<(&str, &str)> = r.iter().map(|x| (x[0].as_str(), x[2].as_str())).collect();
    assert_eq!(key, [("237.1", "s1"), ("237.1", "s2"), ("253.1", "s2")]);
    assert_eq!(r[0][1], "A");
    assert_eq!((r[0][3].as_str(), r[0][4].as_str()), ("2.800", "3.200"));
}

#[test]
fn zero_and_line_are_the_measure_of_the_data_view() {
    let w = format!("[{}]", win(0, 0, 2.8, 3.2, "zero"));
    let r = rows(&table_text(&files(), &w, false).unwrap());
    let l = parse(files().as_bytes()).unwrap();
    let tr = &traces(l.arr()[0].get("transitions").unwrap())[0];
    let [area, _, height, apex] = peak_area(&tr.t, &tr.y, 2.8, 3.2, Baseline::Zero).unwrap();
    assert_eq!(r[0][6], sci(area, 5));
    assert_eq!(r[0][7], sci(height, 5));
    assert_eq!(r[0][5], format!("{apex:.3}"));
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
        (r[0][6].as_str(), r[0][9].as_str()),
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
    let (z, e): (f64, f64) = (r[0][6].parse().unwrap(), r[1][6].parse().unwrap());
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
    assert_eq!(&r[2][..3], ["237.1", "A", "sum"]);
    let s: f64 = r[0][6].parse::<f64>().unwrap() + r[1][6].parse::<f64>().unwrap();
    assert_eq!(r[2][6], sci(s, 5));
    assert_eq!(r[2].len(), 10);
    assert_eq!(
        &r[3][..3],
        ["ion 3", "", "s1"],
        "a channel without Q1 is its place in the list"
    );
    assert_eq!(r[4][2], "sum");
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
fn transition_label_and_quoted_names() {
    let f = format!(
        "[{{\"name\":\"a,b.mzML\",\"transitions\":[{}]}}]",
        trace(Some(152.1), Some(110.0), "Quant, 1", &[(3.0, 1e4)])
    );
    let r = table_text(&f, &format!("[{}]", win(0, 0, 2.8, 3.2, "edges")), false).unwrap();
    assert!(
        r.lines()
            .nth(1)
            .unwrap()
            .starts_with("152.1>110,\"Quant, 1\",\"a,b\","),
        "{r}"
    );
}
