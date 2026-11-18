//! A window applied to another sample that finds no peak there keeps the engine's numbers (the chart shows the same ones).

use super::tests::rows_with_time;
use super::*;

/// One SIM sample of 10 min at 0.8 s: a background of 200 cps with a deterministic ripple of `noise` cps and a Gaussian of height `h` (sigma 3.2 s) at 7.19 min.
fn sample(name: &str, h: f64, noise: f64) -> String {
    let (mut t, mut y) = (Vec::new(), Vec::new());
    for i in 0..750 {
        let x = i as f64 * 0.8 / 60.0;
        let g = h * (-0.5 * ((x - 7.19) * 60.0 / 3.2).powi(2)).exp();
        t.push(format!("{x:.4}"));
        y.push(format!(
            "{:.1}",
            200.0 + noise * ((i * 7 % 11) as f64 - 5.0) / 5.0 + g
        ));
    }
    format!(
        "{{\"name\":\"{name}.mzML\",\"transitions\":[{{\"q1\":271.1,\"q3\":null,\"name\":\"\",\"rt\":[{}],\"y\":[{}]}}]}}",
        t.join(","),
        y.join(",")
    )
}

/// The value of column `n` in the row of the sample `s`.
fn cell(t: &str, s: &str, n: &str) -> String {
    let h: Vec<&str> = HEAD.trim().split(',').collect();
    let c = h.iter().position(|x| *x == n).unwrap();
    let r = rows_with_time(t);
    r.iter().find(|x| x[1] == s).unwrap()[c].clone()
}

/// A peak of 2.7e3 counts·s (height 337 cps, sigma 3.2 s) over a background of 200 in a sample where the applied window finds no peak (S/N under
/// `FOLLOW_SN`, a ripple of 300 cps): the row keeps the engine's area and height, the very numbers the same window drawn by hand there gets.
#[test]
fn a_no_peak_row_has_the_numbers_of_its_window() {
    let files = format!(
        "[{},{},{}]",
        sample("t7", 6e4, 30.0),
        sample("t8", 337.0, 300.0),
        sample("t9", 337.0, 0.0)
    );
    let win = "{\"file\":0,\"tr\":0,\"a\":6.931,\"b\":7.58,\"baseline\":\"edges\"";
    let t = table_text(&files, &format!("[{win},\"follow\":true}}]"), false).unwrap();
    assert_eq!(cell(&t, "t8", "origin"), "no_peak", "{t}");
    let own =
        format!("[{win}}},{{\"file\":1,\"tr\":0,\"a\":6.931,\"b\":7.58,\"baseline\":\"edges\"}}]");
    let d = table_text(&files, &own, false).unwrap();
    for n in ["area", "height", "snr", "apex_rt_min"] {
        assert_eq!(cell(&t, "t8", n), cell(&d, "t8", n), "{n}: {t} {d}");
    }
    let area: f64 = cell(&t, "t8", "area").parse().unwrap();
    assert!(area.is_finite() && area != 0.0, "{area}");
    // the clean sample (peak inside the window: applied): the analytic area h·sigma·sqrt(2π) of the Gaussian over the edge line
    assert_eq!(cell(&t, "t9", "origin"), "applied", "{t}");
    let a9: f64 = cell(&t, "t9", "area").parse().unwrap();
    let want = 337.0 * 3.2 * (2.0 * std::f64::consts::PI).sqrt();
    assert!((a9 - want).abs() < 0.03 * want, "{a9} vs {want}");
    let h9: f64 = cell(&t, "t9", "height").parse().unwrap();
    assert!((h9 - 337.0).abs() < 0.05 * 337.0, "{h9}");
}
