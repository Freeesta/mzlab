//! Hand-computed cases of `calib`: exact lines, the worked examples of Miller & Miller (unweighted §5.4, weighted §5.10), the closed forms of a line
//! forced through zero and of a 1/x fit, singular designs, and a property test: the back-calculation of the standards of an exact line returns them.

use super::*;

fn close(got: f64, want: f64, tol: f64) {
    assert!(
        (got - want).abs() <= tol * want.abs().max(1.0),
        "{got} != {want}"
    );
}

#[test]
fn an_exact_line_is_found_and_has_no_residual() {
    let x = [1.0, 2.0, 5.0, 10.0, 20.0];
    let y: Vec<f64> = x.iter().map(|v| 2.0 + 3.0 * v).collect();
    let f = fit(&x, &y, &[1.0; 5], Model::Linear, false).unwrap();
    close(f.a, 2.0, 1e-12);
    close(f.b, 3.0, 1e-12);
    assert_eq!(f.c, 0.0);
    close(f.r2.unwrap(), 1.0, 1e-12);
    assert!(f.s_yx.unwrap() < 1e-12);
    assert_eq!((f.n, f.p), (5, 2));
}

#[test]
fn the_unweighted_example_of_miller_and_miller() {
    // fluorescence of fluorescein standards, Miller & Miller 6th ed. §5.4: b = 1.93, a = 1.52, r = 0.9989, s_y/x = 0.4329
    let x = [0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0];
    let y = [2.1, 5.0, 9.0, 12.6, 17.3, 21.0, 24.7];
    let f = fit(&x, &y, &[1.0; 7], Model::Linear, false).unwrap();
    let b = 216.2 / 112.0; // Σ(x − x̄)(y − ȳ) / Σ(x − x̄)²
    close(f.b, b, 1e-12);
    close(f.a, 13.1 - 6.0 * b, 1e-12);
    close(f.s_yx.unwrap(), 0.432_847_713_240_052_7, 1e-12);
    close(f.r2.unwrap().sqrt(), 0.998_879_565_348_519_8, 1e-12);
    // without weights s_w is s_y/x
    assert_eq!(f.s_w, f.s_yx);
}

#[test]
fn the_weighted_example_of_miller_and_miller() {
    // §5.10: weights 1/s² of the replicate standard deviations; the book gives b = 0.0738, a = 0.0091
    let x = [0.0, 2.0, 4.0, 6.0, 8.0, 10.0];
    let y = [0.009, 0.158, 0.301, 0.472, 0.577, 0.739];
    let s = [0.001, 0.004, 0.010, 0.013, 0.017, 0.022];
    let w: Vec<f64> = s.iter().map(|v| 1.0 / (v * v)).collect();
    let f = fit(&x, &y, &w, Model::Linear, false).unwrap();
    assert!(
        (f.b - 0.0738).abs() < 5e-5 && (f.a - 0.0091).abs() < 5e-5,
        "{f:?}"
    );
    // the weights are relative: scaling them leaves the line
    let w2: Vec<f64> = w.iter().map(|v| v * 1e-6).collect();
    let g = fit(&x, &y, &w2, Model::Linear, false).unwrap();
    close(g.a, f.a, 1e-12);
    close(g.b, f.b, 1e-12);
}

#[test]
fn a_one_over_x_fit_equals_its_closed_form() {
    let x = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0];
    let y = [0.61, 1.05, 2.2, 4.9, 10.4, 19.1, 51.7];
    let w: Vec<f64> = x
        .iter()
        .map(|v| Weight::InvX.of(*v, 0.0).unwrap())
        .collect();
    let sw: f64 = w.iter().sum();
    let xm = (0..7).map(|i| w[i] * x[i]).sum::<f64>() / sw;
    let ym = (0..7).map(|i| w[i] * y[i]).sum::<f64>() / sw;
    let sxy: f64 = (0..7).map(|i| w[i] * (x[i] - xm) * (y[i] - ym)).sum();
    let sxx: f64 = (0..7).map(|i| w[i] * (x[i] - xm) * (x[i] - xm)).sum();
    let f = fit(&x, &y, &w, Model::Linear, false).unwrap();
    close(f.b, sxy / sxx, 1e-12);
    close(f.a, ym - f.b * xm, 1e-10);
    // 1/x² gives a different line, closer to the low standards
    let w2: Vec<f64> = x.iter().map(|v| 1.0 / (v * v)).collect();
    let g = fit(&x, &y, &w2, Model::Linear, false).unwrap();
    assert!((g.predict(0.5) - 0.61).abs() < (f.predict(0.5) - 0.61).abs());
}

#[test]
fn a_line_forced_through_zero_is_sum_xy_over_sum_xx() {
    let x = [1.0, 2.0, 4.0];
    let y = [3.1, 5.9, 12.2];
    let f = fit(&x, &y, &[1.0; 3], Model::Linear, true).unwrap();
    assert_eq!(f.a, 0.0);
    close(f.b, 63.7 / 21.0, 1e-12);
    assert_eq!(f.p, 1);
    // with weights 1/x: Σw·x·y / Σw·x² = Σy / Σx
    let w = [1.0, 0.5, 0.25];
    let g = fit(&x, &y, &w, Model::Linear, true).unwrap();
    close(g.b, 21.2 / 7.0, 1e-12);
}

