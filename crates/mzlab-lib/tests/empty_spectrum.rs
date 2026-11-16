//! A spectrum with no intensity (no peaks, or every peak at 0) has entropy similarity 0 with any spectrum, as in the ms_entropy reference
//! and as the sum of the pair contributions says; the bare formula gives 1 - ln 2 / ln 4 = 0.5.

use mzlab_lib::contrib::entropy_contributions;
use mzlab_lib::scores::{entropy_from_pairs, pair_scores, Spec};

fn full() -> Spec {
    Spec {
        mz: vec![100.0, 150.0, 200.0],
        it: vec![10.0, 50.0, 100.0],
        prec: 250.0,
    }
}

fn empties() -> [Spec; 2] {
    [
        Spec {
            mz: vec![],
            it: vec![],
            prec: 250.0,
        },
        Spec {
            mz: vec![120.0, 180.0],
            it: vec![0.0, 0.0],
            prec: 250.0,
        },
    ]
}

#[test]
fn an_empty_spectrum_scores_zero_on_every_route() {
    let a = full();
    for e in empties() {
        for (q, l) in [(&a, &e), (&e, &a), (&e, &e)] {
            let r = pair_scores(q, l, 0.02);
            assert_eq!(r.entropy, 0.0, "pair_scores {:?} against {:?}", q.it, l.it);
            assert_eq!(entropy_from_pairs(&q.it, &l.it, &r.pairs), 0.0);
            let sum: f64 = entropy_contributions(q, l, &r.pairs)
                .iter()
                .map(|c| c.1)
                .sum();
            assert_eq!(sum, 0.0);
        }
    }
}
