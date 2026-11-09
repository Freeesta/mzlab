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
fn noise(i: usize) -> f64 {
    ((i * 37) % 7) as f64 * 0.5 - 1.5
}

/// Decoys among the targets (the same spectrum): how many of the 16 decoys, the fragments moved by `spostamento` Da, get a candidate, and how
/// many of the 16 targets keep theirs.
fn mixed(shift: f64) -> (usize, usize) {
    let mz = |i: usize, f: &str| ion(f) * (1.0 + (-2.5 + noise(i)) * 1e-6);
    let mut peaks: Vec<(f64, f64)> = vec![];
    for (i, f) in FRAMMENTI.iter().enumerate() {
        peaks.push((mz(i, f), 100.0 - i as f64 * 5.0));
        peaks.push((mz(i, f) + shift, 97.0 - i as f64 * 5.0));
    }
    let o = Options {
        formula: Some("C17H21N3O5+H".into()),
        ..Options::default()
    };
    let g = graph_from_peaks((ion("C17H22N3O5"), 1), ("CID", 30.0), &peaks, &o);
    let found = |(m, _): &(f64, f64)| {
        g.nodi
            .iter()
            .any(|n| (n.mz - m).abs() < 1e-4 && n.formula.is_some())
    };
    let bersagli = peaks.iter().step_by(2).filter(|p| found(p)).count();
    let decoys = peaks.iter().skip(1).step_by(2).filter(|p| found(p)).count();
    (bersagli, decoys)
}

