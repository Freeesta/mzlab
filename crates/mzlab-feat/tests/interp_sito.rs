#![cfg(feature = "interp")]
//! The position of a change (`interp::sito`) and its drawing (`interp::disegno`) on textbook molecules: the candidate sites, the declared confidence and
//! the form of the drawing. Peaks are the fragments the cleavages of one candidate predict, so the answer is known before the call.

use mzlab_core::elements::parse_formula;
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{expected_fragments, sites, Peak, Pol};
use mzlab_feat::interp::disegno::drawing;
use mzlab_feat::interp::sito::{analyse, candidates, undetermined, Facts, SiteReport};

const FACTS: Facts = Facts {
    scans: 3,
    clean: 3,
    isomers: "single",
    has_reference: true,
};

/// The peaks of one candidate site (all its fragments, descending intensity) plus the precursor.
fn peaks_of(smiles: &str, tr: &str, atom: usize) -> Vec<Peak> {
    let mol = Mol::parse(smiles).unwrap();
    let s = sites(&mol, tr).unwrap();
    let site = s.iter().find(|x| x.atom == atom).unwrap();
    let mut p: Vec<Peak> = expected_fragments(&mol, site, Pol::Pos)
        .iter()
        .enumerate()
        .map(|(i, f)| Peak {
            mz: f.mz,
            inten: 100.0 - i as f64,
        })
        .collect();
    p.sort_by(|a, b| a.mz.total_cmp(&b.mz));
    p.dedup_by(|a, b| (a.mz - b.mz).abs() < 1e-6);
    p
}

fn report(smiles: &str, parent: &str, tp: &str, peaks: &[Peak], facts: &Facts) -> SiteReport {
    let c = candidates(
        smiles,
        &parse_formula(parent).unwrap(),
        &parse_formula(tp).unwrap(),
    )
    .unwrap();
    analyse(&c, peaks, Pol::Pos, facts, None)
}

const PHENOL: &str = "CCc1ccc(O)cc1";

#[test]
fn a_chain_position_the_fragments_single_out_is_a_site_and_drawn_as_a_structure() {
    let p = peaks_of(PHENOL, "+O", 1);
    let r = report(PHENOL, "C8H10O", "C8H10O2", &p, &FACTS);
    assert_eq!(r.confidence, "site");
    assert!(r.reasons.is_empty());
    assert_eq!(r.sublevel, "3a");
    let d = drawing(PHENOL, &r);
    assert_eq!(
        (d.mode, d.atoms.as_slice(), d.added),
        ("structure", &[1usize][..], Some("O"))
    );
    assert_eq!((d.uncertainty, d.fallback), ("none", Some("bracket")));
    assert_eq!(d.clone().without_molfile().mode, "bracket");
    let total: f64 = r.votes.iter().map(|v| v.support).sum();
    assert!(
        (total - 1.0).abs() < 1e-9,
        "the support of the classes left adds up to 1"
    );
}

#[test]
fn ring_positions_are_a_region_and_drawn_as_a_multicenter() {
    let p = peaks_of(PHENOL, "+O", 3);
    let r = report(PHENOL, "C8H10O", "C8H10O2", &p, &FACTS);
    assert_eq!(r.confidence, "region");
    assert_eq!(r.region.as_ref().map(|x| x.0.as_str()), Some("ring A"));
    let d = drawing(PHENOL, &r);
    assert_eq!(d.mode, "multicenter");
    assert_eq!(d.ends, [3, 4, 7, 8]);
    assert_eq!(d.uncertainty, "zone");
    assert!(d.label.contains("within ring A"));
}

#[test]
fn what_cannot_be_said_is_said_with_its_reason_and_a_whole_parent_bracket() {
    let p = peaks_of(PHENOL, "+O", 1);
    let cases = [
        (
            Facts {
                scans: 0,
                clean: 0,
                ..FACTS
            },
            "no_ms2",
        ),
        (
            Facts {
                scans: 2,
                clean: 0,
                ..FACTS
            },
            "chimeric",
        ),
        (
            Facts {
                isomers: "not_separated",
                ..FACTS
            },
            "isomers_not_separated",
        ),
        (
            Facts {
                has_reference: false,
                ..FACTS
            },
            "no_reference",
        ),
    ];
    for (facts, reason) in cases {
        let r = report(PHENOL, "C8H10O", "C8H10O2", &p, &facts);
        assert_eq!(
            (r.confidence, r.reasons.as_slice()),
            ("cannot_say", &[reason][..]),
            "{reason}"
        );
        assert_eq!(r.sublevel, "3b");
        let d = drawing(PHENOL, &r);
        assert_eq!(
            (d.mode, d.uncertainty, d.atoms.len()),
            ("bracket", "whole", 9)
        );
        assert!(d.label.ends_with("position not determined"));
    }
    let one = [Peak {
        mz: 123.04406,
        inten: 100.0,
    }];
    let r = report(PHENOL, "C8H10O", "C8H10O2", &one, &FACTS);
    assert_eq!(r.confidence, "cannot_say");
    assert!(r.reasons.contains(&"few_fragments"));
}

#[test]
fn several_isomers_never_give_a_site() {
    let p = peaks_of(PHENOL, "+O", 1);
    let facts = Facts {
        isomers: "separated",
        ..FACTS
    };
    let r = report(PHENOL, "C8H10O", "C8H10O2", &p, &facts);
    assert_ne!(r.confidence, "site");
}

#[test]
fn a_change_posizioni_does_not_know_is_named() {
    let parent = parse_formula("C8H10O").unwrap();
    let two_o = parse_formula("C8H10O3").unwrap();
    assert_eq!(
        candidates(PHENOL, &parent, &two_o).err(),
        Some("multiple_changes")
    );
    let nitro = parse_formula("C8H11NO").unwrap();
    assert_eq!(
        candidates(PHENOL, &parent, &nitro).err(),
        Some("unsupported_change")
    );
    let r = undetermined("+2O", vec!["multiple_changes"]);
    let d = drawing(PHENOL, &r);
    assert_eq!((d.mode, d.delta.as_str()), ("bracket", "+2O"));
}

#[test]
fn the_delta_label_uses_the_true_minus() {
    let c = candidates(
        "CCCO",
        &parse_formula("C3H8O").unwrap(),
        &parse_formula("C3H6O").unwrap(),
    )
    .unwrap();
    assert_eq!(c.transform, "-2H");
    let r = undetermined(c.transform, vec!["no_ms2"]);
    assert_eq!(drawing("CCCO", &r).delta, "\u{2212}2H");
}

#[test]
fn a_loss_keeps_the_rest_and_a_region_highlights_its_best_class() {
    let sm = "COc1ccc(CCN)cc1";
    let c = candidates(
        sm,
        &parse_formula("C9H13NO").unwrap(),
        &parse_formula("C8H11NO").unwrap(),
    )
    .unwrap();
    assert_eq!(c.transform, "-CH2");
    let p = peaks_of(sm, "-CH2", 0);
    let r = analyse(&c, &p, Pol::Pos, &FACTS, None);
    assert_eq!(r.confidence, "site", "{:?}", r.reasons);
    let d = drawing(sm, &r);
    assert_eq!(d.lost, [0]);
    assert_eq!(d.kept.len(), 10);
    assert!(!d.kept.contains(&0));
    let p = peaks_of(PHENOL, "+O", 3);
    let r = report(PHENOL, "C8H10O", "C8H10O2", &p, &FACTS);
    let d = drawing(PHENOL, &r);
    assert_eq!(d.highlight, [3, 4, 7, 8]);
    assert!(d.zones.is_empty());
}
