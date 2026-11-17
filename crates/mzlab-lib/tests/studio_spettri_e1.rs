//! Spectral study, E1 (small compounds): today's numbers of the baseline arms, pinned (`docs/agenti/studio-spettri.md`, MISURA studio_e1).
//! E1: compounds below 200 Da with 2-3 fragments, the library entry with 5 more peaks at 1-2 %, two isobaric (same formula)
//! competitors in the precursor window.
//! A change of the engine or of the generator that moves a number updates the pin and the MISURA line together.

mod studio_spettri;

const PINS: &[(&str, &str, &str, f64)] = &[
    ("entropy", "all", "recall1", 0.9833),
    ("entropy", "all", "mrr", 0.9917),
    ("entropy", "all", "pauprc", 0.6),
    ("entropy", "all", "ece", 0.0167),
    ("entropy", "all", "fdp_q01", 0.0167),
    ("entropy", "all", "gate", 0.0),
    ("entropy", "all", "hit", 1.0),
    ("entropy", "all", "score", 0.7412),
    ("entropy", "all", "q_true", 0.0),
    ("cosine", "all", "recall1", 0.9917),
    ("cosine", "all", "mrr", 0.9958),
    ("cosine", "all", "pauprc", 0.9843),
    ("cosine", "all", "ece", 0.0083),
    ("cosine", "all", "fdp_q01", 0.0083),
    ("cosine", "all", "gate", 0.0),
    ("cosine", "all", "hit", 1.0),
    ("cosine", "all", "score", 0.9558),
    ("cosine", "all", "q_true", 0.0),
    ("modified_cosine", "all", "recall1", 0.9917),
    ("modified_cosine", "all", "mrr", 0.9958),
    ("modified_cosine", "all", "pauprc", 0.9843),
    ("modified_cosine", "all", "ece", 0.0083),
    ("modified_cosine", "all", "fdp_q01", 0.0083),
    ("modified_cosine", "all", "gate", 0.0),
    ("modified_cosine", "all", "hit", 1.0),
    ("modified_cosine", "all", "score", 0.9558),
    ("modified_cosine", "all", "q_true", 0.0),
];

#[test]
fn small_compounds() {
    studio_spettri::pinned("e1", 120, PINS);
}