/// The co-isolated infusion (a root whose MS2 holds a strong fragment A (C4H7N2) and an ion P (C5H6N3O) that is isolated for an MS3, where A appears
/// too although it is no sub-formula of P (a co-isolated ion): A is under the root), with invented peaks `esche` added to every spectrum.
fn coisolated_graph(decoys: &[f64]) -> Graph {
    use crate::msntree_build::{albero_di, Grouping};
    let step = |v: &[(f64, &str, f64)]| -> Vec<(f64, String, f64)> {
        v.iter().map(|&(m, a, c)| (m, a.to_string(), c)).collect()
    };
    let paths = [
        step(&[(195.0, "CID", 30.0)]),
        step(&[(195.0, "CID", 30.0), (124.0, "CID", 30.0)]),
    ];
    let sp = |p: &[(f64, f64)]| Spettro {
        mz: p.iter().map(|x| x.0).collect(),
        rel: p.iter().map(|x| x.1).collect(),
    };
    let with_decoys = |mut v: Vec<(f64, f64)>| {
        v.extend(decoys.iter().map(|&m| (m, 25.0)));
        v
    };
    let ms2 = sp(&with_decoys(vec![
        (ion("C8H11N4O2"), 20.0),
        (ion("C4H7N2"), 100.0),
        (ion("C5H6N3O"), 30.0),
    ]));
    let ms3 = sp(&with_decoys(vec![
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
    let al = albero_di(scans, &Grouping::default());
    let per_node: Vec<Spettro> = al
        .nodi
        .iter()
        .map(|n| match n.id.as_str() {
            "195@cid30" => ms2.clone(),
            _ => ms3.clone(),
        })
        .collect();
    let o = Options {
        formula: Some("C8H10N4O2+H".into()),
        ..Options::default()
    };
    build(
        &Input {
            albero: &al,
            ms1: Some(&ms1),
            spettri: &per_node,
            widths: &vec![None; al.nodi.len()],
            polarita: 1,
        },
        &o,
    )
}

#[test]
fn invented_peaks_get_no_formula_where_a_co_isolated_fragment_keeps_it() {
    // half-integer defects (the sub-formulas of a CHNO root of this size lie within -0.1..+0.25 of the integer): no formula can fit them
    let decoys = [97.45, 110.48, 123.41, 136.46, 149.43];
    let g = coisolated_graph(&decoys);
    let con = |m: f64| {
        g.nodi
            .iter()
            .filter(|n| (n.mz - m).abs() < 1e-4)
            .any(|n| n.formula.as_ref().is_some_and(|f| !f.candidates.is_empty()))
    };
    assert!(con(ion("C4H7N2")), "the real fragment keeps its formula");
    assert_eq!(decoys.iter().filter(|&&m| con(m)).count(), 0);
}

#[test]
fn decoys_mixed_with_the_fragments_leave_most_of_them_without_a_formula() {
    // measured on a drug-sized root: 10 ppm alone lets 28 of the 80 decoys (shifts of 0.02-0.11 Da) hold a candidate; with the recalibration 18 do, and the 16 targets all keep theirs
    let mut decoys = 0;
    for d in [0.02, 0.03, 0.05, 0.07, 0.11] {
        let (bersagli, e) = mixed(d);
        assert_eq!(bersagli, 16, "targets at {d}");
        decoys += e;
    }
    assert!((10..=20).contains(&decoys), "{decoys} of 80 decoys");
}

/// Sub-formulas of a sulfur-bearing root (C22H31N6O7S, m/z of the ion) that are even-electron ions: a denser lattice than `FRAMMENTI`.
fn bersagli_s(n: usize) -> Vec<String> {
    let mut v = vec![];
    for c in 4..=22 {
        for h in 4..=31 {
            for nn in 0..=6 {
                for o in 0..=7 {
                    for s in 0..=1 {
                        let dbe2 = 2 * c + 2 + nn - h - 1;
                        if 2 + 2 * (22 - c) + (6 - nn) - (31 - h) < 0
                            || dbe2 < 1
                            || dbe2 % 2 != 0
                            || ion_s(c, h, nn, o, s) < 150.0
                        {
                            continue;
                        }
                        v.push(format!("C{c}H{h}N{nn}O{o}S{s}"));
                    }
                }
            }
        }
    }
    let mut x: u64 = 12345;
    let mut out = vec![];
    for _ in 0..n {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        out.push(v[(x >> 33) as usize % v.len()].clone());
    }
    out
}
fn ion_s(c: i32, h: i32, n: i32, o: i32, s: i32) -> f64 {
    12.0 * c as f64
        + 1.00782503207 * h as f64
        + 14.0030740048 * n as f64
        + 15.99491461956 * o as f64
        + 31.97207100 * s as f64
        - 0.00054858
}

/// (targets, decoys) among `n` targets of the sulfur root and their decoys moved by `d` Da, in one spectrum, that keep a formula candidate.
fn mixed_s(n: usize, d: f64) -> (usize, usize) {
    let ts = bersagli_s(n);
    let sc = |i: usize| (((i * 37) % 7) as f64 * 0.5 - 1.5) * 0.3e-6;
    let mut peaks = vec![];
    for (i, f) in ts.iter().enumerate() {
        let f = f.replace("S0", "").replace("O0", "").replace("N0", "");
        let m = ion(&f) * (1.0 - 2.8e-6 + sc(i));
        peaks.push((m, 100.0 - i as f64));
        peaks.push((m + d, 99.5 - i as f64));
    }
    let o = Options {
        formula: Some("C22H30N6O7S+H".into()),
        ..Options::default()
    };
    let g = graph_from_peaks((ion("C22H31N6O7S"), 1), ("CID", 30.0), &peaks, &o);
    let found = |(m, _): &(f64, f64)| {
        g.nodi.iter().any(|n| {
            (n.mz - m).abs() < 1e-4 && n.formula.as_ref().is_some_and(|f| !f.candidates.is_empty())
        })
    };
    (
        peaks.iter().step_by(2).filter(|p| found(p)).count(),
        peaks.iter().skip(1).step_by(2).filter(|p| found(p)).count(),
    )
}

#[test]
fn decoys_among_the_targets_of_a_dense_lattice_are_fewer_with_the_recalibration_window_read_from_the_cluster(
) {
    // a root of m/z ~490 with sulfur: at 10 ppm more than half of the decoys fit a sub-formula (the decoys of 0.03-0.04 Da are near the
    // CH4/O swap, 0.0364 Da, and fit exactly); the window read from the densest 40 % of the values and floored at 2 ppm leaves 92 of 176
    // (108 with the floor at 3 ppm and the quartile), and every target keeps its formula
    let (mut bersagli, mut decoys) = (0, 0);
    for d in [0.02, 0.03, 0.04, 0.05] {
        let (b, e) = mixed_s(44, d);
        bersagli += b;
        decoys += e;
    }
    assert!(bersagli >= 175, "{bersagli} of 176 targets");
    assert!(decoys <= 95, "{decoys} of 176 decoys");
}
