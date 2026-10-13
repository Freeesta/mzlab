//! Gate 1 of the view of the products by role (`tp::build_by_role`): which reference takes a row out (blank, t0, dark), the infusion never a reference,
//! a series without t0 noted on every row, the counts in the meta and back; the minutes alone keep the old gate.

use mzlab_core::campioni::proponi;
use mzlab_core::json::parse;
use mzlab_feat::evid::rowproofs::{Signs, NONE};
use mzlab_feat::hypo::{Dir, Hypothesis, SRC_RULE};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::tp::*;

/// Files: blank, t0, dark, infusion, then 5, 10, 20 and 40 minutes of treatment.
const NAMES: [&str; 8] = [
    "X_blank",
    "X_TiO2_t000min",
    "X_dark-adsorption",
    "X_standard_direct-infusion_MSn",
    "X_TiO2_t005min",
    "X_TiO2_t010min",
    "X_TiO2_t020min",
    "X_TiO2_t040min",
];

/// The parent; an ion only in the blank; one at t0 that does not grow; one at t0 that grows 5-fold; one in the dark; one only in the infusion
/// (and a little in the treated); a product.
#[rustfmt::skip]
const ROWS: [(f64, f64, [f64; 8]); 7] = [
    (300.0, 5.0, [0.0, 1e7, 8e6, 2e7, 9e6, 7e6, 5e6, 3e6]),
    (310.0, 6.0, [5e5, 0.0, 0.0, 0.0, 4e5, 4e5, 4e5, 4e5]),
    (320.0, 6.5, [0.0, 5e5, 0.0, 0.0, 5e5, 6e5, 6e5, 5e5]),
    (330.0, 7.0, [0.0, 1e5, 0.0, 0.0, 2e5, 3e5, 4e5, 5e5]),
    (340.0, 7.5, [0.0, 0.0, 5e5, 0.0, 2e5, 4e5, 6e5, 6e5]),
    (350.0, 8.0, [0.0, 0.0, 0.0, 1e6, 1e5, 2e5, 3e5, 4e5]),
    (360.0, 8.5, [0.0, 0.0, 0.0, 0.0, 1e5, 3e5, 6e5, 9e5]),
];

