//! Tests of `cut/1` (`cut.rs`): homolytic, heterolytic and «+H» cuts, ring bonds, the H count pinned where the valence rule differs, and errors.

use super::*;

/// V2000 molfile of atoms `(el, x, y)` with bonds `(a, b, type)` (1-based) and property lines.
pub(crate) fn molfile(
    atoms: &[(&str, f64, f64)],
    bonds: &[(usize, usize, u8)],
    props: &[&str],
) -> String {
    let mut s = format!(
        "\n  qqq\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
        atoms.len(),
        bonds.len()
    );
    for (e, x, y) in atoms {
        s += &format!(
            "{x:>10.4}{y:>10.4}{:>10.4} {e:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n",
            0.0
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

fn req(mf: &str, bonds: &str, mode: &str, cation: &str) -> String {
    format!(
        "{{\"molfile\":{},\"bonds\":{bonds},\"mode\":\"{mode}\",\"cation\":{cation}}}",
        {
            let mut o = Out::default();
            o.str(mf);
            o.0
        }
    )
}

fn ethanol() -> String {
    molfile(
        &[("C", 0.0, 0.0), ("C", 1.0, 0.0), ("O", 2.0, 0.0)],
        &[(1, 2, 1), (2, 3, 1)],
        &[],
    )
}

#[test]
fn homolytic_gives_two_radicals_moved_apart() {
    let j = cut_json(&req(&ethanol(), "[[0,1]]", "homolytic", "[]")).unwrap();
    assert!(
        j.contains("{\"i\":0,\"charge\":0,\"radical\":2,\"h\":3,\"pin\":false}"),
        "{j}"
    );
    assert!(
        j.contains("\"formula\":\"CH3\"") && j.contains("\"formula\":\"CH3O\""),
        "{j}"
    );
    assert!(
        j.contains("\"kind\":\"radical\"") && !j.contains("\"kind\":\"neutral\""),
        "{j}"
    );
    assert!(
        j.contains("\"shift\":[-0.5,0]") && j.contains("\"shift\":[0.5,0]"),
        "{j}"
    );
    assert!(
        j.contains("\"unpaired\":1,\"electrons\":9,\"odd\":true"),
        "{j}"
    );
}

#[test]
fn heterolytic_puts_the_pair_on_the_anion() {
    let j = cut_json(&req(&ethanol(), "[[1,2]]", "heterolytic", "[1]")).unwrap();
    assert!(
        j.contains("\"text\":\"C2H5+\"") && j.contains("\"kind\":\"cation\""),
        "{j}"
    );
    assert!(
        j.contains("\"text\":\"HO-\"") && j.contains("\"kind\":\"anion\""),
        "{j}"
    );
    assert!(
        j.contains("{\"i\":2,\"charge\":-1,\"radical\":0,\"h\":1,\"pin\":false}"),
        "{j}"
    );
    assert_eq!(
        cut_json(&req(&ethanol(), "[[1,2]]", "heterolytic", "[0]"))
            .unwrap_err()
            .key,
        "error.grafo.cut.request"
    );
}

#[test]
fn a_nitrenium_keeps_its_hydrogens() {
    let mf = molfile(&[("C", 0.0, 0.0), ("N", 1.0, 0.0)], &[(1, 2, 1)], &[]);
    let j = cut_json(&req(&mf, "[[0,1]]", "heterolytic", "[1]")).unwrap();
    assert!(
        j.contains("{\"i\":1,\"charge\":1,\"radical\":0,\"h\":2,\"pin\":true}"),
        "{j}"
    );
    assert!(
        j.contains("\"text\":\"H2N+\"") && j.contains("\"text\":\"CH3-\""),
        "{j}"
    );
}

#[test]
fn hydrogen_shorthand_and_ring_cuts() {
    let j = cut_json(&req(&ethanol(), "[[1,2]]", "hydrogen", "[]")).unwrap();
    assert!(
        j.contains("\"formula\":\"C2H6\"")
            && j.contains("\"formula\":\"H2O\"")
            && j.contains("\"mz\":null"),
        "{j}"
    );
    let ring = molfile(
        &[
            ("C", 0.0, 0.0),
            ("C", 1.0, 0.0),
            ("C", 1.5, 0.8),
            ("C", 1.0, 1.6),
            ("C", 0.0, 1.6),
            ("C", -0.5, 0.8),
        ],
        &[
            (1, 2, 1),
            (2, 3, 1),
            (3, 4, 1),
            (4, 5, 1),
            (5, 6, 1),
            (6, 1, 1),
        ],
        &[],
    );
    let one = cut_json(&req(&ring, "[[0,1]]", "homolytic", "[]")).unwrap();
    assert!(
        one.contains("\"formula\":\"C6H12\"")
            && one.contains("\"unpaired\":2")
            && one.contains("\"shift\":[0,0]"),
        "{one}"
    );
    let two = cut_json(&req(&ring, "[[0,1],[3,4]]", "homolytic", "[]")).unwrap();
    assert_eq!(two.matches("\"formula\":\"C3H6\"").count(), 2, "{two}");
}

#[test]
fn existing_radical_and_charge_are_counted() {
    let mf = molfile(
        &[("C", 0.0, 0.0), ("N", 1.0, 0.0)],
        &[(1, 2, 1)],
        &["M  CHG  1   2   1", "M  RAD  1   2   2"],
    );
    let j = cut_json(&req(&mf, "[[0,1]]", "hydrogen", "[]")).unwrap();
    assert!(
        j.contains("\"text\":\"NH3+\"") && j.contains("\"kind\":\"radicalCation\""),
        "{j}"
    );
    let bad = |b: &str, m: &str| cut_json(&req(&mf, b, m, "[]")).unwrap_err().key;
    assert_eq!(bad("[]", "homolytic"), "error.grafo.cut.none");
    assert_eq!(bad("[[0,1],[1,0]]", "homolytic"), "error.grafo.cut.request");
    assert_eq!(bad("[[0,1]]", "sideways"), "error.grafo.cut.mode");
    let ene = molfile(&[("C", 0.0, 0.0), ("C", 1.0, 0.0)], &[(1, 2, 2)], &[]);
    assert_eq!(
        cut_json(&req(&ene, "[[0,1]]", "heterolytic", "[]"))
            .unwrap_err()
            .key,
        "error.grafo.cut.order"
    );
    assert!(cut_json(&req(&ene, "[[0,1]]", "homolytic", "[]"))
        .unwrap()
        .contains("\"radical\":3"));
}
