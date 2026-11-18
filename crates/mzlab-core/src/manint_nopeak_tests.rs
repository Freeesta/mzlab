//! A window applied to another sample that finds no peak there keeps the engine's numbers (the chart shows the same ones).

use super::tests::rows_with_time;
use super::*;

/// A low noisy peak (about 2.7e3 counts·s over a background of 200) in a sample where the applied window finds no peak (S/N under `FOLLOW_SN`):
/// the row keeps the engine's area and height of the window as drawn, the same numbers the chart shows, never blank.
#[test]
fn a_no_peak_row_keeps_the_area_and_height_of_its_window() {
    let sample = |name: &str, h: f64| {
        let (mut t, mut y) = (Vec::new(), Vec::new());
        for i in 0..750 {
            let x = i as f64 * 0.8 / 60.0;
            let g = h * (-0.5 * ((x - 7.19) * 60.0 / 3.2).powi(2)).exp();
            t.push(format!("{x:.4}"));
            y.push(format!(
                "{:.1}",
                200.0 + 60.0 * ((i * 7 % 11) as f64 - 5.0) / 5.0 + g
            ));
        }
        format!(
            "{{\"name\":\"{name}.mzML\",\"transitions\":[{{\"q1\":271.1,\"q3\":null,\"name\":\"\",\"rt\":[{}],\"y\":[{}]}}]}}",
            t.join(","),
            y.join(",")
        )
    };
    let files = format!("[{},{}]", sample("t7", 6e4), sample("t8", 60.0));
    let w = "[{\"file\":0,\"tr\":0,\"a\":6.931,\"b\":7.58,\"baseline\":\"edges\",\"follow\":true}]";
    let t = table_text(&files, w, false).unwrap();
    let r = rows_with_time(&t);
    let h: Vec<&str> = HEAD.trim().split(',').collect();
    let c = |n: &str| h.iter().position(|x| *x == n).unwrap();
    let t8 = &r[1];
    assert_eq!(t8[c("origin")], "no_peak", "{t}");
    let (area, height): (f64, f64) = (
        t8[c("area")].parse().unwrap(),
        t8[c("height")].parse().unwrap(),
    );
    assert!(area.is_finite() && area != 0.0 && height > 0.0, "{t8:?}");
    assert!(
        !t8[c("snr")].is_empty(),
        "the S/N of the window is there too: {t8:?}"
    );
}
