//! MEASURE (no default is changed): a true product whose formula is also an MS2 fragment of the parent (so it co-elutes inside the ISF window), 2 % of the parent
//! at the t0 and growing to 40 % of it at the last of 7 treated times. Its status in `tp/1` with the default gates, for parents that decay to 100 %, 50 %, 20 %,
//! 10 % and 5 % of their t0 area, with and without the fragment list of the parent's MS2. The table is built here (the gates read areas, RT and the MS2 fragments).
//! Numbers: `docs/agenti/mzfeat-tp.md` (MISURA). A constant 2 % ratio (a genuine in-source fragment) is the control of the harness.

use mzlab_feat::evid::rowproofs::NONE;
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::tp::*;

/// Files: blank, t0, then 7 treated times (minutes).
const MINUTES: [f64; 9] = [f64::NAN, 0.0, 5.0, 10.0, 20.0, 30.0, 45.0, 60.0, 90.0];
const PARENT_MZ: f64 = 300.1234;
const FRAGMENT_MZ: f64 = PARENT_MZ - 18.010565;

fn table(parent: &[f64], product: &[f64]) -> Aligned {
    let mut t = Aligned::with_capacity(MINUTES.len(), 2);
    for (mz, rt, a) in [(PARENT_MZ, 5.0, parent), (FRAGMENT_MZ, 5.02, product)] {
        t.mz.push(mz);
        t.rt.push(rt);
        for v in a {
            t.area.push(*v);
            t.height.push((v / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state
                .push(if *v > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

/// Areas of the parent (decaying to `f_end` of its t0 area at 90 min) and of the product with the share `share(t)` of the parent; the blank has neither.
fn series(f_end: f64, share: impl Fn(f64) -> f64) -> (Vec<f64>, Vec<f64>) {
    let k = -f_end.ln() / 90.0;
    let parent: Vec<f64> = MINUTES
        .iter()
        .map(|&t| {
            if t.is_nan() {
                0.0
            } else {
                1e8 * (-k * t).exp()
            }
        })
        .collect();
    let product = MINUTES
        .iter()
        .zip(&parent)
        .map(|(&t, &p)| if t.is_nan() { 0.0 } else { p * share(t) })
        .collect();
    (parent, product)
}

/// (status, notes) of the product row.
fn status(parent: &[f64], product: &[f64], fragments: Option<&[f64]>) -> (u8, u8) {
    let t = table(parent, product);
    let signs = vec![[NONE; 10]; 2];
    let inp = TpInputs {
        aligned: &t,
        minutes: &MINUTES,
        hyps: &[],
        signs: &signs,
        parent: (PARENT_MZ, 5.0),
        fragments,
        infusion: None,
        params: &TpParams::default(),
        rules: None,
    };
    let b = build(&inp).unwrap().expect("the parent is in the table");
    (b.status[1], b.notes[1])
}

/// Pinned: a growing product is kept (status 3, formed but unexplained: no hypothesis is given here) while its best treated area beats `ratio` (3) times its t0
/// area; the best is at the middle times, so it survives a parent decaying to 10 % of t0 and is lost (status 1, control) at 5 %. The fragment list only adds
/// the note "co-elutes with a fragment" (notes 3 = t0 present + co-elution, against 1 without the list).
#[test]
fn a_growing_product_with_the_fragment_formula_is_lost_when_the_parent_decays_far() {
    let frags = [FRAGMENT_MZ];
    for (f_end, want) in [
        (1.0, ST_UNEXPLAINED),
        (0.5, ST_UNEXPLAINED),
        (0.2, ST_UNEXPLAINED),
        (0.1, ST_UNEXPLAINED),
        (0.05, ST_CONTROL),
    ] {
        let (p, a) = series(f_end, |t| 0.02 + 0.38 * t / 90.0);
        let kept = want == ST_UNEXPLAINED;
        assert_eq!(
            status(&p, &a, None),
            (want, u8::from(kept)),
            "f_end {f_end}"
        );
        let notes = if kept {
            NOTE_T0_PRESENT | NOTE_ISF_COELUTES
        } else {
            0
        };
        assert_eq!(
            status(&p, &a, Some(&frags)),
            (want, notes),
            "f_end {f_end}, fragments"
        );
    }
}

/// A constant 2 % of the parent (a genuine in-source fragment) never reaches the ISF status here: the formed gate sends it to the controls first, with or
/// without the fragment list, for every parent decay. Pinned.
#[test]
fn a_constant_ratio_goes_to_the_controls() {
    for f_end in [1.0, 0.5, 0.1, 0.05] {
        let (p, a) = series(f_end, |_| 0.02);
        assert_eq!(
            status(&p, &a, Some(&[FRAGMENT_MZ])),
            (ST_CONTROL, 0),
            "f_end {f_end}"
        );
        assert_eq!(status(&p, &a, None), (ST_CONTROL, 0), "f_end {f_end}");
    }
}
