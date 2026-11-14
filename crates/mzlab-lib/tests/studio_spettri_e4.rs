//! Spectral study, E4 (collision energy): today's numbers of the baseline arms, pinned (`docs/agenti/studio-spettri.md`, MISURA studio_e4).
//! E4: queries at CE 10/20/50/65 against the library at CE 35 (the kinetic model moves the branching ratios).
//! A change of the engine or of the generator that moves a number updates the pin and the MISURA line together.

mod studio_spettri;

const PINS: &[(&str, &str, &str, f64)] = &[
    ("entropy", "all", "recall1", 1.0),
    ("entropy", "all", "mrr", 1.0),
    ("entropy", "all", "pauprc", 1.0),
    ("entropy", "all", "ece", 0.0),
    ("entropy", "all", "fdp_q01", 0.0),
    ("entropy", "all", "gate", 0.3),
    ("entropy", "all", "hit", 1.0),
    ("entropy", "all", "score", 0.8326),
    ("entropy", "all", "q_true", 0.0),
    ("entropy", "ce=10", "recall1", 1.0),
    ("entropy", "ce=10", "score", 0.7119),
    ("entropy", "ce=10", "gate", 0.1333),
    ("entropy", "ce=10", "q_true", 0.0),
    ("entropy", "ce=20", "recall1", 1.0),
    ("entropy", "ce=20", "score", 0.8611),
    ("entropy", "ce=20", "gate", 0.3333),
    ("entropy", "ce=20", "q_true", 0.0),
    ("entropy", "ce=50", "recall1", 1.0),
    ("entropy", "ce=50", "score", 0.8872),
    ("entropy", "ce=50", "gate", 0.3),
    ("entropy", "ce=50", "q_true", 0.0),
    ("entropy", "ce=65", "recall1", 1.0),
    ("entropy", "ce=65", "score", 0.8636),
    ("entropy", "ce=65", "gate", 0.4333),
    ("entropy", "ce=65", "q_true", 0.0),
    ("cosine", "all", "recall1", 1.0),
    ("cosine", "all", "mrr", 1.0),
    ("cosine", "all", "pauprc", 1.0),
    ("cosine", "all", "ece", 0.0),
    ("cosine", "all", "fdp_q01", 0.0),
    ("cosine", "all", "gate", 0.3),
    ("cosine", "all", "hit", 1.0),
    ("cosine", "all", "score", 0.9577),
    ("cosine", "all", "q_true", 0.0),
    ("modified_cosine", "all", "recall1", 1.0),
    ("modified_cosine", "all", "mrr", 1.0),
    ("modified_cosine", "all", "pauprc", 1.0),
    ("modified_cosine", "all", "ece", 0.0),
    ("modified_cosine", "all", "fdp_q01", 0.0),
    ("modified_cosine", "all", "gate", 0.3),
    ("modified_cosine", "all", "hit", 1.0),
    ("modified_cosine", "all", "score", 0.9577),
    ("modified_cosine", "all", "q_true", 0.0),
];

#[test]
fn collision_energy() {
    studio_spettri::pinned("e4", 120, PINS);
}
