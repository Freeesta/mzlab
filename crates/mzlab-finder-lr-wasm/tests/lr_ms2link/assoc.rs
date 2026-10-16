//! The rule that ties an MS2 to a feature (`docs/agenti/mzfinder-ms2-regola.md`): the precursor within the tolerance of the m/z read in the table AND the
//! scans inside the peak of the feature. A feature of the synthetic series with its read m/z: row 7 at 7.4 min, read 257.3595 (m/z of the table).

use super::*;
use mzlab_feat::legacy::lr::ms2frag;

fn read_mz(d: &lr::LrDig, id: i64) -> f64 {
    row(d, id).observed.unwrap()
}

/// Row `id` with its peak narrowed to `[lo, hi]` in every file (a feature narrower than the window the Dig looks in).
fn narrow(mut d: lr::LrDig, id: i64, lo: f64, hi: f64) -> lr::LrDig {
    for c in &mut d.entries.iter_mut().find(|e| e.id == id).unwrap().cells {
        (c.lo, c.hi) = (lo, hi);
    }
    d
}

fn narrow_x(precursor: f64, apex: f64) -> X {
    X {
        peaks: vec![(apex, 0.03, 5.0e4)],
        ..x(precursor, &[])
    }
}

#[test]
fn a_precursor_0_62_da_from_the_feature_is_not_matched_and_0_03_da_is() {
    let d = base().1.clone();
    let o = read_mz(&d, 7);
    // the file writes a window of 1 Da: the tolerance is half of it, 0.5 Da, and the match says where it comes from.
    // 0.62 Da from the feature is outside it: row 7 does not get the peak as its product (it is at most the isotope peak of it, which gives no scans)
    let far = ((o + 0.62) * 100.0).round() / 100.0;
    let d_far = one(d.clone(), "far.mzML", x(far, &[(7.4, 0.07, 5.0e4)]));
    assert!(
        links(&d_far)
            .links
            .iter()
            .all(|k| k.row != 7 || k.state == State::Isotope),
        "{:?}",
        links(&d_far).links
    );
    let near = ((o + 0.03) * 100.0).round() / 100.0;
    let d_near = one(d, "near.mzML", x(near, &[(7.4, 0.07, 5.0e4)]));
    let k = links(&d_near).links.iter().find(|k| k.row == 7).unwrap();
    assert!((k.dmz - (near - o).abs()).abs() < 0.001, "{}", k.dmz);
    assert!(
        (k.dppm - (near - o).abs() / o * 1e6).abs() < 0.1,
        "{}",
        k.dppm
    );
    assert_eq!((k.state, k.tol, k.tol_window), (State::Auto, 0.5, true));
    assert!(
        k.why.contains("half isolation window of the file"),
        "{}",
        k.why
    );
}

#[test]
fn the_scans_of_a_match_are_the_ones_inside_the_peak_of_the_feature() {
    let d = narrow(base().1.clone(), 7, 7.35, 7.45);
    let p = read_mz(&d, 7);
    // the trace peak is 0.2 min after the feature: no scan of it lies between the start and the end of the peak
    let late = one(d.clone(), "late.mzML", narrow_x(p, 7.6));
    assert!(
        links(&late).links.iter().all(|k| k.row != 7),
        "{:?}",
        links(&late).links
    );
    // on the feature: the scans are the ones inside 7.35..7.45 (a scan every 0.02 min), not the whole peak of the trace
    let on = one(d, "on.mzML", narrow_x(p, 7.4));
    let k = links(&on).links.iter().find(|k| k.row == 7).unwrap();
    assert!(
        (k.lo - 7.35).abs() < 1e-3 && (k.hi - 7.45).abs() < 1e-3,
        "{} {}",
        k.lo,
        k.hi
    );
    assert!((5..=6).contains(&k.n_in), "{}", k.n_in);
    assert!(links(&on).exps[0].peaks[k.peak].hi - links(&on).exps[0].peaks[k.peak].lo > 0.12);
}

#[test]
fn the_detail_shows_the_representative_scan_and_the_fragment_text() {
    let d = base().1.clone();
    let p = read_mz(&d, 7);
    let f = ms2_file("rep.mzML", &[x(p, &[(7.4, 0.07, 5.0e4)])]);
    let d = matched(d, &[("rep.mzML", None, f.clone())], &[]);
    let cols = &base().0;
    let mut op = |k: usize| {
        Ok(if k < cols.len() {
            open(&cols[k])
        } else {
            open(&f)
        })
    };
    let det: Value = serde_json::from_str(&detail(&d, 7, &mut op).unwrap()).unwrap();
    let r = &det["ms2_rep"];
    // the most intense scan of the peak is at its top; the others are counted
    assert!((r["rt"].as_f64().unwrap() - 7.4).abs() < 0.06, "{r}");
    let (n, others) = (r["n_in"].as_i64().unwrap(), r["others"].as_i64().unwrap());
    assert!(n >= 10 && others == n - 1, "{n} {others}");
    assert_eq!(r["intensity_src"], "scan TIC");
    assert!(
        r["dmz"].as_f64().unwrap() < 0.01 && r["drt"].as_f64().unwrap().abs() < 0.1,
        "{r}"
    );
    // fragments: P - 17 (strongest) and P - 44, with the loss in m/z
    let found = r["found"].as_array().unwrap();
    assert_eq!(found.len(), 2, "{r}");
    assert!((found[0]["loss"].as_f64().unwrap() - 17.0).abs() < 0.01);
    assert!((found[1]["loss"].as_f64().unwrap() - 44.0).abs() < 0.01);
    let text = det["ms2_text"].as_str().unwrap();
    assert!(text.starts_with("Fragment ions found: m/z "), "{text}");
    assert!(
        text.contains("loss \u{2212}\u{394}Pm/z = \u{2212}17.0000,"),
        "{text}"
    );
    assert_eq!(text.matches(" ; ").count(), 1, "{text}");
    // a row without a scan in its peak says so, and has no spectrum
    let det: Value = serde_json::from_str(&detail(&d, 10007, &mut op).unwrap()).unwrap();
    assert!(det["ms2_rep"].is_null());
    assert_eq!(det["ms2_text"], "No MS\u{b2} in the feature peak");
}

#[test]
fn the_text_of_a_scan_is_what_the_engine_computes_from_its_fragments() {
    let ions = |v: &[(f64, f64)]| -> Vec<ms2frag::Ion> {
        v.iter()
            .map(|(mz, intensity)| ms2frag::Ion {
                mz: *mz,
                intensity: *intensity,
                formula: None,
            })
            .collect()
    };
    let found = ms2frag::find(
        300.0,
        "C10H10N2O2",
        &ions(&[(283.0, 100.0), (256.0, 50.0)]),
        5.0,
        ms2frag::LOSS_TOL_LR,
    );
    assert_eq!(found.len(), 2);
    assert_eq!(
        ms2frag::text(&found)
            .matches("loss \u{2212}\u{394}Pm/z = \u{2212}")
            .count(),
        2
    );
}
