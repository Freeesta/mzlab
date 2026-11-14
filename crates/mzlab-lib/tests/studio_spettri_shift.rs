//! Spectral study, arm D (analogues with two modifications): candidates from the edge channel (inter-peak differences), scored by the overlap
//! of the differences, by a 4-shift cosine and by a 2-shift Sankoff DP on the max-likelihood arborescence, against the engine's analogue
//! search on E5 and E0; numbers pinned (MISURA studio_shift_*). A change of the engine, the generator or an arm moves pin and MISURA together.

mod studio_spettri;

use studio_spettri::{pinned_with, shift::run_shift};

const PINS_E5: &[(&str, &str, &str, f64)] = &[
    ("analogs", "all", "recall1", 0.2917),
    ("analogs", "all", "mrr", 0.2917),
    ("analogs", "all", "pauprc", 0.25),
    ("analogs", "all", "hit", 0.3),
    ("analogs", "all", "hit_q01", 0.3),
    ("analogs", "all", "score", 0.8091),
    ("analogs", "all", "hit_s90", 0.1083),
    ("analogs", "all", "recall1_s90", 0.1083),
    ("nl", "all", "recall1", 0.2667),
    ("nl", "all", "mrr", 0.2961),
    ("nl", "all", "pauprc", 0.0167),
    ("nl", "all", "hit", 0.675),
    ("nl", "all", "hit_q01", 0.6167),
    ("nl", "all", "score", 0.261),
    ("nl", "all", "hit_s90", 0.0),
    ("nl", "all", "recall1_s90", 0.0),
    ("nl", "all", "index_keys", 21.0),
    ("nl", "all", "index_peaks", 6.3),
    ("mc4", "all", "recall1", 0.5667),
    ("mc4", "all", "mrr", 0.5708),
    ("mc4", "all", "pauprc", 0.5667),
    ("mc4", "all", "hit", 0.5833),
    ("mc4", "all", "hit_q01", 0.5833),
    ("mc4", "all", "score", 0.9791),
    ("mc4", "all", "hit_s90", 0.475),
    ("mc4", "all", "recall1_s90", 0.475),
    ("sankoff", "all", "recall1", 0.3583),
    ("sankoff", "all", "mrr", 0.375),
    ("sankoff", "all", "pauprc", 0.2917),
    ("sankoff", "all", "hit", 0.4167),
    ("sankoff", "all", "hit_q01", 0.4167),
    ("sankoff", "all", "score", 0.9141),
    ("sankoff", "all", "hit_s90", 0.2333),
    ("sankoff", "all", "recall1_s90", 0.2333),
    ("sankoff", "all", "delta_wrong", 0.4),
    ("mc4_flash", "all", "recall1", 0.3583),
    ("mc4_flash", "all", "mrr", 0.3583),
    ("mc4_flash", "all", "pauprc", 0.3583),
    ("mc4_flash", "all", "hit", 0.3583),
    ("mc4_flash", "all", "hit_q01", 0.3583),
    ("mc4_flash", "all", "score", 0.9806),
    ("mc4_flash", "all", "hit_s90", 0.275),
    ("mc4_flash", "all", "recall1_s90", 0.275),
];
const PINS_E0: &[(&str, &str, &str, f64)] = &[
    ("analogs", "all", "recall1", 0.0),
    ("analogs", "all", "mrr", 0.0),
    ("analogs", "all", "pauprc", 0.0),
    ("analogs", "all", "hit", 0.0),
    ("analogs", "all", "hit_q01", 0.0),
    ("analogs", "all", "hit_s90", 0.0),
    ("analogs", "all", "recall1_s90", 0.0),
    ("nl", "all", "recall1", 0.0),
    ("nl", "all", "mrr", 0.0),
    ("nl", "all", "pauprc", 0.0),
    ("nl", "all", "hit", 0.7083),
    ("nl", "all", "hit_q01", 0.6667),
    ("nl", "all", "score", 0.2491),
    ("nl", "all", "hit_s90", 0.0),
    ("nl", "all", "recall1_s90", 0.0),
    ("nl", "all", "index_keys", 21.0),
    ("nl", "all", "index_peaks", 6.3),
    ("mc4", "all", "recall1", 0.0),
    ("mc4", "all", "mrr", 0.0),
    ("mc4", "all", "pauprc", 0.0),
    ("mc4", "all", "hit", 0.2583),
    ("mc4", "all", "hit_q01", 0.2583),
    ("mc4", "all", "score", 0.4593),
    ("mc4", "all", "hit_s90", 0.0),
    ("mc4", "all", "recall1_s90", 0.0),
    ("sankoff", "all", "recall1", 0.0),
    ("sankoff", "all", "mrr", 0.0),
    ("sankoff", "all", "pauprc", 0.0),
    ("sankoff", "all", "hit", 0.2583),
    ("sankoff", "all", "hit_q01", 0.2583),
    ("sankoff", "all", "score", 0.4515),
    ("sankoff", "all", "hit_s90", 0.0),
    ("sankoff", "all", "recall1_s90", 0.0),
    ("mc4_flash", "all", "recall1", 0.0),
    ("mc4_flash", "all", "mrr", 0.0),
    ("mc4_flash", "all", "pauprc", 0.0),
    ("mc4_flash", "all", "hit", 0.075),
    ("mc4_flash", "all", "hit_q01", 0.075),
    ("mc4_flash", "all", "score", 0.3057),
    ("mc4_flash", "all", "hit_s90", 0.0),
    ("mc4_flash", "all", "recall1_s90", 0.0),
];

#[test]
fn studio_shift_e5() {
    pinned_with(run_shift, "e5", 120, PINS_E5);
}

#[test]
fn studio_shift_e0() {
    pinned_with(run_shift, "e0", 120, PINS_E0);
}
