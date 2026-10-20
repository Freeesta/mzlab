#![cfg(all(feature = "interp", feature = "record"))]
//! The position of a change and its drawing from a `mol/1` record against the SMILES path: same candidates, same report, same drawing, for every
//! kind of answer (site, region, cannot say), over textbook molecules and the transformations `interp` knows.

use mzlab_core::elements::parse_formula;
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{expected_fragments, sites, Peak, Pol};
use mzlab_feat::interp::disegno::{drawing, drawing_record};
use mzlab_feat::interp::sito::{analyse, candidates, candidates_record, Facts};

const FACTS: Facts = Facts {
    scans: 3,
    clean: 3,
    isomers: "single",
    has_reference: true,
};

/// (parent SMILES, parent formula, product formula, transformation)
const CASES: [(&str, &str, &str, &str); 8] = [
    ("CCc1ccc(O)cc1", "C8H10O", "C8H10O2", "+O"),
    ("Cc1ccccc1", "C7H8", "C7H8O", "+O"),
    ("CN(C)c1ccccc1", "C8H11N", "C7H9N", "-CH2"),
    ("COc1ccc(Cl)cc1", "C7H7ClO", "C6H5ClO", "-CH2"),
    ("CCN(CC)CC", "C6H15N", "C6H15NO", "+O"),
    ("Oc1ccccc1", "C6H6O", "C6H5ClO", "+Cl-H"),
    ("CC(C)Cc1ccccc1", "C10H14", "C10H12O", "+O-2H"),
    ("CCCC", "C4H10", "C4H8", "-2H"),
];

/// The peaks of the fragments one candidate predicts.
fn peaks_of(smiles: &str, tr: &str, which: usize) -> Vec<Peak> {
    let mol = Mol::parse(smiles).unwrap();
    let s = sites(&mol, tr).unwrap();
    let mut p: Vec<Peak> = expected_fragments(&mol, &s[which % s.len()], Pol::Pos)
        .iter()
        .enumerate()
        .map(|(i, f)| Peak {
            mz: f.mz,
            inten: 100.0 - i as f64,
        })
        .collect();
    p.sort_by(|a, b| a.mz.total_cmp(&b.mz));
    p
}

#[test]
fn record_candidates_report_and_drawing_equal_the_smiles_path() {
    let mut n = 0;
    for (smi, parent, tp, tr) in CASES {
        let (pf, tf) = (parse_formula(parent).unwrap(), parse_formula(tp).unwrap());
        let rec = mzlab_mol::Mol::from_smiles(smi).unwrap();
        let a = candidates(smi, &pf, &tf);
        let b = candidates_record(&rec, smi, &pf, &tf);
        let (Ok(a), Ok(b)) = (a, b) else {
            panic!("{smi} {tr}: candidates differ in success");
        };
        assert_eq!((a.transform, &a.sites), (b.transform, &b.sites), "{smi}");
        for which in 0..3 {
            let peaks = peaks_of(smi, tr, which);
            let (ra, rb) = (
                analyse(&a, &peaks, Pol::Pos, &FACTS, None),
                analyse(&b, &peaks, Pol::Pos, &FACTS, None),
            );
            assert_eq!(ra, rb, "{smi} {which}");
            assert_eq!(
                drawing(smi, &ra),
                drawing_record(&rec, smi, &rb),
                "{smi} {which}"
            );
            n += 1;
        }
    }
    assert!(n >= 24);
}

#[test]
fn a_change_interp_does_not_know_is_refused_alike() {
    let rec = mzlab_mol::Mol::from_smiles("CCc1ccc(O)cc1").unwrap();
    let (p, t) = (
        parse_formula("C8H10O").unwrap(),
        parse_formula("C8H11NO").unwrap(),
    );
    assert_eq!(
        candidates("CCc1ccc(O)cc1", &p, &t).err(),
        candidates_record(&rec, "CCc1ccc(O)cc1", &p, &t).err()
    );
}
