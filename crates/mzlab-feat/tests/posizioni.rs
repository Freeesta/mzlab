#![cfg(feature = "chem")]
//! Positions of a substitution: sites, symmetry, expected fragments and the comparison with the MS2. Structures are textbook molecules; masses and
//! formulas were worked out by hand (H 1.00782503, C 12, N 14.003074, O 15.99491462, Cl 34.96885268, proton 1.00727646).

use std::collections::BTreeSet;

use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{
    check_formulas, match_spectrum, positions_json, sites, Class, Peak, Pol, Report, Site, Tol,
};

fn run(smiles: &str, tr: &str, peaks: &[(f64, f64)], tol: Tol) -> Report {
    let mol = Mol::parse(smiles).unwrap();
    let s = sites(&mol, tr).unwrap();
    let p: Vec<Peak> = peaks
        .iter()
        .map(|&(mz, inten)| Peak { mz, inten })
        .collect();
    match_spectrum(&mol, &s, &p, tol, Pol::Pos).unwrap()
}

fn classes(smiles: &str, tr: &str) -> (Vec<Site>, usize) {
    let s = sites(&Mol::parse(smiles).unwrap(), tr).unwrap();
    let n = s
        .iter()
        .map(|x| (x.class, x.kind))
        .collect::<BTreeSet<_>>()
        .len();
    (s, n)
}

fn of(r: &Report, atom: usize) -> &Class {
    r.classes.iter().find(|c| c.atoms.contains(&atom)).unwrap()
}

#[test]
fn phenol_oxidation_has_three_distinct_aromatic_positions() {
    let (s, n) = classes("Oc1ccccc1", "+O");
    assert_eq!((s.len(), n), (5, 3));
    assert!(s
        .iter()
        .all(|x| x.kind == "aromatic_oh" && x.tp_formula == "C6H6O2"));
    // C6H6O2 = 110.03677943; [M+H]+ = 111.04405589
    let r = run("Oc1ccccc1", "+O", &[], Tol::Ppm(5.0));
    assert!((r.tp_mz - 111.0441).abs() < 1e-4);
}

#[test]
fn toluene_has_aromatic_and_aliphatic_sites() {
    let (s, n) = classes("Cc1ccccc1", "+O");
    assert_eq!((s.len(), n), (6, 4));
    assert_eq!(s.iter().filter(|x| x.kind == "aliphatic_oh").count(), 1);
    assert!(s.iter().all(|x| x.tp_formula == "C7H8O"));
}

// 4-methyldiphenylamine: ring A (atoms 1-4, 12, 13) carries the methyl (0), ring B is atoms 6-11.
const DPA: &str = "Cc1ccc(Nc2ccccc2)cc1";
// C7H8O (tolyl + O, +H) -> 109.06479132 ; C12H11NO (all but the methyl, +O) -> 186.09134041
const A_SIDE: f64 = 109.0648;
const REST: f64 = 186.0913;

#[test]
fn fragment_with_oxygen_on_ring_a_supports_a_and_excludes_b() {
    let r = run(DPA, "+O", &[(A_SIDE, 100.0), (REST, 50.0)], Tol::Ppm(5.0));
    assert_eq!(r.tp_formula, "C13H13NO");
    let a = of(&r, 2);
    assert_eq!(
        (a.status, a.atoms.clone()),
        ("supported", vec![2, 3, 12, 13])
    );
    assert_eq!(a.support[0].formula, "C7H8O");
    assert_eq!(of(&r, 8).status, "excluded");
    assert_eq!(of(&r, 8).atoms, vec![7, 8, 9, 10, 11]);
    assert_eq!(of(&r, 0).status, "excluded");
    assert_eq!(r.sublevel, "3a");
    assert!(r.unexplained.is_empty());
}

#[test]
fn without_fragments_nothing_is_distinguished() {
    for peaks in [vec![], vec![(300.0, 100.0)]] {
        let r = run(DPA, "+O", &peaks, Tol::Ppm(5.0));
        assert!(r.classes.iter().all(|c| c.status == "undistinguished"));
        assert_eq!(r.sublevel, "3b");
    }
}

#[test]
fn low_resolution_gives_the_same_classes() {
    let hr = run(DPA, "+O", &[(A_SIDE, 100.0), (REST, 50.0)], Tol::Ppm(5.0));
    let lr = run(DPA, "+O", &[(109.26, 100.0), (186.29, 50.0)], Tol::Da(0.5));
    let sig = |r: &Report| {
        r.classes
            .iter()
            .map(|c| (c.atoms.clone(), c.status))
            .collect::<Vec<_>>()
    };
    assert_eq!(sig(&hr), sig(&lr));
}

