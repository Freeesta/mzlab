//! `baseline::integrate` and `peak_area` on synthetic traces with known areas: a Gaussian against zero (its analytic area and the plain trapezoid of
//! y), a level and a sloped line cutting through the peak (signed area and the part under the line from the error function), a triangle (exact).

use super::{integrate, peak_area, Baseline};

/// `h exp(-(t - c)^2 / 2 s^2)` sampled every `dt` min from `c - w` to `c + w`.
fn gauss(c: f64, s: f64, h: f64, w: f64, dt: f64) -> (Vec<f64>, Vec<f64>) {
    let n = (2.0 * w / dt).round() as usize;
    let rt: Vec<f64> = (0..=n).map(|i| c - w + i as f64 * dt).collect();
    let y = rt
        .iter()
        .map(|t| h * (-((t - c).powi(2)) / (2.0 * s * s)).exp())
        .collect();
    (rt, y)
}

/// The integral of the Gaussian from `c - x` to `c + x`.
fn g_int(s: f64, h: f64, x: f64) -> f64 {
    h * s * (2.0 * std::f64::consts::PI).sqrt() * libm::erf(x / (s * std::f64::consts::SQRT_2))
}

#[test]
fn from_zero_is_the_trapezoid_of_y_and_the_analytic_area() {
    let (c, s, h) = (5.0, 0.05, 1.0e6);
    let (rt, y) = gauss(c, s, h, 6.0 * s, 0.001);
    let n = rt.len() - 1;
    let r = integrate(&rt, &y, 0, n, |_| 0.0);
    let trap: f64 = (1..=n)
        .map(|j| (y[j] + y[j - 1]) / 2.0 * (rt[j] - rt[j - 1]))
        .sum();
    assert!((r.area - trap).abs() <= 1e-9 * trap);
    assert!((r.area - g_int(s, h, 6.0 * s)).abs() < 1e-6 * r.area);
    assert_eq!((r.below, r.height, rt[r.apex]), (0.0, h, c));
    // the Data view: counts x s, boundaries at the nearest points
    let v = peak_area(&rt, &y, c - 6.0 * s, c + 6.0 * s + 1e-4, Baseline::Zero).unwrap();
    assert_eq!(v, [r.area * 60.0, 0.0, h, c]);
}

#[test]
fn a_level_inside_the_peak_gives_the_signed_area_and_the_part_below() {
    let (c, s, h, w) = (2.0, 0.04, 2.0e5, 0.24);
    let (rt, y) = gauss(c, s, h, w, 0.0005);
    let lvl = h / 2.0;
    let r = integrate(&rt, &y, 0, rt.len() - 1, |_| lvl);
    let xh = s * (2.0 * std::f64::consts::LN_2).sqrt();
    let signed = g_int(s, h, w) - lvl * 2.0 * w;
    let below = lvl * 2.0 * (w - xh) - (g_int(s, h, w) - g_int(s, h, xh));
    assert!(
        (r.area - signed).abs() < 1e-5 * below,
        "{} {signed}",
        r.area
    );
    assert!(
        (r.below - below).abs() < 1e-5 * below,
        "{} {below}",
        r.below
    );
    assert!(r.area < 0.0 && r.below > 0.0);
    assert_eq!(r.height, h - lvl);
}

#[test]
fn a_sloped_line_through_the_peak() {
    // the integral of a line is the trapezoid of its ends, so the signed area is known whatever the slope
    let (c, s, h, w) = (3.0, 0.05, 1.0e4, 0.3);
    let (rt, y) = gauss(c, s, h, w, 0.0005);
    let line = Baseline::Line {
        t0: c - w,
        y0: 0.2 * h,
        t1: c + w,
        y1: 0.6 * h,
    };
    let r = integrate(&rt, &y, 0, rt.len() - 1, |t| line.at(t).unwrap());
    let signed = g_int(s, h, w) - (0.2 + 0.6) / 2.0 * h * 2.0 * w;
    assert!((r.area - signed).abs() < 1e-6 * h, "{} {signed}", r.area);
    assert!(r.below > 0.0 && (r.area + r.below) > 0.0);
}

#[test]
fn a_triangle_split_where_it_crosses_the_line() {
    let (rt, y) = (vec![0.0, 1.0, 2.0], vec![0.0, 10.0, 0.0]);
    let r = integrate(&rt, &y, 0, 2, |_| 5.0);
    assert_eq!((r.area, r.below, r.height, r.apex), (0.0, 2.5, 5.0, 1));
    // all under the line: negative area and height
    let r = integrate(&rt, &y, 0, 2, |_| 20.0);
    assert_eq!((r.area, r.below, r.height, r.apex), (-30.0, 30.0, -10.0, 1));
    // one point: nothing
    assert_eq!(integrate(&rt, &y, 1, 1, |_| 0.0).area, 0.0);
}

#[test]
fn the_parameters_and_the_page_measure() {
    let num = |v: &'static [(&'static str, f64)]| {
        move |k: &str| v.iter().find(|(n, _)| *n == k).map(|(_, x)| *x)
    };
    assert_eq!(Baseline::from_params(None, num(&[])), Ok(Baseline::Edges));
    assert_eq!(
        Baseline::from_params(Some("zero"), num(&[])),
        Ok(Baseline::Zero)
    );
    let lvl = Baseline::from_params(Some("line"), num(&[("base_y0", 7.0)])).unwrap();
    assert_eq!(
        (lvl.at(-3.0), lvl.at(9.0), lvl.name()),
        (Some(7.0), Some(7.0), "line")
    );
    let sl = Baseline::from_params(
        Some("line"),
        num(&[
            ("base_t0", 1.0),
            ("base_y0", 0.0),
            ("base_t1", 3.0),
            ("base_y1", 4.0),
        ]),
    )
    .unwrap();
    assert_eq!(sl.at(2.0), Some(2.0));
    assert!(Baseline::from_params(Some("line"), num(&[])).is_err());
    assert!(Baseline::from_params(Some("valley"), num(&[])).is_err());
    assert_eq!(Baseline::Edges.at(1.0), None);
    let (rt, y) = (vec![0.0, 0.5, 1.0, 1.5], vec![1.0, 3.0, 1.0, 0.0]);
    assert_eq!(peak_area(&rt, &y, 0.1, 1.1, Baseline::Edges), None);
    // 0.1 -> point 0, 1.1 -> point 2; boundaries in any order
    assert_eq!(
        peak_area(&rt, &y, 1.1, 0.1, Baseline::Zero),
        Some([120.0, 0.0, 3.0, 0.5])
    );
}

#[test]
fn the_json_of_the_page() {
    let (rt, y) = (vec![0.0, 0.5, 1.0], vec![0.0, 4.0, 0.0]);
    let line = Baseline::Line {
        t0: 0.0,
        y0: 1.0,
        t1: 1.0,
        y1: 1.0,
    };
    assert_eq!(
        super::peak_area_json(&rt, &y, 0.0, 1.0, line).unwrap(),
        "{\"baseline\":\"line\",\"area\":60,\"below\":7.5,\"height\":3,\"rt\":0.5}"
    );
    assert_eq!(
        super::peak_area_json(&rt, &y, 0.0, 1.0, Baseline::Edges),
        None
    );
}
