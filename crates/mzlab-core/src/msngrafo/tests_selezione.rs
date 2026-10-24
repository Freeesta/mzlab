//! Selectivity of the formulas on a drug-sized root: target-decoy counts of the nodes with a candidate and with one chosen formula.

use super::tests::ion;
use super::*;

/// Fragments of a drug-sized root, as sub-formulas: the targets of the target-decoy tests (the decoys are the same m/z moved by +0.02 Da).
const FRAMMENTI: [&str; 16] = [
    "C17H20N3O4",
    "C16H18N3O4",
    "C15H18N3O3",
    "C14H16N3O3",
    "C13H14N3O2",
    "C12H12N3O2",
    "C10H10NO3",
    "C9H8NO2",
    "C8H10N3O",
    "C7H8N2O",
    "C6H6NO",
    "C5H8N3",
    "C11H12NO2",
    "C13H16N2O3",
    "C12H15N2O2",
    "C10H13N2O",
];

/// (nodes with a candidate, nodes with one chosen formula) of the graph of one MS2 whose fragments are moved by `spostamento` Da.
fn conteggi(spostamento: f64) -> (usize, usize) {
    let peaks: Vec<(f64, f64)> = FRAMMENTI
        .iter()
        .enumerate()
        .map(|(i, f)| {
            (
                ion(f) * (1.0 - 2.5e-6) + spostamento,
                100.0 - i as f64 * 5.0,
            )
        })
        .collect();
    let o = Opzioni {
        formula: Some("C17H21N3O5+H".into()),
        ..Opzioni::default()
    };
    let g = grafo_da_picchi((ion("C17H22N3O5"), 1), ("CID", 30.0), &peaks, &o);
    let con = |n: &&NodoG| n.formula.as_ref().is_some_and(|f| !f.candidati.is_empty());
    let unici = |n: &&NodoG| n.formula.as_ref().is_some_and(|f| f.scelta.is_some());
    (
        g.nodi.iter().filter(con).count(),
        g.nodi.iter().filter(unici).count(),
    )
}

#[test]
fn an_even_electron_candidate_is_preferred_to_a_radical_one_without_evidence() {
    // the root and the 16 fragments: every node has exactly one formula, the radical alternatives of the low masses are gone
    assert_eq!(conteggi(0.0), (17, 17));
}

#[test]
fn decoys_do_not_gain_a_choice_from_the_preference() {
    // a decoy that matches a sub-formula is kept (the window is 10 ppm) but a radical candidate never adds a second one
    let (con, unici) = conteggi(0.02);
    assert!(con < 17 && con == unici, "{con} {unici}");
}