fn table() -> Aligned {
    let mut t = Aligned::with_capacity(8, ROWS.len());
    for (mz, rt, a) in ROWS {
        t.mz.push(mz);
        t.rt.push(rt);
        for v in a {
            t.area.push(v);
            t.height.push((v / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state
                .push(if v > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

fn hyp(mz: f64) -> Hypothesis {
    Hypothesis {
        id: 1,
        sources: SRC_RULE,
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

fn roles(names: &[&str]) -> TpRoles {
    let files: Vec<(String, String, String)> = names
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    TpRoles::of(&proponi(&files))
}

/// The view with the roles (`Some`) or with the minutes alone (`None`: NaN for every file that is not timed, as the old pages send).
fn run(ro: Option<&TpRoles>) -> TpBlock {
    let t = table();
    let hyps: Vec<Hypothesis> = ROWS.iter().skip(1).map(|r| hyp(r.0)).collect();
    let signs: Vec<Signs> = vec![[NONE; 10]; ROWS.len()];
    let minutes = [f64::NAN, 0.0, f64::NAN, f64::NAN, 5.0, 10.0, 20.0, 40.0];
    let params = TpParams::default();
    let inp = TpInputs {
        aligned: &t,
        minutes: &minutes,
        hyps: &hyps,
        signs: &signs,
        parent: (300.0, 5.0),
        fragments: None,
        params: &params,
        rules: None,
    };
    match ro {
        Some(ro) => build_by_role(&inp, ro),
        None => build(&inp),
    }
    .unwrap()
    .expect("the parent is in the table")
}

#[test]
fn the_reference_that_takes_a_row_out_is_noted() {
    let ro = roles(&NAMES);
    assert_eq!(
        (
            ro.blank.as_slice(),
            ro.t0.as_slice(),
            ro.dark.as_slice(),
            ro.infusion.as_slice()
        ),
        (&[0][..], &[1][..], &[2][..], &[3][..])
    );
    let b = run(Some(&ro));
    assert_eq!(
        b.status,
        [ST_PARENT, ST_CONTROL, ST_CONTROL, ST_TP, ST_CONTROL, ST_TP, ST_TP]
    );
    let reason =
        |r: usize| b.notes[r] & (NOTE_BLANK | NOTE_T0 | NOTE_DARK | NOTE_T0_PRESENT | NOTE_NO_T0);
    assert_eq!(
        (1..7).map(reason).collect::<Vec<_>>(),
        [NOTE_BLANK, NOTE_T0, NOTE_T0_PRESENT, NOTE_DARK, 0, 0]
    );
    let c = b.roles.clone().unwrap();
    assert_eq!((c.by_blank, c.by_t0, c.by_dark, c.no_t0), (1, 1, 1, false));
    assert_eq!(c.dark_over_t0, Some(0.8));
    assert_eq!(c.blank_kinds, [""]);
}

#[test]
fn the_minutes_alone_keep_the_old_gate() {
    let b = run(None);
    // the infusion is a control there: the ion only in it is taken out; no reason, no roles in the meta
    assert_eq!(
        b.status,
        [ST_PARENT, ST_CONTROL, ST_CONTROL, ST_TP, ST_CONTROL, ST_CONTROL, ST_TP]
    );
    assert!(b
        .notes
        .iter()
        .all(|n| n & (NOTE_BLANK | NOTE_T0 | NOTE_DARK | NOTE_NO_T0) == 0));
    assert!(b.roles.is_none());
    assert!(!b.meta_json().contains("ruoli"));
}

#[test]
fn without_t0_every_row_says_so() {
    let mut names = NAMES;
    names[1] = "X_TiO2_untimed";
    let ro = roles(&names);
    assert!(ro.t0.is_empty());
    let b = run(Some(&ro));
    assert!(b.notes.iter().all(|n| n & NOTE_NO_T0 != 0));
    // the ion at t0 that does not grow is no longer taken out: that is the price of a series without t0
    assert_eq!(b.status[2], ST_TP);
    let c = b.roles.unwrap();
    assert!(c.no_t0 && c.dark_over_t0.is_none() && c.by_t0 == 0);
}

#[test]
fn no_treated_or_no_reference_is_an_error() {
    let t = table();
    let minutes = [f64::NAN; 8];
    let params = TpParams::default();
    let signs: Vec<Signs> = vec![[NONE; 10]; ROWS.len()];
    let inp = TpInputs {
        aligned: &t,
        minutes: &minutes,
        hyps: &[],
        signs: &signs,
        parent: (300.0, 5.0),
        fragments: None,
        params: &params,
        rules: None,
    };
    let only_treated = roles(&[
        "a_t5", "a_t10", "a_t20", "a_t40", "b_t5", "b_t10", "b_t20", "b_t40",
    ]);
    assert_eq!(
        build_by_role(&inp, &only_treated).unwrap_err().key,
        "error.tpNoControls"
    );
    let no_treated = roles(&[
        "X_blank",
        "t0",
        "X_dark",
        "X_infusion",
        "X_blank2",
        "t0",
        "X_dark",
        "X_infusion",
    ]);
    assert_eq!(
        build_by_role(&inp, &no_treated).unwrap_err().key,
        "error.tpNoControls"
    );
}

#[test]
fn the_counts_go_in_the_meta_and_back() {
    let mut names = NAMES;
    names[0] = "X_blank_solv";
    let b = run(Some(&roles(&names)));
    let meta = b.meta_json();
    assert!(meta.contains("\"scartate_bianco\":1,\"scartate_t0\":1,\"scartate_buio\":1},\"ruoli\":{\"senza_t0\":false,\"buio_su_t0\":0.8,\"bianchi\":[\"solvent\"]}"), "{meta}");
    let back = TpBlock::from_meta(&parse(meta.as_bytes()).unwrap());
    assert_eq!(back.roles, b.roles);
}
