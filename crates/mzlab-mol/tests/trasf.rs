//! The record `trasf/1`: validation against the two molecules it names, the stereo rule, chains and the canonical rewrite
//! (docs/studi/STRUTTURA_DATO_TRASFORMAZIONI.md). The molecules are small and the maps are written by hand.

use mzlab_mol::{delta, Mol, RegionKind, TRegion, TSite, Trasf, TRASF_FORMAT};

fn mol(smiles: &str) -> Mol {
    Mol::from_smiles(smiles).unwrap_or_else(|e| panic!("{smiles}: {e}"))
}

/// A certain step between two molecules with the map `pairs` (parent atom, product atom or lost).
fn step(
    parent: &Mol,
    product: &Mol,
    delta: &str,
    pairs: &[(u32, Option<u32>)],
    added: &[u32],
    centre: &[u32],
) -> Trasf {
    Trasf {
        format: TRASF_FORMAT.into(),
        parent: parent.core_hash_hex(),
        product: Some(product.core_hash_hex()),
        delta: delta.into(),
        rule: "hydroxylation".into(),
        rule_set: "regole_tp/2:fotocatalisi".into(),
        rule_version: String::new(),
        map: pairs.to_vec(),
        added: added.to_vec(),
        centre: centre.to_vec(),
        sites: Vec::new(),
        region: None,
        stereo_rule: None,
        level: None,
        evidence: Vec::new(),
    }
}

/// Toluene `Cc1ccccc1` → benzyl alcohol `OCc1ccccc1`: the oxygen is new and written first.
fn toluene_to_benzyl() -> (Mol, Mol, Trasf) {
    let (p, q) = (mol("Cc1ccccc1"), mol("OCc1ccccc1"));
    let pairs: Vec<(u32, Option<u32>)> = (0..7).map(|i| (i, Some(i + 1))).collect();
    let t = step(&p, &q, "+O", &pairs, &[0], &[0]);
    (p, q, t)
}

