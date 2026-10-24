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

/// The infusion of a root whose MS2 holds a strong fragment A (C4H7N2) and an ion P (C5H6N3O) that is isolated for an MS3, where A appears
/// too although it is no sub-formula of P (a co-isolated ion): A must keep its formula, it is under the root.
fn grafo_coisolato() -> Grafo {
    use crate::msnalbero::{albero_di, Raggruppa};
    let passo = |v: &[(f64, &str, f64)]| -> Vec<(f64, String, f64)> {
        v.iter().map(|&(m, a, c)| (m, a.to_string(), c)).collect()
    };
    let paths = [
        passo(&[(195.0, "CID", 30.0)]),
        passo(&[(195.0, "CID", 30.0), (124.0, "CID", 30.0)]),
    ];
    let sp = |p: &[(f64, f64)]| Spettro {
        mz: p.iter().map(|x| x.0).collect(),
        rel: p.iter().map(|x| x.1).collect(),
    };
    let ms2 = sp(&[
        (ion("C8H11N4O2"), 20.0),
        (ion("C4H7N2"), 100.0),
        (ion("C5H6N3O"), 30.0),
    ]);
    let ms3 = sp(&[(ion("C5H6N3O"), 100.0), (ion("C4H7N2"), 40.0)]);
    let ms1 = sp(&[(ion("C8H11N4O2"), 100.0)]);
    let mut scans = vec![(1u8, None)];
    for p in &paths {
        for _ in 0..3 {
            scans.push(((p.len() + 1) as u8, Some(&p[..])));
        }
    }
    let al = albero_di(scans, &Raggruppa::default());
    let per_nodo: Vec<Spettro> = al
        .nodi
        .iter()
        .map(|n| match n.id.as_str() {
            "195@cid30" => ms2.clone(),
            _ => ms3.clone(),
        })
        .collect();
    let o = Opzioni {
        formula: Some("C8H10N4O2+H".into()),
        ..Opzioni::default()
    };
    costruisci(
        &Ingresso {
            albero: &al,
            ms1: Some(&ms1),
            spettri: &per_nodo,
            larghezze: &vec![None; al.nodi.len()],
            polarita: 1,
        },
        &o,
    )
}

#[test]
fn a_fragment_under_the_root_survives_a_precursor_it_is_not_under() {
    let g = grafo_coisolato();
    let m = ion("C4H7N2");
    let n = g.nodi.iter().find(|n| (n.mz - m).abs() < 1e-4);
    let f = n.and_then(|n| n.formula.as_ref());
    assert_eq!(
        f.and_then(|f| f.scelta.as_deref()),
        Some("C4H7N2"),
        "{:?}",
        g.nodi.iter().map(|n| n.mz).collect::<Vec<_>>()
    );
}
