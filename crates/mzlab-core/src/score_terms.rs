//! The terms of a score: every score the program shows can be opened as a weighted sum of named terms, `value = Σ weight · term value`. The record is written
//! next to the score (`score_terms`) only when a route is asked for it, so the existing outputs stay byte for byte as they are. Used by the library match,
//! the in-silico score, the isotope/mass score and the RT check.

use crate::json::Out;

/// One term of a score: its name, its value and the weight it has in the sum (weights may be negative: a difference is a term with weight -1).
#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub name: String,
    pub value: f64,
    pub weight: f64,
}

/// A score and what it is made of. `formula_ref` is the stable key of the formula in the Theory (the page links it; the Theory owns the text).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreTerms {
    pub name: &'static str,
    pub value: f64,
    pub terms: Vec<Term>,
    pub formula_ref: &'static str,
}

impl Term {
    pub fn new(name: impl Into<String>, value: f64, weight: f64) -> Term {
        Term {
            name: name.into(),
            value,
            weight,
        }
    }
}

impl ScoreTerms {
    /// The record whose value is the weighted sum of `terms`.
    pub fn sum(name: &'static str, terms: Vec<Term>, formula_ref: &'static str) -> ScoreTerms {
        let value = terms.iter().map(|t| t.weight * t.value).sum();
        ScoreTerms {
            name,
            value,
            terms,
            formula_ref,
        }
    }

    /// Weighted mean of the `(name, value, raw weight)` parts: the weights of the record are the raw ones over their total (a part that is not there is left out
    /// by the caller). Zero total weight: no terms and value 0, as the scores it describes do.
    pub fn mean(
        name: &'static str,
        parts: &[(&str, f64, f64)],
        formula_ref: &'static str,
    ) -> ScoreTerms {
        let total: f64 = parts.iter().map(|p| p.2).sum();
        if total <= 0.0 {
            return ScoreTerms {
                name,
                value: 0.0,
                terms: Vec::new(),
                formula_ref,
            };
        }
        let terms = parts
            .iter()
            .map(|p| Term::new(p.0, p.1, p.2 / total))
            .collect();
        ScoreTerms::sum(name, terms, formula_ref)
    }

    /// A score that is a sum over matched pairs: the `shown` strongest contributions `(m/z, value)` named by their m/z, the rest in one term; all of weight 1.
    pub fn over_pairs(
        name: &'static str,
        value: f64,
        mut parts: Vec<(f64, f64)>,
        shown: usize,
        formula_ref: &'static str,
    ) -> ScoreTerms {
        parts.sort_by(|x, y| y.1.total_cmp(&x.1));
        let mut terms: Vec<Term> = parts
            .iter()
            .take(shown)
            .map(|&(mz, v)| Term::new(format!("m/z {mz:.4}"), v, 1.0))
            .collect();
        if parts.len() > shown {
            let rest = parts[shown..].iter().map(|p| p.1).sum();
            terms.push(Term::new(
                format!("{} other pairs", parts.len() - shown),
                rest,
                1.0,
            ));
        }
        ScoreTerms {
            name,
            value,
            terms,
            formula_ref,
        }
    }

    /// `value` minus the weighted sum of the terms; zero up to rounding for a score that is what it says.
    pub fn residual(&self) -> f64 {
        self.value - self.terms.iter().map(|t| t.weight * t.value).sum::<f64>()
    }

    /// `{"name","value","terms":[{"name","value","weight"}],"formula_ref"}`.
    pub fn write(&self, o: &mut Out) {
        o.raw("{\"name\":")
            .str(self.name)
            .raw(",\"value\":")
            .num(self.value)
            .raw(",\"terms\":[");
        for (i, t) in self.terms.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            o.raw("{\"name\":")
                .str(&t.name)
                .raw(",\"value\":")
                .num(t.value)
                .raw(",\"weight\":")
                .num(t.weight)
                .raw("}");
        }
        o.raw("],\"formula_ref\":").str(self.formula_ref).raw("}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
