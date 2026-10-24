//! Target-decoy count of the formula annotation on a drug-sized root: decoys mixed with the real fragments in the same MS2 must mostly stay
//! without a formula, the targets must all keep theirs (protocol of CRITERI.md §5c).

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

/// Scatter of the measurement around the systematic error, ppm (deterministic, within ±1.5).
fn rumore(i: usize) -> f64 {
    ((i * 37) % 7) as f64 * 0.5 - 1.5
}

/// Decoys among the targets (the same spectrum): how many of the 16 decoys, the fragments moved by `spostamento` Da, get a candidate, and how
/// many of the 16 targets keep theirs.
fn misto(spostamento: f64) -> (usize, usize) {
    let mz = |i: usize, f: &str| ion(f) * (1.0 + (-2.5 + rumore(i)) * 1e-6);
    let mut peaks: Vec<(f64, f64)> = vec![];
    for (i, f) in FRAMMENTI.iter().enumerate() {
        peaks.push((mz(i, f), 100.0 - i as f64 * 5.0));
        peaks.push((mz(i, f) + spostamento, 97.0 - i as f64 * 5.0));
    }
    let o = Opzioni {
        formula: Some("C17H21N3O5+H".into()),
        ..Opzioni::default()
    };
    let g = grafo_da_picchi((ion("C17H22N3O5"), 1), ("CID", 30.0), &peaks, &o);
    let trovati = |(m, _): &(f64, f64)| {
        g.nodi
            .iter()
            .any(|n| (n.mz - m).abs() < 1e-4 && n.formula.is_some())
    };
    let bersagli = peaks.iter().step_by(2).filter(|p| trovati(p)).count();
    let esche = peaks
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|p| trovati(p))
        .count();
    (bersagli, esche)
}

/// The co-isolated infusion (a root whose MS2 holds a strong fragment A (C4H7N2) and an ion P (C5H6N3O) that is isolated for an MS3, where A appears
/// too although it is no sub-formula of P (a co-isolated ion): A is under the root), with invented peaks `esche` added to every spectrum.
fn grafo_coisolato(esche: &[f64]) -> Grafo {
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
    let con_esche = |mut v: Vec<(f64, f64)>| {
        v.extend(esche.iter().map(|&m| (m, 25.0)));
        v
    };
    let ms2 = sp(&con_esche(vec![
        (ion("C8H11N4O2"), 20.0),
        (ion("C4H7N2"), 100.0),
        (ion("C5H6N3O"), 30.0),
    ]));
    let ms3 = sp(&con_esche(vec![
        (ion("C5H6N3O"), 100.0),
        (ion("C4H7N2"), 40.0),
    ]));
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
fn invented_peaks_get_no_formula_where_a_co_isolated_fragment_keeps_it() {
    // half-integer defects (the sub-formulas of a CHNO root of this size lie within -0.1..+0.25 of the integer): no formula can fit them
    let esche = [97.45, 110.48, 123.41, 136.46, 149.43];
    let g = grafo_coisolato(&esche);
    let con = |m: f64| {
        g.nodi
            .iter()
            .filter(|n| (n.mz - m).abs() < 1e-4)
            .any(|n| n.formula.as_ref().is_some_and(|f| !f.candidati.is_empty()))
    };
    assert!(con(ion("C4H7N2")), "the real fragment keeps its formula");
    assert_eq!(esche.iter().filter(|&&m| con(m)).count(), 0);
}

#[test]
fn decoys_mixed_with_the_fragments_leave_most_of_them_without_a_formula() {
    // measured on a drug-sized root: 10 ppm alone lets 28 of the 80 decoys (shifts of 0.02-0.11 Da) hold a candidate; with the recalibration 18 do, and the 16 targets all keep theirs
    let mut esche = 0;
    for d in [0.02, 0.03, 0.05, 0.07, 0.11] {
        let (bersagli, e) = misto(d);
        assert_eq!(bersagli, 16, "targets at {d}");
        esche += e;
    }
    assert!((10..=20).contains(&esche), "{esche} of 80 decoys");
}