// 4-methoxy-N-methylaniline: methyls on O (atom 0) and on N (atom 7).
const ANI: &str = "COc1ccc(NC)cc1";

#[test]
fn demethylation_sites_are_told_apart_by_the_fragment_with_the_methyl() {
    let (s, n) = classes(ANI, "-CH2");
    assert_eq!((s.len(), n), (2, 2));
    assert!(s
        .iter()
        .all(|x| x.kind == "demethyl" && x.tp_formula == "C7H9NO"));
    // C6H6O (ring side without the O-methyl, +H) -> 95.0491 : only the O-demethylated isomer makes it
    let r = run(ANI, "-CH2", &[(95.0491, 100.0)], Tol::Ppm(5.0));
    assert_eq!(of(&r, 0).status, "supported");
    assert_eq!(of(&r, 7).status, "excluded");
    // C6H7N (ring side without the N-methyl, +H) -> 94.0651 : the other one
    let r = run(ANI, "-CH2", &[(94.0651, 100.0)], Tol::Ppm(5.0));
    assert_eq!(of(&r, 7).status, "supported");
    assert_eq!(of(&r, 0).status, "excluded");
}

#[test]
fn n_oxide_against_carbon_hydroxyl_suggests_hdx() {
    let (s, _) = classes("CN(C)Cc1ccccc1", "+O");
    assert!(s.iter().any(|x| x.kind == "n_oxide"));
    let r = run("CN(C)Cc1ccccc1", "+O", &[], Tol::Ppm(5.0));
    assert!(r.suggest.contains(&"hdx"));
    assert_eq!(*r.suggest.last().unwrap(), "standard");
    assert_eq!(r.ox_type, Some((false, false)));
    assert_eq!((r.classes.len(), r.classes[0].kind.as_str()), (1, "mixed"));
}

#[test]
fn chlorination_of_p_cresol() {
    let (s, n) = classes("Cc1ccc(O)cc1", "+Cl-H");
    assert_eq!((s.len(), n), (4, 2));
    assert!(s
        .iter()
        .all(|x| x.kind == "aromatic_cl" && x.tp_formula == "C7H7ClO"));
    // C6H5ClO (ring side, +H) -> 129.01016891 ; its 37Cl peak at +1.99705
    let r = run(
        "Cc1ccc(O)cc1",
        "+Cl-H",
        &[(129.0102, 100.0), (131.0072, 30.0)],
        Tol::Ppm(5.0),
    );
    assert_eq!(r.classes.len(), 1);
    assert_eq!(r.classes[0].status, "supported");
    assert!((r.classes[0].support[0].iso_mz.unwrap() - 131.0072).abs() < 1e-3);
    assert!(r.unexplained.is_empty());
}

#[test]
fn different_formulas_are_an_error() {
    let mut s = sites(&Mol::parse("Oc1ccccc1").unwrap(), "+O").unwrap();
    s[1].tp_formula = "C6H6O".into();
    assert_eq!(check_formulas(&s).unwrap_err().key, "error.posiz.formula");
    let mol = Mol::parse("Oc1ccccc1").unwrap();
    assert_eq!(
        match_spectrum(&mol, &s, &[], Tol::Da(0.5), Pol::Pos)
            .unwrap_err()
            .key,
        "error.posiz.formula"
    );
    assert_eq!(sites(&mol, "+N").unwrap_err().key, "error.posiz.transform");
    assert_eq!(
        sites(&Mol::parse("c1ccccc1").unwrap(), "-CH2")
            .unwrap_err()
            .key,
        "error.posiz.noSites"
    );
}

#[test]
fn json_contract() {
    let j = positions_json(
        DPA,
        "+O",
        "[[109.0648,100],{\"mz\":186.0913,\"i\":50}]",
        5.0,
        true,
        true,
    )
    .unwrap();
    for k in [
        "\"format\":\"posiz\"",
        "\"version\":1",
        "\"status\":\"supported\"",
        "\"status\":\"excluded\"",
        "\"sublevel\":\"3a\"",
        "\"suggest\":[",
        "\"unexplained\":[]",
    ] {
        assert!(j.contains(k), "{k} in {j}");
    }
    assert!(!j.contains("\"level\""));
    assert_eq!(
        positions_json(DPA, "+O", "{}", 5.0, true, true)
            .unwrap_err()
            .key,
        "error.posiz.peaks"
    );
}
