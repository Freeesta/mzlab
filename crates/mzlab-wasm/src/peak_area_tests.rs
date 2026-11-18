fn trace() -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..280).map(|i| f64::from(i) * 0.025).collect();
    let g = |t: f64, c: f64, h: f64| h * (-((t - c) / 0.06).powi(2) / 2.0).exp();
    let y = rt
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            g(t, 2.0, 2.0e4) + g(t, 2.24, 1.2e4) + 200.0 + f64::from((i * 7919 % 13) as u32) * 9.0
        })
        .collect();
    (rt, y)
}

#[test]
fn lr_auto_integrate_is_the_core_text() {
    use mzlab_core::area_unit::{lr_auto_json_in, AreaUnit};
    use mzlab_core::peakq_lrauto::{lr_auto_integrate, LrAutoParams};
    let (rt, y) = trace();
    let p = LrAutoParams {
        half: 3,
        ..Default::default()
    };
    let r = lr_auto_integrate(&rt, &y, 0.0, 7.0, p);
    let want = lr_auto_json_in(&rt, &r, AreaUnit::Seconds);
    assert_eq!(super::lr_auto_integrate(&rt, &y, 0.0, 7.0, &[3.0]), want);
    assert!(want.contains("\"end1\":\"valley\""), "{want}");
    // slot 6 = 1 keeps the old unit (counts x min); the other slots keep their defaults when negative
    let old = lr_auto_json_in(&rt, &r, AreaUnit::Minutes);
    let p6 = [3.0, -1.0, -1.0, -1.0, -1.0, -1.0, 1.0];
    assert_eq!(super::lr_auto_integrate(&rt, &y, 0.0, 7.0, &p6), old);
    assert_ne!(old, want);
}

#[test]
fn window_end_is_the_core_text() {
    let rt = [0.0, 0.1, 0.2, 0.3, 0.4];
    assert_eq!(
        super::window_end(&rt, 0.0, 0.4, 0.18, 0, f64::NAN, f64::NAN),
        "{\"t\":0.2,\"i\":2}"
    );
    assert_eq!(
        super::window_end(&rt, 0.4, 0.0, f64::NAN, -1, f64::NAN, f64::NAN),
        "{\"t\":0.3,\"i\":3}"
    );
    assert_eq!(
        super::window_end(&rt, 0.3, 0.4, 0.0, 0, 0.1, f64::NAN),
        "{\"t\":0.1,\"i\":1}"
    );
    assert_eq!(
        super::window_end(&rt[..1], 0.0, 0.0, 0.0, 0, f64::NAN, f64::NAN),
        ""
    );
}

#[test]
fn peak_area_is_the_core_text() {
    use super::{peak_area_json, Baseline};
    let (rt, y) = (vec![0.0, 0.5, 1.0, 1.5], vec![1.0, 9.0, 3.0, 1.0]);
    assert_eq!(
        super::peak_area(&rt, &y, 0.0, 1.5, 1, &[]),
        peak_area_json(&rt, &y, 0.0, 1.5, Baseline::Zero).unwrap()
    );
    let line = Baseline::Line {
        t0: 0.0,
        y0: 2.0,
        t1: 1.5,
        y1: 4.0,
    };
    assert_eq!(
        super::peak_area(&rt, &y, 0.0, 1.5, 2, &[0.0, 2.0, 1.5, 4.0]),
        peak_area_json(&rt, &y, 0.0, 1.5, line).unwrap()
    );
    // dragged past its second point the line is held at 0: the text carries `clamped` (the bytes are the core's, `baseline_test.rs`)
    let held = super::peak_area(&rt, &y, 0.0, 1.5, 2, &[0.0, 2.0, 0.5, 0.0]);
    assert!(held.contains("\"clamped\""), "{held}");
    assert_eq!(super::peak_area(&rt, &y, 0.0, 1.5, 0, &[]), "");
    assert_eq!(super::peak_area(&rt, &y, 0.0, 1.5, 2, &[1.0]), "");
}
