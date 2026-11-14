//! Spectral study, E5 (two-shift analogues): today's numbers of the baseline arms, pinned (`docs/agenti/studio-spettri.md`, MISURA studio_e5).
//! E5: analogues with +O and -Cl+H (or -CH2) on disjoint subtrees; the modified compound is absent, its parent is the truth.
//! A change of the engine or of the generator that moves a number updates the pin and the MISURA line together.

mod studio_spettri;

const PINS: &[(&str, &str, &str, f64)] = &[
    ("analogs", "all", "recall1", 0.2917),
    ("analogs", "all", "mrr", 0.2917),
    ("analogs", "all", "pauprc", 0.25),
    ("analogs", "all", "ece", 0.0278),
    ("analogs", "all", "fdp_q01", 0.0278),
    ("analogs", "all", "gate", 0.1083),
    ("analogs", "all", "hit", 0.3),
    ("analogs", "all", "score", 0.8091),
    ("analogs", "all", "q_true", 1.0),
    ("analogs", "e5/-CH2", "recall1", 0.2946),
    ("analogs", "e5/-CH2", "score", 0.8383),
    ("analogs", "e5/-CH2", "gate", 0.1161),
    ("analogs", "e5/-CH2", "q_true", 1.0),
    ("analogs", "e5/-Cl+H", "recall1", 0.25),
    ("analogs", "e5/-Cl+H", "score", 0.8091),
    ("analogs", "e5/-Cl+H", "gate", 0.0),
    ("analogs", "e5/-Cl+H", "q_true", 1.0),
];

#[test]
fn two_shift_analogues() {
    studio_spettri::pinned("e5", 120, PINS);
}
