//! `spectra.match` decoys with the cosine scores (the entropy ones are in `flow_ident_tests`).

use crate::flow_ident_tests::{hits, is_null, mix_fixture, run};

/// Cosine and modified cosine count the decoys with their own score: q-values on the best hit, no decoy listed, and the decoy of a query never beats
/// the unit score of an exact match.
#[test]
fn decoys_work_with_the_cosine_scores() {
    let dir = mix_fixture("decoy_cos");
    for alg in ["cosine", "modified_cosine"] {
        run(&dir, alg, r#"{"decoys":"spectrum"}"#);
        let h = hits(&dir);
        assert_eq!(h.get("decoys").unwrap().str(), Some("spectrum"), "{alg}");
        let list = h.get("hits").unwrap().arr();
        assert!(list
            .iter()
            .all(|x| !x.get("id").unwrap().str().unwrap().starts_with("DECOY:")));
        let with_q: Vec<_> = list.iter().filter(|x| !is_null(x, "decoy_q")).collect();
        assert!(!with_q.is_empty(), "{alg}");
        for x in with_q {
            let q = x.get("decoy_q").unwrap().f64().unwrap();
            assert!((0.0..=1.0).contains(&q), "{alg} {q}");
            assert!(x.get("best_decoy").unwrap().f64().unwrap() <= 1.0);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
