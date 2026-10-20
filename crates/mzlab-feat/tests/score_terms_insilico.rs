//! The in-silico score (`fragmenter_score`) opened in its terms: the share of each explained peak. They add up to the score to 1e-12 on every molecule of the
//! reference set, the score does not change with the option, and without it the ranking carries no record.

use mzlab_feat::attributio::ms2_engine::insilico::{
    fragmenter_contributions, fragmenter_score, rank, read_candidates, InSilicoOpts,
};
use mzlab_feat::attributio::review::fragmenter::fragment_molecule;
use serde_json::Value;

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

#[test]
fn the_shares_of_the_explained_peaks_add_up_to_the_score() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/attributio/ms2_engine/insilico.json"
    );
    let g: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let mut n = 0;
    for m in g["molecole"].as_array().unwrap() {
        let (peaks, ints) = (nums(&m["peaks"]), nums(&m["intensities"]));
        let pairs: Vec<(f64, f64)> = peaks.iter().cloned().zip(ints).collect();
        let fr = fragment_molecule(
            m["smiles"].as_str().unwrap(),
            m["pol"].as_str().unwrap(),
            &peaks,
            10.0,
            200,
        )
        .unwrap();
        let got: Vec<f64> = fr.explained.iter().map(|e| e.peak).collect();
        let sum: f64 = fragmenter_contributions(&pairs, &got)
            .iter()
            .map(|c| c.1)
            .sum();
        assert!(
            (sum - fragmenter_score(&pairs, &got)).abs() < 1e-12,
            "{}",
            m["smiles"]
        );
        n += 1;
    }
    assert!(n > 10, "the reference set is there");
}

#[test]
fn the_ranking_carries_the_record_only_when_asked_and_the_score_is_the_same() {
    let list = "Identifier,CompoundName,SMILES,MolecularFormula\n1,Caffeine,Cn1cnc2c1c(=O)n(C)c(=O)n2C,C8H10N4O2\n";
    let cands = read_candidates(list);
    let peaks = [(138.0662, 100.0), (110.0713, 40.0), (42.0, 5.0)];
    let off = rank(195.0877, "POS", &peaks, &cands, &InSilicoOpts::default());
    let on = rank(
        195.0877,
        "POS",
        &peaks,
        &cands,
        &InSilicoOpts {
            terms: true,
            ..InSilicoOpts::default()
        },
    );
    assert!(off[0].terms.is_none());
    assert_eq!(off[0].score, on[0].score);
    let t = on[0].terms.as_ref().unwrap();
    assert!(t.residual().abs() < 1e-12);
    assert_eq!(t.formula_ref, "score/fragmenter");
    assert!((t.value - on[0].score).abs() < 1e-15);
}
