//! Stereo sites piece by piece: the same sites as the flip test over the whole record (kept here as the reference) on the RDKit golden
//! molecules, the canonical-form set and random records of 1 to 4 pieces, and work that grows linearly with equal pieces (counted, not timed).

use super::*;

#[path = "../tests/canon_set/mod.rs"]
mod canon_set;

/// The analysis before the split: every flip test runs on the whole record against the candidates of every piece.
fn whole_record(m: &Mol) -> (Vec<Site>, u64) {
    let base = m.stripped();
    let cands = base.candidates();
    let sites: Vec<Site> = cands.iter().map(|c| c.site).collect();
    let work = Cell::new(0);
    let mut out = Vec::new();
    for (i, c) in cands.iter().enumerate() {
        if c.sure || base.flips(&sites, i, &work) {
            out.push(c.site);
        }
    }
    out.sortm();
    (out, work.get())
}

fn golden() -> Vec<String> {
    let g: serde_json::Value =
        serde_json::from_str(include_str!("../tests/data/golden_stereo.json")).expect("golden");
    g["sites"]
        .as_array()
        .expect("sites")
        .iter()
        .map(|r| r[0].as_str().unwrap_or_default().to_string())
        .collect()
}

fn same_as_whole_record(smiles: &str) {
    let m = Mol::from_smiles(smiles).unwrap_or_else(|e| panic!("{smiles}: {e}"));
    assert_eq!(m.stereo_sites(), whole_record(&m).0, "{smiles}");
}

#[test]
fn golden_molecules_keep_their_sites() {
    for s in golden() {
        same_as_whole_record(&s);
    }
}

#[test]
fn canonical_set_keeps_its_sites() {
    // the reference is exponential in the pieces that hold undecided sites: up to four pieces, two of the cages (4 cubanes take it 70 s)
    let most = |s: &str| {
        if s.starts_with("C12") || s.starts_with("C1C2") {
            2
        } else {
            4
        }
    };
    for s in canon_set::smiles()
        .iter()
        .filter(|s| s.split(['.', ' ']).count() <= most(s))
    {
        same_as_whole_record(s);
    }
}

#[test]
fn random_records_of_up_to_four_pieces_keep_their_sites() {
    let pool = golden();
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = |n: usize| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as usize % n
    };
    for _ in 0..200 {
        let k = 1 + next(4);
        // the CXSMILES extension (enhanced stereo) names atoms of one molecule: only the plain SMILES are joined
        let parts: Vec<&str> = (0..k)
            .map(|_| pool[next(pool.len())].split(' ').next().unwrap_or_default())
            .collect();
        same_as_whole_record(&parts.join("."));
    }
}

/// The record of the report: an amide with two cis/trans double bonds on a 1,4-disubstituted cyclohexane (ring cis/trans).
const PIECE: &str = "CC(C)C=CC(=O)NC1CCC(CC1)C=CC";

fn copies(k: usize) -> Mol {
    Mol::from_smiles(&vec![PIECE; k].join(".")).expect("smiles")
}

#[test]
fn equal_pieces_add_their_work() {
    let (one, w1) = copies(1).stereo_sites_counted();
    assert_eq!(one.len(), 4, "{one:?}");
    assert!(w1 > 0, "the ring centres need a flip test");
    for k in [2, 8, 40] {
        let (sites, w) = copies(k).stereo_sites_counted();
        assert_eq!(sites.len(), 4 * k, "{k} pieces");
        assert_eq!(w, w1 * k as u64, "{k} pieces: work {w}, one piece {w1}");
    }
}

#[test]
fn equal_pieces_were_multiplied_by_the_whole_record_test() {
    // the reference on two pieces already costs more than twice one piece: the split is what removes the product
    let (_, w1) = whole_record(&copies(1));
    let (_, w2) = whole_record(&copies(2));
    assert!(w2 > 2 * w1, "whole record {w2}, one piece {w1}");
}