#[test]
fn an_exact_quadratic_over_three_decades_keeps_its_digits() {
    let x: Vec<f64> = (0..10).map(|i| 1.0 * 2.15f64.powi(i)).collect();
    let y: Vec<f64> = x.iter().map(|v| 1.0 + 2.0 * v + 5e-4 * v * v).collect();
    let w: Vec<f64> = x.iter().map(|v| 1.0 / (v * v)).collect();
    let f = fit(&x, &y, &w, Model::Quadratic, false).unwrap();
    close(f.a, 1.0, 1e-9);
    close(f.b, 2.0, 1e-9);
    close(f.c, 5e-4, 1e-9);
    for (xi, yi) in x.iter().zip(&y) {
        close(f.inverse(*yi).unwrap(), *xi, 1e-9);
    }
}

#[test]
fn a_quadratic_without_a_real_root_gives_none() {
    let f = Fit {
        a: 0.0,
        b: 2.0,
        c: -1.0,
        n: 3,
        p: 3,
        r2: None,
        s_yx: None,
        s_w: None,
    };
    // the vertex is at y = 1
    assert_eq!(f.inverse(1.5), None);
    close(f.inverse(0.75).unwrap(), 0.5, 1e-15);
    let flat = Fit {
        b: 0.0,
        c: 0.0,
        ..f
    };
    assert_eq!(flat.inverse(1.0), None);
}

#[test]
fn singular_designs_and_bad_values_are_refused() {
    let one = [1.0; 4];
    assert_eq!(
        fit(&[2.0; 4], &[1.0, 2.0, 3.0, 4.0], &one, Model::Linear, false),
        Err(FitError::Singular)
    );
    assert_eq!(
        fit(&[0.0; 4], &[1.0; 4], &one, Model::Linear, true),
        Err(FitError::Singular)
    );
    // three points on two concentrations cannot hold a parabola
    assert_eq!(
        fit(
            &[1.0, 1.0, 2.0],
            &[1.0, 1.1, 2.0],
            &[1.0; 3],
            Model::Quadratic,
            false
        ),
        Err(FitError::Singular)
    );
    assert_eq!(
        fit(&[1.0], &[1.0], &[1.0], Model::Linear, false),
        Err(FitError::TooFewPoints)
    );
    assert_eq!(
        fit(
            &[1.0, 2.0],
            &[1.0, f64::NAN],
            &[1.0; 2],
            Model::Linear,
            false
        ),
        Err(FitError::NotFinite)
    );
    assert_eq!(
        fit(&[1.0, 2.0], &[1.0, 2.0], &[1.0, 0.0], Model::Linear, false),
        Err(FitError::NotFinite)
    );
    // two points: the line is exact, there is no residual standard deviation
    let f = fit(&[1.0, 2.0], &[1.0, 3.0], &[1.0; 2], Model::Linear, false).unwrap();
    assert_eq!(f.s_yx, None);
    assert_eq!(FitError::Singular.key(), "err.flow.curve.singular");
}

#[test]
fn weights_are_undefined_at_zero_and_named_as_the_step_writes_them() {
    assert_eq!(Weight::InvX.of(0.0, 5.0), None);
    assert_eq!(Weight::InvY2.of(5.0, -1.0), None);
    assert_eq!(Weight::One.of(0.0, 0.0), Some(1.0));
    assert_eq!(Weight::InvX2.of(2.0, 0.0), Some(0.25));
    for n in Weight::NAMES {
        assert_eq!(Weight::parse(n).unwrap().name(), n);
    }
    assert_eq!(Weight::parse("1/x^2"), None);
}

/// A small deterministic generator (no dependency): the same cases on every run.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[test]
fn back_calculation_of_an_exact_line_returns_every_standard() {
    let mut g = Lcg(7);
    for case in 0..500 {
        let n = 3 + (g.next() * 8.0) as usize;
        let lo = 10f64.powf(-3.0 + 4.0 * g.next());
        let x: Vec<f64> = (0..n)
            .map(|i| lo * (1.0 + i as f64 + g.next() * 0.5) * (1.0 + i as f64))
            .collect();
        let zero = case % 3 == 0;
        let quad = case % 2 == 1 && n >= 4;
        let a = if zero { 0.0 } else { (g.next() - 0.2) * 100.0 };
        let b = 10f64.powf(-2.0 + 6.0 * g.next());
        let c = if quad {
            b / x[n - 1] * 0.1 * g.next()
        } else {
            0.0
        };
        let y: Vec<f64> = x.iter().map(|v| a + b * v + c * v * v).collect();
        let scheme = Weight::ALL[case % 5];
        let w: Vec<f64> = (0..n)
            .map(|i| scheme.of(x[i], y[i]).unwrap_or(1.0))
            .collect();
        let model = if quad {
            Model::Quadratic
        } else {
            Model::Linear
        };
        let f = fit(&x, &y, &w, model, zero).unwrap();
        for (xi, yi) in x.iter().zip(&y) {
            let back = f.inverse(*yi).unwrap();
            assert!(
                (back - xi).abs() <= 1e-9 * xi.abs(),
                "case {case}: {back} != {xi} ({f:?})"
            );
        }
    }
}
