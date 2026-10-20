//! The record of a score as the weighted sum of its terms (`mzlab_core::score_terms`).

use mzlab_core::json::Out;
use mzlab_core::score_terms::{ScoreTerms, Term};

#[test]
fn a_mean_has_normalised_weights_and_its_terms_give_back_the_value() {
    let r = ScoreTerms::mean(
        "x",
        &[("mass", 0.9, 1.0), ("iso", 0.5, 2.0), ("frag", 0.7, 1.0)],
        "score/x",
    );
    assert!((r.terms.iter().map(|t| t.weight).sum::<f64>() - 1.0).abs() < 1e-15);
    assert!((r.value - (0.9 + 1.0 + 0.7) / 4.0).abs() < 1e-15);
    assert!(r.residual().abs() < 1e-15);
}

#[test]
fn a_difference_is_a_term_with_a_negative_weight() {
    let r = ScoreTerms::sum(
        "rt",
        vec![
            Term::new("observed", 7.2, 1.0),
            Term::new("predicted", 6.9, -1.0),
        ],
        "score/rt",
    );
    assert!((r.value - 0.3).abs() < 1e-12);
    let mut o = Out::default();
    r.write(&mut o);
    assert!(o.0.starts_with("{\"name\":\"rt\",\"value\":0.29"));
    assert!(o.0.contains("\"weight\":-1}"));
    assert!(o.0.ends_with("\"formula_ref\":\"score/rt\"}"));
}

#[test]
fn without_weight_there_are_no_terms() {
    let r = ScoreTerms::mean("x", &[("a", 1.0, 0.0)], "score/x");
    assert!(r.terms.is_empty() && r.value == 0.0);
}
