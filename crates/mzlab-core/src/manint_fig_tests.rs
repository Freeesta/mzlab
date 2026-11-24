//! The optional figure columns of `manint::table_text` (`columns`: `sn`, `fig`, `quality`): measured on a Gaussian of known width, empty where they
//! cannot be measured, and the bytes without them unchanged.

use crate::manint::tests::trace;
use crate::manint::*;

fn files() -> String {
    format!(
        "[{{\"name\":\"s1.mzML\",\"transitions\":[{}]}}]",
        trace(Some(237.1), None, "A", &[(3.0, 8e4)])
    )
}

fn win(extra: &str) -> String {
    format!("{{\"file\":0,\"tr\":0,\"a\":2.5,\"b\":3.5,\"baseline\":\"edges\"{extra}}}")
}

fn cells(doc: &str) -> (Vec<String>, Vec<String>) {
    let t = table_text(&files(), doc, false).unwrap();
    let mut l = t
        .lines()
        .map(|l| l.split(',').map(str::to_string).collect::<Vec<_>>());
    (l.next().unwrap(), l.next().unwrap())
}

#[test]
fn the_object_form_without_columns_is_the_array_form() {
    let w = win("");
    let a = table_text(&files(), &format!("[{w}]"), true).unwrap();
    let o = table_text(&files(), &format!("{{\"windows\":[{w}]}}"), true).unwrap();
    assert_eq!(a, o);
}

#[test]
fn figures_of_a_gaussian_peak() {
    let w = win("");
    let (h, r) = cells(&format!(
        "{{\"windows\":[{w}],\"columns\":[\"fig\",\"quality\"]}}"
    ));
    let n = HEAD.trim_end().split(',').count();
    assert_eq!(
        &h[n..],
        [
            "w_half_min",
            "plates",
            "tailing",
            "gaussian",
            "asymmetry",
            "noise_score"
        ]
    );
    let v = |i: usize| r[n + i].parse::<f64>().unwrap();
    // sigma 3.2 s: FWHM 0.1256 min, N = 5.54 (3 / 0.1256)^2 = 3160
    assert!((v(0) - 0.1256).abs() < 0.004, "w1/2 {}", v(0));
    assert!((v(1) / 3160.0 - 1.0).abs() < 0.06, "plates {}", v(1));
    assert!((v(2) - 1.0).abs() < 0.05, "tailing {}", v(2));
    assert!(v(3) > 0.99 && (v(4) - 1.0).abs() < 0.1);
}

#[test]
fn the_snr_needs_a_noise_zone() {
    let none = format!("{{\"windows\":[{}],\"columns\":[\"sn\"]}}", win(""));
    let (h, r) = cells(&none);
    let n = HEAD.trim_end().split(',').count();
    assert_eq!(h[n], "sn_zone");
    assert_eq!(r[n], "");
    let zone = format!(
        "{{\"windows\":[{}],\"columns\":[\"sn\"]}}",
        win(",\"noise\":[0.5,1.5]")
    );
    let (_, r) = cells(&zone);
    assert!(r[n].parse::<f64>().unwrap() > 100.0, "{}", r[n]);
}

#[test]
fn the_state_column_names_how_the_window_was_made() {
    let (h, r) = cells(&format!(
        "{{\"windows\":[{}],\"columns\":[\"state\"]}}",
        win("")
    ));
    let n = HEAD.trim_end().split(',').count();
    assert_eq!((h[n].as_str(), r[n].as_str()), ("state", "forced"));
    // `forced` is a given window (origin manual): the table without the column keeps its bytes
    let plain = table_text(&files(), &format!("[{}]", win("")), false).unwrap();
    let with = table_text(
        &files(),
        &format!("{{\"windows\":[{}],\"columns\":[\"state\"]}}", win("")),
        false,
    )
    .unwrap();
    let cut = |t: &str| {
        t.lines()
            .map(|l| l.rsplit_once(',').unwrap().0.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(cut(&with) + "\n", plain);
}

#[test]
fn a_window_too_narrow_gives_empty_cells_and_an_unknown_group_is_refused() {
    let w = "{\"file\":0,\"tr\":0,\"a\":3.0,\"b\":3.01,\"baseline\":\"edges\"}";
    let (_, r) = cells(&format!("{{\"windows\":[{w}],\"columns\":[\"fig\"]}}"));
    let n = HEAD.trim_end().split(',').count();
    assert_eq!(&r[n..], ["", "", ""]);
    let e = table_text(
        &files(),
        &format!("{{\"windows\":[{}],\"columns\":[\"x\"]}}", win("")),
        false,
    );
    assert!(e.is_err());
}

#[test]
fn the_sum_row_keeps_the_columns_aligned() {
    let w = win("");
    let t = table_text(
        &files(),
        &format!("{{\"windows\":[{w}],\"columns\":[\"sn\",\"fig\",\"quality\"]}}"),
        true,
    )
    .unwrap();
    let widths: Vec<usize> = t.lines().map(|l| l.split(',').count()).collect();
    assert!(widths.iter().all(|&n| n == widths[0]), "{widths:?}");
}