#[test]
fn a_certain_step_validates_and_round_trips() {
    let (p, q, t) = toluene_to_benzyl();
    t.validate_against(&p, Some(&q)).expect("valid");
    assert_eq!(Trasf::from_json(&t.to_json()).expect("json"), t);
    assert!(t.to_json().starts_with(r#"{"format":"trasf/1","parent":""#));
    assert_eq!(t.product_atom(0), Some(1));
    assert_eq!(t.parent_atom(0), None, "the added oxygen has no parent");
    assert_eq!(t.parent_atom(2), Some(1));
}

#[test]
fn a_step_that_does_not_fit_its_molecules_is_refused() {
    let (p, q, good) = toluene_to_benzyl();
    let mut t = good.clone();
    t.parent = q.core_hash_hex();
    assert!(
        t.validate_against(&p, Some(&q)).is_err(),
        "wrong parent hash"
    );
    let mut t = good.clone();
    t.map[3].1 = t.map[2].1;
    assert!(t.validate().is_err(), "two parents, one product atom");
    let mut t = good.clone();
    t.added.clear();
    assert!(t.validate().is_err(), "a product atom with no origin");
    let mut t = good.clone();
    t.map.swap(1, 2);
    assert!(t.validate().is_err(), "map out of order");
    let mut t = good.clone();
    t.centre.clear();
    t.map[0].1 = Some(0);
    t.added = vec![1];
    assert!(
        t.validate_against(&p, Some(&q)).is_err(),
        "the carbon becomes the oxygen outside the centre"
    );
    let mut t = good;
    t.delta = "+Oxygen".into();
    assert!(t.validate().is_err(), "delta");
}

#[test]
fn a_stereo_centre_at_the_reaction_centre_is_not_copied_as_defined() {
    let p = mol("C[C@H](N)C(=O)O");
    // hydroxylation at the stereocentre: the product has the tertiary alcohol, with its centre defined or not
    let defined = mol("C[C@](O)(N)C(=O)O");
    let unspecified = mol("CC(O)(N)C(=O)O");
    let pairs: Vec<(u32, Option<u32>)> = [0, 1, 3, 4, 5, 6]
        .iter()
        .enumerate()
        .map(|(i, &b)| (i as u32, Some(b)))
        .collect();
    // product atoms: C0 C1 O2 N3 C4 O5 O6; the parent C0 C1 N2 C3 O4 O5 maps to 0 1 3 4 5 6
    let mut t = step(&p, &unspecified, "+O", &pairs, &[2], &[1]);
    assert_eq!(t.touched_stereo(&p), vec![0]);
    t.validate_against(&p, Some(&unspecified))
        .expect("unknown stereo is fine");
    t.product = Some(defined.core_hash_hex());
    assert!(t.validate_against(&p, Some(&defined)).is_err());
    t.stereo_rule = Some("retention".into());
    t.validate_against(&p, Some(&defined))
        .expect("a rule says so");
    t.stereo_rule = None;
    t.centre = vec![2];
    assert!(
        t.touched_stereo(&p).is_empty(),
        "a centre elsewhere leaves it"
    );
}

#[test]
fn an_uncertain_product_has_sites_and_no_map() {
    let p = mol("Cc1ccccc1");
    let t = Trasf {
        format: TRASF_FORMAT.into(),
        parent: p.core_hash_hex(),
        product: None,
        delta: "+O".into(),
        rule: "hydroxylation".into(),
        rule_set: String::new(),
        rule_version: String::new(),
        map: Vec::new(),
        added: Vec::new(),
        centre: Vec::new(),
        sites: vec![TSite { atom: 2, p: 0.5 }, TSite { atom: 4, p: 0.5 }],
        region: Some(TRegion {
            atoms: vec![1, 2, 3, 4, 5, 6],
            kind: RegionKind::Multicenter,
        }),
        stereo_rule: None,
        level: Some("3".into()),
        evidence: Vec::new(),
    };
    t.validate_against(&p, None).expect("valid");
    assert!(!t.is_certain());
    assert_eq!(Trasf::from_json(&t.to_json()).expect("json"), t);
    let mut bad = t.clone();
    bad.map.push((0, Some(0)));
    assert!(bad.validate().is_err(), "an uncertain product has no map");
    let mut bad = t.clone();
    bad.sites.clear();
    assert!(bad.validate().is_err(), "an uncertain product needs sites");
    let mut bad = t.clone();
    bad.sites[0].atom = 4;
    assert!(bad.validate().is_err(), "a site twice");
    let mut bad = t.clone();
    bad.region = Some(TRegion {
        atoms: vec![1],
        kind: RegionKind::Zone,
    });
    assert!(bad.validate().is_err(), "region without the sites");
    let mut bad = t;
    bad.sites[0].atom = 99;
    assert!(
        bad.validate_against(&p, None).is_err(),
        "site outside the parent"
    );
}

#[test]
fn deltas_add_in_one_spelling() {
    assert_eq!(delta::add("+O", "+O").as_deref(), Ok("+O2"));
    assert_eq!(delta::add("+O", "-O").as_deref(), Ok(""));
    assert_eq!(delta::add("-Cl+OH", "+O").as_deref(), Ok("+HO2-Cl"));
    assert_eq!(delta::add("+CH2", "-H2+O").as_deref(), Ok("+CO"));
    assert_eq!(delta::add("", "").as_deref(), Ok(""));
    assert!(delta::parse("O").is_err());
    assert!(delta::parse("+o").is_err());
}

#[test]
fn two_steps_compose_and_a_lost_atom_stays_lost() {
    let (p, q, first) = toluene_to_benzyl();
    // benzyl alcohol → benzaldehyde: same atoms, the carbon is the centre
    let ald = mol("O=Cc1ccccc1");
    let same: Vec<(u32, Option<u32>)> = (0..8).map(|i| (i, Some(i))).collect();
    let second = step(&q, &ald, "-H2", &same, &[], &[1]);
    second
        .validate_against(&q, Some(&ald))
        .expect("second step");
    let whole = first.compose(&second).expect("chain");
    whole.validate_against(&p, Some(&ald)).expect("composed");
    assert_eq!(whole.delta, "+O-H2");
    assert_eq!(whole.added, vec![0]);
    assert_eq!(whole.centre, vec![0]);
    assert_eq!(whole.product_atom(0), Some(1));
    // benzyl alcohol → benzene: the side chain is lost
    let benzene = mol("c1ccccc1");
    let mut lose: Vec<(u32, Option<u32>)> = vec![(0, None), (1, None)];
    lose.extend((2..8).map(|i| (i, Some(i - 2))));
    let third = step(&q, &benzene, "-CH2O", &lose, &[], &[1]);
    third
        .validate_against(&q, Some(&benzene))
        .expect("third step");
    let down = first.compose(&third).expect("chain");
    down.validate_against(&p, Some(&benzene)).expect("composed");
    assert_eq!(down.delta, "-CH2");
    assert_eq!(down.product_atom(0), None, "the methyl carbon is gone");
    assert!(down.added.is_empty());
    assert!(second.compose(&first).is_err(), "not a chain");
}

#[test]
fn the_canonical_rewrite_does_not_depend_on_how_the_molecules_were_written() {
    let (p, q, a) = toluene_to_benzyl();
    // the same two molecules written the other way round: toluene ring c0..c5 then the methyl, benzyl alcohol ring c0 c1 c2 c3(C4 O5) c6 c7
    let (pb, qb) = (mol("c1ccccc1C"), mol("c1ccc(CO)cc1"));
    let pairs = [
        (0, Some(6)),
        (1, Some(7)),
        (2, Some(0)),
        (3, Some(1)),
        (4, Some(2)),
        (5, Some(3)),
        (6, Some(4)),
    ];
    let b = step(&pb, &qb, "+O", &pairs, &[5], &[6]);
    b.validate_against(&pb, Some(&qb)).expect("valid");
    let (ca, cb) = (
        a.canonical(&p, Some(&q)).expect("a"),
        b.canonical(&pb, Some(&qb)).expect("b"),
    );
    assert_eq!(ca, cb);
    ca.validate_against(&p.canonical(), Some(&q.canonical()))
        .expect("fits the canonical records");
}
