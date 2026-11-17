//! Spectral study, E2 (positional isomers): today's numbers of the baseline arms, pinned (`docs/agenti/studio-spettri.md`, MISURA studio_e2).
//! E2: two isomers in the library with a shared base peak (90 % of the intensity) and 2 diagnostic ions at 3 % each.
//! A change of the engine or of the generator that moves a number updates the pin and the MISURA line together.

mod studio_spettri;

const PINS: &[(&str, &str, &str, f64)] = &[
    ("entropy", "all", "recall1", 1.0),
    ("entropy", "all", "mrr", 1.0),
    ("entropy", "all", "pauprc", 1.0),
    ("entropy", "all", "ece", 0.0),
    ("entropy", "all", "fdp_q01", 0.0),
    ("entropy", "all", "gate", 0.4167),
    ("entropy", "all", "hit", 1.0),
    ("entropy", "all", "score", 0.8407),
    ("entropy", "all", "q_true", 0.0),
    ("cosine", "all", "recall1", 1.0),
    ("cosine", "all", "mrr", 1.0),
    ("cosine", "all", "pauprc", 1.0),
    ("cosine", "all", "ece", 0.0),
    ("cosine", "all", "fdp_q01", 0.0),
    ("cosine", "all", "gate", 0.4167),
    ("cosine", "all", "hit", 1.0),
    ("cosine", "all", "score", 0.9765),
    ("cosine", "all", "q_true", 0.0),
    ("modified_cosine", "all", "recall1", 1.0),
    ("modified_cosine", "all", "mrr", 1.0),
    ("modified_cosine", "all", "pauprc", 1.0),
    ("modified_cosine", "all", "ece", 0.0),
    ("modified_cosine", "all", "fdp_q01", 0.0),
    ("modified_cosine", "all", "gate", 0.4167),
    ("modified_cosine", "all", "hit", 1.0),
    ("modified_cosine", "all", "score", 0.9765),
    ("modified_cosine", "all", "q_true", 0.0),
];

#[test]
fn positional_isomers() {
    studio_spettri::pinned("e2", 120, PINS);
}
