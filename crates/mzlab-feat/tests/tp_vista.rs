//! The view of the transformation products (`tp/1`): the gates in order with their status, kinship from the signs of the proofs, the block in the `.mzfeat`
//! and back, and that the controls are read (a view with no controls is an error, not a pass).

use mzlab_feat::evid::rowproofs::{Signs, AGAINST, FOR, NONE};
use mzlab_feat::hypo::{Dir, Hypothesis, SRC_RULE};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{read_table, write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::tp::*;

/// Files: blank, t0, then 0, 5, 10, 20 and 40 minutes of treatment.
const MINUTES: [f64; 7] = [f64::NAN, 0.0, 5.0, 10.0, 20.0, 40.0, 80.0];

fn row(mz: f64, rt: f64, a: [f64; 7]) -> (f64, f64, [f64; 7]) {
    (mz, rt, a)
}

/// The parent (decays), its in-source fragment (constant ratio), a product that forms, one present in the blank, a product also in the t0 that grows,
/// a product that forms and is not covered by anything, and a product that forms with a rise-and-fall that goes against the kinetics.
fn table() -> Aligned {
    let parent = [0.0, 1e7, 9e6, 7e6, 5e6, 3e6, 1e6];
    let isf: Vec<f64> = parent.iter().map(|a| a * 0.05).collect();
    let rows = [
        row(300.0, 5.0, parent),
        row(250.0, 5.02, isf.clone().try_into().unwrap()),
        row(316.0, 4.0, [0.0, 0.0, 1e5, 3e5, 6e5, 8e5, 9e5]),
        row(330.0, 6.0, [5e5, 4e5, 5e5, 6e5, 7e5, 8e5, 9e5]),
        row(332.0, 4.5, [0.0, 1e5, 2e5, 5e5, 8e5, 9e5, 9e5]),
        row(340.0, 7.0, [0.0, 0.0, 1e5, 2e5, 3e5, 4e5, 5e5]),
        row(345.0, 3.0, [0.0, 0.0, 2e5, 5e5, 4e5, 2e5, 1e5]),
    ];
    let mut t = Aligned::with_capacity(7, rows.len());
    for (mz, rt, a) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for v in a {
            t.area.push(v);
            t.height.push((v / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state.push(if v > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

fn hyp(mz: f64, sources: u8) -> Hypothesis {
    Hypothesis {
        id: 1,
        sources,
        label: "r".into(),
        formula: String::new(),
        neutral: mz - 1.00727646688,
        mz,
        adduct: "[M+H]+",
        polarity: 1,
        steps: 1,
        level: 3,
        dir: Dir::Free,
        rt_ref: None,
        alternatives: Vec::new(),
    }
}

fn run(signs: &[Signs], fragments: Option<&[f64]>) -> TpBlock {
    run_with(&MINUTES, signs, fragments)
}

fn run_with(minutes: &[f64], signs: &[Signs], fragments: Option<&[f64]>) -> TpBlock {
    let t = table();
    let hyps = [hyp(316.0, SRC_RULE), hyp(332.0, SRC_RULE), hyp(345.0, SRC_RULE), hyp(250.0, SRC_RULE)];
    let inp = TpInputs { aligned: &t, minutes, hyps: &hyps, signs, parent: (300.0, 5.0), fragments, params: &TpParams::default() };
    build(&inp).unwrap().expect("the parent is in the table")
}

fn signs() -> Vec<Signs> {
    let mut s = vec![[NONE; 10]; 7];
    // the kinetics proof is the sixth kind (Kind::ALL order), the controls the seventh, the network the fifth
    s[2][5] = FOR;
    s[4][5] = FOR;
    s[5][5] = FOR;
    s[6][5] = AGAINST;
    s
}

#[test]
fn the_gates_give_a_status_to_every_row() {
    let b = run(&signs(), None);
    // 0 parent; 1 its in-source fragment, which is in the t0 as high as later (the first gate already takes it out); 2 a product; 3 present in the blank;
    // 4 also in the t0 but it grows 3-fold; 5 formed but covered by no hypothesis; 6 covered but against the kinetics
    assert_eq!(b.status, vec![ST_PARENT, ST_CONTROL, ST_TP, ST_CONTROL, ST_TP, ST_UNEXPLAINED, ST_TP]);
    assert_eq!(b.notes[4] & NOTE_T0_PRESENT, NOTE_T0_PRESENT);
    assert_eq!(b.parent_row, 0);
    assert_eq!(b.count(ST_TP), 3);
}

#[test]
fn kinship_reads_the_proofs_and_a_proof_against_is_never_offset() {
    let mut s = signs();
    let b = run(&s, None);
    assert_eq!((b.kinship[2], b.kinship[4], b.kinship[6]), (KIN_PROBABLE, KIN_PROBABLE, KIN_DOUBTFUL));
    assert_eq!(b.kinship[5], KIN_NONE, "an unexplained row has no kinship");
    // an MS2 related to the parent's makes it certain, unless something is against
    s[2][4] = FOR;
    s[6][4] = FOR;
    let b = run(&s, None);
    assert_eq!((b.kinship[2], b.kinship[6]), (KIN_CERTAIN, KIN_DOUBTFUL));
    // without the kinetic proof an explained row is only possible
    s[4][5] = NONE;
    assert_eq!(run(&s, None).kinship[4], KIN_POSSIBLE);
}

#[test]
fn the_fragments_of_the_parent_decide_the_in_source_fragments() {
    // a series with no t0: the second gate is the one that tells the in-source fragment (its ratio to the parent is constant)
    let m = [f64::NAN, 2.0, 5.0, 10.0, 20.0, 40.0, 80.0];
    assert_eq!(run_with(&m, &signs(), None).status[1], ST_ISF, "no MS2 of the parent: the ratio and the coelution decide");
    let b = run_with(&m, &signs(), Some(&[250.0001, 120.0]));
    assert_eq!(b.status[1], ST_ISF, "present in the MS2 of the parent");
    let b = run_with(&m, &signs(), Some(&[120.0]));
    assert_ne!(b.status[1], ST_ISF, "the MS2 of the parent does not hold it: not an in-source fragment of it");
}

#[test]
fn no_controls_is_an_error_and_no_parent_is_no_view() {
    let t = table();
    let s = signs();
    let treated_only = [5.0; 7];
    let mut inp = TpInputs { aligned: &t, minutes: &treated_only, hyps: &[], signs: &s, parent: (300.0, 5.0), fragments: None, params: &TpParams::default() };
    assert!(build(&inp).is_err());
    inp.minutes = &MINUTES;
    inp.parent = (999.0, 5.0);
    assert!(build(&inp).unwrap().is_none());
}

#[test]
fn the_block_survives_the_file() {
    let mut t = table();
    t.tp = run(&signs(), None);
    let bytes = write_table(&t, &Recipe::default(), &Origin::default());
    assert_eq!(bytes, write_table(&t, &Recipe::default(), &Origin::default()));
    let (back, meta) = read_table(&bytes).unwrap();
    assert_eq!(back.tp, t.tp);
    assert_eq!(meta.get("tp").unwrap().get("schema").and_then(|v| v.str()), Some("tp/1"));
    assert_eq!(meta.get("tp").unwrap().get("conteggi").unwrap().get("tp").and_then(|v| v.f64()), Some(3.0));
    // a table without the block reads as before
    let t2 = table();
    let (b2, m2) = read_table(&write_table(&t2, &Recipe::default(), &Origin::default())).unwrap();
    assert!(b2.tp.is_empty() && m2.get("tp").is_none());
}
