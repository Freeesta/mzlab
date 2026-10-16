//! Tests of `nomi.rs`: tabular cases of net change, modification and TP names.

use super::*;

const P: &str = "C12H17N3O3S";

#[test]
fn net_change_cases() {
    for (f, want) in [
        (P, "\u{2014}"),
        ("C12H17N3O4S", "+O"),
        ("C10H14N3O6S", "\u{2212}C2H3 +O3"),
        ("C12H15N3O3S", "\u{2212}H2"),
        ("C8H9N3O3", "\u{2212}C4H8S"),
        ("C12H18N3O3SCl", "+HCl"),
        ("C14H20N3O3S2Cl", "+C2H3ClS"),
        ("", ""),
    ] {
        assert_eq!(net_change(f, P), want, "{f}");
    }
}

#[test]
fn modification_cases() {
    for (n, u, want) in [
        ("progenitore", false, "\u{2014}"),
        ("Oxidation_to_carbonyl", false, "Oxidation to carbonyl"),
        (
            "oxidation to carbonyl @ 13.57 min",
            false,
            "Oxidation to carbonyl",
        ),
        ("loss of C4H8 + loss of SO2", false, "Loss of C4H8 and SO2"),
        (
            "hydroxylation + loss of propene (N-deisopropylation)",
            false,
            "Hydroxylation + loss of propene (N-deisopropylation)",
        ),
        ("unexpected ion m/z 301.20", true, ""),
    ] {
        assert_eq!(modification(n, u), want, "{n}");
    }
}

fn e(id: i64, mz: f64, f: &str, rt: Option<f64>, label: &'static str) -> Entry {
    Entry {
        id,
        mz,
        formula: f.into(),
        ref_rt: rt,
        label,
        ..Default::default()
    }
}

fn nm(es: &[Entry]) -> Vec<String> {
    tp_names(es, &vec![true; es.len()])
}

#[test]
fn searched_not_detected_gets_no_tp_name() {
    let es = [
        e(1, 362.09, "A", Some(5.0), "forte"),
        e(2, 362.09, "A", None, "debole"),
        e(3, 362.31, "B", Some(6.0), "forte"),
    ];
    assert_eq!(
        tp_names(&es, &[true, false, true]),
        ["TP 362_09", "Candidate", "TP 362_31"]
    );
    assert_eq!(tp_names(&es[..2], &[true, false]), ["TP 362", "Candidate"]);
}

#[test]
fn outputs_are_english_text() {
    // Tabui writes English only: the words of the columns carry nothing but ASCII, the minus and the dash
    for t in [
        net_change("C10H14N3O6S", P),
        modification("loss of propene (N-deisopropylation)", false),
        modification("progenitore", false),
    ] {
        assert!(
            t.chars()
                .all(|c| c.is_ascii() || c == '\u{2212}' || c == '\u{2014}'),
            "{t}"
        );
    }
}

#[test]
fn names() {
    let n = tp_names(
        &[
            e(0, 362.1, P, Some(5.0), "progenitore"),
            e(1, 378.2, "X", None, "forte"),
        ],
        &[true, true],
    );
    assert_eq!(n, ["Parent", "TP 378"]);
    let n = nm(&[
        e(3, 362.0912, "A", Some(7.2), "forte"),
        e(1, 362.0912, "A", Some(5.1), "forte"),
        e(2, 362.0912, "A", Some(6.0), "forte"),
    ]);
    assert_eq!(n, ["TP 362 C", "TP 362 A", "TP 362 B"]);
    let n = nm(&[
        e(1, 362.3112, "A", Some(5.0), "forte"),
        e(2, 362.2234, "B", Some(6.0), "forte"),
    ]);
    assert_eq!(n, ["TP 362_31", "TP 362_22"]);
    let n = nm(&[
        e(1, 362.3112, "A", Some(5.0), "forte"),
        e(2, 362.3112, "A", Some(6.0), "forte"),
        e(3, 362.3140, "B", Some(7.0), "forte"),
    ]);
    assert_eq!(n, ["TP 362_311 A", "TP 362_311 B", "TP 362_314"]);
    assert_eq!(nm(&[e(1, 362.51, "A", None, "forte")]), ["TP 363"]);
}
