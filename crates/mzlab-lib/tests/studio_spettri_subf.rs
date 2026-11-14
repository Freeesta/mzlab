//! Spectral study, arm C (sub-formula validation as de-chimerization): the query filtered by the sub-formulas of each candidate's ion at 5 and
//! 10 ppm, with and without the kept-intensity weight, and the NNLS of `spectra.chimeric` as the reference, on E3 and E0; numbers pinned
//! (MISURA studio_subf_*). A change of the engine, the generator or an arm that moves a number updates the pin and the MISURA line together.

mod studio_spettri;

use studio_spettri::{pinned_with, subf::run_subf};

const PINS_E3: &[(&str, &str, &str, f64)] = &[
    ("entropy", "all", "recall1", 1.0),
    ("entropy", "all", "pauprc", 1.0),
    ("entropy", "all", "fdp_q01", 0.0),
    ("entropy", "all", "score", 0.7061),
    ("entropy", "eta=0.1/isobaric", "score", 0.8249),
    ("entropy", "eta=0.1/isomer", "score", 0.8075),
    ("entropy", "eta=0.6/isobaric", "score", 0.5682),
    ("entropy", "eta=0.6/isomer", "score", 0.493),
    ("sub5", "all", "recall1", 1.0),
    ("sub5", "all", "pauprc", 1.0),
    ("sub5", "all", "fdp_q01", 0.0),
    ("sub5", "all", "score", 0.8215),
    ("sub5", "eta=0.1/isobaric", "score", 0.9355),
    ("sub5", "eta=0.1/isomer", "score", 0.8445),
    ("sub5", "eta=0.6/isobaric", "score", 0.8435),
    ("sub5", "eta=0.6/isomer", "score", 0.5165),
    ("sub5", "all", "peaks.true_lost", 0.0133),
    ("sub5", "all", "peaks.int_removed", 0.3543),
    ("sub5", "all", "peaks.noise_removed", 0.8914),
    ("sub5", "eta=0.6/isobaric", "peaks.int_removed", 0.7745),
    ("sub5", "eta=0.6/isomer", "peaks.int_removed", 0.0081),
    ("sub10", "all", "recall1", 1.0),
    ("sub10", "all", "pauprc", 1.0),
    ("sub10", "all", "fdp_q01", 0.0),
    ("sub10", "all", "score", 0.8047),
    ("sub10", "eta=0.1/isobaric", "score", 0.9222),
    ("sub10", "eta=0.1/isomer", "score", 0.8445),
    ("sub10", "eta=0.6/isobaric", "score", 0.8441),
    ("sub10", "eta=0.6/isomer", "score", 0.5165),
    ("sub10", "all", "peaks.true_lost", 0.0),
    ("sub10", "all", "peaks.int_removed", 0.2872),
    ("sub10", "all", "peaks.noise_removed", 0.8525),
    ("sub10", "eta=0.6/isobaric", "peaks.int_removed", 0.6667),
    ("sub10", "eta=0.6/isomer", "peaks.int_removed", 0.0),
    ("sub5x", "all", "recall1", 1.0),
    ("sub5x", "all", "pauprc", 1.0),
    ("sub5x", "all", "fdp_q01", 0.0),
    ("sub5x", "all", "score", 0.6453),
    ("sub5x", "eta=0.1/isobaric", "score", 0.8357),
    ("sub5x", "eta=0.1/isomer", "score", 0.8242),
    ("sub5x", "eta=0.6/isobaric", "score", 0.3686),
    ("sub5x", "eta=0.6/isomer", "score", 0.5017),
    ("sub10x", "all", "recall1", 1.0),
    ("sub10x", "all", "pauprc", 1.0),
    ("sub10x", "all", "fdp_q01", 0.0),
    ("sub10x", "all", "score", 0.6961),
    ("sub10x", "eta=0.1/isobaric", "score", 0.8489),
    ("sub10x", "eta=0.1/isomer", "score", 0.8242),
    ("sub10x", "eta=0.6/isobaric", "score", 0.3686),
    ("sub10x", "eta=0.6/isomer", "score", 0.5017),
    ("nnls", "all", "recall1", 1.0),
    ("nnls", "all", "pauprc", 1.0),
    ("nnls", "all", "fdp_q01", 0.0),
    ("nnls", "all", "score", 0.6496),
    ("nnls", "eta=0.1/isobaric", "score", 0.866),
    ("nnls", "eta=0.1/isomer", "score", 0.9157),
    ("nnls", "eta=0.6/isobaric", "score", 0.4005),
    ("nnls", "eta=0.6/isomer", "score", 0.3285),
];
const PINS_E0: &[(&str, &str, &str, f64)] = &[
    ("entropy", "all", "recall1", 1.0),
    ("entropy", "all", "pauprc", 1.0),
    ("entropy", "all", "fdp_q01", 0.0),
    ("entropy", "all", "score", 0.9154),
    ("sub5", "all", "recall1", 1.0),
    ("sub5", "all", "pauprc", 1.0),
    ("sub5", "all", "fdp_q01", 0.0),
    ("sub5", "all", "score", 0.9531),
    ("sub5", "all", "peaks.true_lost", 0.0108),
    ("sub5", "all", "peaks.int_removed", 0.0),
    ("sub5", "all", "peaks.noise_removed", 0.913),
    ("sub10", "all", "recall1", 1.0),
    ("sub10", "all", "pauprc", 1.0),
    ("sub10", "all", "fdp_q01", 0.0),
    ("sub10", "all", "score", 0.9588),
    ("sub10", "all", "peaks.true_lost", 0.0),
    ("sub10", "all", "peaks.int_removed", 0.0),
    ("sub10", "all", "peaks.noise_removed", 0.8758),
    ("sub5x", "all", "recall1", 1.0),
    ("sub5x", "all", "pauprc", 1.0),
    ("sub5x", "all", "fdp_q01", 0.0),
    ("sub5x", "all", "score", 0.9348),
    ("sub10x", "all", "recall1", 1.0),
    ("sub10x", "all", "pauprc", 1.0),
    ("sub10x", "all", "fdp_q01", 0.0),
    ("sub10x", "all", "score", 0.9456),
    ("nnls", "all", "recall1", 1.0),
    ("nnls", "all", "pauprc", 1.0),
    ("nnls", "all", "fdp_q01", 0.0),
    ("nnls", "all", "score", 0.982),
];

#[test]
fn studio_subf_e3() {
    pinned_with(run_subf, "e3", 120, PINS_E3);
}

#[test]
fn studio_subf_e0() {
    pinned_with(run_subf, "e0", 120, PINS_E0);
}
