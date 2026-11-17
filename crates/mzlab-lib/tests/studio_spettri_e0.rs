//! Spectral study, E0 (clean replicate): today's numbers of the baseline arms, pinned (`docs/agenti/studio-spettri.md`, MISURA studio_e0).
//! E0: every query is a new measurement (CE 35, fresh noise) of a library entry.
//! A change of the engine or of the generator that moves a number updates the pin and the MISURA line together.

mod studio_spettri;

const PINS: &[(&str, &str, &str, f64)] = &[
    ("entropy", "all", "recall1", 1.0),
    ("entropy", "all", "mrr", 1.0),
    ("entropy", "all", "pauprc", 1.0),
    ("entropy", "all", "ece", 0.0),
    ("entropy", "all", "fdp_q01", 0.0),
    ("entropy", "all", "gate", 0.525),
    ("entropy", "all", "hit", 1.0),
    ("entropy", "all", "score", 0.9154),
    ("entropy", "all", "q_true", 0.0),
    ("cosine", "all", "recall1", 1.0),
    ("cosine", "all", "mrr", 1.0),
    ("cosine", "all", "pauprc", 1.0),
    ("cosine", "all", "ece", 0.0),
    ("cosine", "all", "fdp_q01", 0.0),
    ("cosine", "all", "gate", 0.525),
    ("cosine", "all", "hit", 1.0),
    ("cosine", "all", "score", 0.9847),
    ("cosine", "all", "q_true", 0.0),
    ("modified_cosine", "all", "recall1", 1.0),
    ("modified_cosine", "all", "mrr", 1.0),
    ("modified_cosine", "all", "pauprc", 1.0),
    ("modified_cosine", "all", "ece", 0.0),
    ("modified_cosine", "all", "fdp_q01", 0.0),
    ("modified_cosine", "all", "gate", 0.525),
    ("modified_cosine", "all", "hit", 1.0),
    ("modified_cosine", "all", "score", 0.9847),
    ("modified_cosine", "all", "q_true", 0.0),
    ("analogs", "all", "recall1", 0.0),
    ("analogs", "all", "mrr", 0.0),
    ("analogs", "all", "pauprc", 0.0),
    ("analogs", "all", "ece", 0.0),
    ("analogs", "all", "fdp_q01", 0.0),
    ("analogs", "all", "gate", 0.0),
    ("analogs", "all", "hit", 0.0),
    ("analogs", "all", "q_true", 1.0),
];

#[test]
fn clean_replicates() {
    studio_spettri::pinned("e0", 120, PINS);
}
