//! The RT-logP residual opened in its terms (observed value, intercept, one term per descriptor with the negative of its coefficient): they add up to the
//! residual to 1e-12, and the residual of the check itself (rounded to 0.01) is the same number.

use mzlab_feat::attributio::review::rtlogp::{
    calibrate, check, check_terms, CalPoint, LogPValues, MIN_N,
};

/// Calibrants whose y follows the logP and the molecular weight, with a small deterministic scatter.
fn points(n: usize) -> Vec<CalPoint> {
    let mut x = 12345u64;
    let mut next = move || {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (x >> 33) as f64 / (1u64 << 31) as f64
    };
    (0..n)
        .map(|_| {
            let (lp, mw, tpsa, hbd, hba, nrot) = (
                -1.0 + 6.0 * next(),
                120.0 + 300.0 * next(),
                20.0 + 100.0 * next(),
                (4.0 * next()).floor(),
                (6.0 * next()).floor(),
                (8.0 * next()).floor(),
            );
            CalPoint {
                logp: LogPValues {
                    xlogp3: Some(lp),
                    crippen: Some(lp + 0.1),
                    class: None,
                    mw: Some(mw),
                    tpsa: Some(tpsa),
                    hbd: Some(hbd),
                    hba: Some(hba),
                    nrot: Some(nrot),
                },
                y: Some(1.0 + 1.4 * lp + 0.004 * mw + 0.2 * (next() - 0.5)),
            }
        })
        .collect()
}

#[test]
fn the_terms_of_the_residual_give_it_back_for_a_line_and_for_a_regression() {
    for n in [MIN_N + 2, 60] {
        let cal = points(n);
        let model = calibrate(&cal, 3.0, 0.0, MIN_N);
        assert!(model.fit.is_some(), "n={n}");
        let probe = LogPValues {
            xlogp3: Some(2.5),
            crippen: Some(2.6),
            class: None,
            mw: Some(250.0),
            tpsa: Some(60.0),
            hbd: Some(2.0),
            hba: Some(3.0),
            nrot: Some(4.0),
        };
        let y = Some(5.9);
        let t = check_terms(&model, &probe, y).expect("a prediction");
        assert!(t.residual().abs() < 1e-12, "n={n}");
        assert_eq!(t.terms[0].name, "observed");
        let c = check(&model, &probe, y, 3.0, 0.0);
        assert!((t.value - c.resid.unwrap()).abs() <= 0.005 + 1e-12, "n={n}");
    }
}

#[test]
fn without_a_prediction_there_is_no_record() {
    let model = calibrate(&points(MIN_N + 2), 3.0, 0.0, MIN_N);
    let none = LogPValues::default();
    assert!(check_terms(&model, &none, Some(5.0)).is_none());
    assert!(check_terms(
        &model,
        &LogPValues {
            xlogp3: Some(2.0),
            ..LogPValues::default()
        },
        None
    )
    .is_none());
}
