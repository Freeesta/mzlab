//! `frag/1` and `parz/1` honour `M  CHG` and `M  ISO` of the piece: a side with a formal charge is the ion itself (m/z = mass - e, no extra H+),
//! a neutral side is [F+H]+, an isotope label stays in the formula and in the mass.

use mzlab_feat::chem::frammenti::{fragments_json, partial_json};
use serde_json::Value;

/// V2000 molfile of atoms `els` with bonds `(a, b, type)` (1-based) and property lines `props`.
fn molfile(els: &[&str], bonds: &[(usize, usize, u8)], props: &[&str]) -> String {
    let mut s = format!(
        "\n  qqq\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
        els.len(),
        bonds.len()
    );
    for (i, e) in els.iter().enumerate() {
        s += &format!(
            "{:>10.4}{:>10.4}{:>10.4} {:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n",
            i as f64, 0.0, 0.0, e
        );
    }
    for (a, b, o) in bonds {
        s += &format!("{a:>3}{b:>3}{o:>3}  0  0  0  0\n");
    }
    for p in props {
        s += &format!("{p}\n");
    }
    s + "M  END"
}

#[test]
fn charged_side_is_the_cation_itself() {
    let mf = molfile(&["C", "N"], &[(1, 2, 1)], &["M  CHG  1   2   1"]);
    let j = fragments_json(&mf, 0, 1).unwrap();
    assert!(
        j.contains("\"formula\":\"H4N+\"") && j.contains("\"mz\":18.0338"),
        "{j}"
    );
    assert!(
        j.contains("\"formula\":\"CH4\"") && j.contains("\"mz\":17.0386"),
        "{j}"
    );
}

#[test]
fn isotope_label_stays_in_the_piece() {
    let mf = molfile(
        &["C", "C", "O", "O"],
        &[(1, 2, 1), (2, 3, 2), (2, 4, 1)],
        &["M  ISO  1   1  13"],
    );
    let j = fragments_json(&mf, 0, 1).unwrap();
    assert!(
        j.contains("\"formula\":\"13CH4\"") && j.contains("\"mass\":17.0347"),
        "{j}"
    );
}

#[test]
fn protonated_caffeine_has_no_fragment_above_the_ion() {
    // [M+H]+ of caffeine drawn with the charge on N9: C8H11N4O2+, m/z 195.0877
    let els = [
        "C", "N", "C", "N", "C", "C", "C", "O", "N", "C", "C", "O", "N", "C",
    ];
    let bonds = [
        (1, 2, 1),
        (2, 3, 4),
        (3, 4, 4),
        (4, 5, 4),
        (5, 6, 4),
        (6, 2, 4),
        (6, 7, 4),
        (7, 8, 2),
        (7, 9, 4),
        (9, 10, 1),
        (9, 11, 4),
        (11, 12, 2),
        (11, 13, 4),
        (13, 5, 4),
        (13, 14, 1),
    ];
    let mf = molfile(&els, &bonds, &["M  CHG  1   4   1"]);
    let v: Value = serde_json::from_str(&fragments_json(&mf, -1, -1).unwrap()).unwrap();
    let cuts = v["cuts"].as_array().unwrap();
    assert!(!cuts.is_empty());
    for s in cuts.iter().flat_map(|c| c["sides"].as_array().unwrap()) {
        assert!(s["mz"].as_f64().unwrap() <= 195.0877 + 1e-3, "{s}");
    }
}

#[test]
fn partial_of_a_cation_adds_no_proton() {
    let mf = molfile(&["C", "N"], &[(1, 2, 1)], &["M  CHG  1   2   1"]);
    let j = partial_json(&mf, "+O").unwrap();
    assert!(
        j.contains("\"formula\":\"CH6NO+\"") && j.contains("\"mz\":48.0444"),
        "{j}"
    );
}
