//! The two failures of a long kinetic campaign on synthetic series: a plan with a standard (the Flow table aligns every file, the Dig used to read
//! one column less and stop with "the series has 16 files, the Dig 15") and one ion split into two rows by the alignment (the kinetic of each piece
//! has zeros where the other holds the peak). Also the evidence of each row: controls, S/N per point, its state (detected, forced, filled, absent), points used for k.

use mzlab_feat::legacy::lr::{self, summary::summary};
use mzlab_feat::EVIDENCE;
use mzlab_feat_wasm::run;
use serde_json::Value;

mod common;
use common::*;

/// The synthetic series with a standard of the parent (no time) after the blank.
fn with_standard() -> (String, Vec<Vec<u8>>) {
    let (files, mut idxs) = series();
    let mut list: Vec<Value> = serde_json::from_str(&files).unwrap();
    list.push(serde_json::json!({"name": "lr_std.mzML", "time": null, "type": "standard", "kind": "full"}));
    // the standard holds the parent only, as a t0 would
    idxs.push(index_of(Some(0.0), 777));
    (serde_json::to_string(&list).unwrap(), idxs)
}

fn sum(d: &lr::LrDig) -> Value {
    serde_json::from_str(&summary(d, "{}")).unwrap()
}

#[test]
fn a_plan_with_a_standard_is_dug_and_the_standard_is_a_control() {
    let (files, idxs) = with_standard();
    let d = dig(&files, &idxs);
    assert_eq!(d.lc.len(), 9, "the standard is a column of the table");
    let s = sum(&d);
    let parent = &s["rows"][0];
    assert_eq!(parent["id"], 0);
    // never a kinetic point: the seven times only
    assert_eq!(parent["kinetics"].as_array().unwrap().len(), TIMES.len());
    let std = parent["controls"]["standard"].as_f64().unwrap();
    assert!(std > 0.3, "the parent is in its standard: {std}");
    assert!(parent["controls"]["blank"].is_number());
    assert!(
        parent["controls"]["dark"].is_null(),
        "no dark control in the plan"
    );
    let roles: Vec<&str> = parent["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles.iter().filter(|r| **r == "std").count(), 1);
    assert_eq!(roles.iter().filter(|r| **r == "blank").count(), 1);
}

#[test]
fn a_table_that_matches_no_files_of_the_plan_gives_a_clear_error() {
    let (files, idxs) = with_standard();
    let mut list: Vec<Value> = serde_json::from_str(&files).unwrap();
    list.remove(3);
    let short = serde_json::to_string(&list).unwrap();
    let names: Vec<String> = (0..idxs.len()).map(|k| format!("f{k}.mzML")).collect();
    let r = run::recipe_for("", &mut open(&idxs[0])).unwrap();
    let stored: Vec<Vec<u8>> = idxs
        .iter()
        .zip(&names)
        .map(|(b, n)| run::detect(&mut open(b), n, &r).unwrap())
        .collect();
    let (feats, origin) = run::read_files(&stored).unwrap();
    let table = run::align(&feats, &origin, &r, |k| Ok(open(&idxs[k]))).unwrap();
    let e = lr::dig(&short, PARENT, "{}", &table, "", &mut |k| {
        Ok(open(&idxs[k]))
    })
    .unwrap_err();
    assert!(
        e.starts_with("the feature table has 9 file columns, but the plan lists 8 full-scan files (6 sample, 1 blank, 1 standard)"),
        "{e}"
    );
}

/// A product at m/z 270.1 that forms at k = 0.08 1/min and drifts in RT late in the run (6.0 min up to 15 min, 6.25 min after), so the
/// alignment cuts it in two rows.
fn drifting(t: Option<f64>) -> Vec<(f64, f64, f64, f64)> {
    let Some(k) = t else { return Vec::new() };
    let rt = if k <= 15.0 { 6.0 } else { 6.25 };
    vec![(270.1, rt, 0.05, 1.0e6 * (1.0 - (-0.08 * k).exp()))]
}

fn rows_near(s: &Value, mz: f64) -> Vec<Value> {
    s["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            (r["mz"].as_f64().unwrap() - mz).abs() < 0.5 && r["max_area"].as_f64().unwrap() > 0.0
        })
        .cloned()
        .collect()
}

#[test]
fn one_ion_split_by_the_alignment_is_merged_before_the_kinetics() {
    let (files, idxs) = series_with(&drifting);
    let off = dig_with(&files, PARENT, r#"{"merge_da": 0}"#, &idxs);
    let split = rows_near(&sum(&off), 270.1);
    assert!(off.merged.is_empty());
    assert!(
        split.len() >= 2,
        "the alignment splits the product: {split:#?}"
    );
    let most = split
        .iter()
        .map(|r| r["k_points"].as_u64().unwrap())
        .max()
        .unwrap();
    assert!(most < 6, "each piece holds part of the kinetic: {most}");
    let on = dig(&files, &idxs);
    let s = sum(&on);
    let joined = rows_near(&s, 270.1);
    assert_eq!(joined.len(), 1, "{joined:#?}");
    assert_eq!(
        joined[0]["k_points"], 6,
        "the six treated times, t0 has no peak"
    );
    // the merge is logged with both rows as they were
    let m = s["merged"].as_array().unwrap();
    let mine: Vec<&Value> = m
        .iter()
        .filter(|x| (x["kept"]["mz"].as_f64().unwrap() - 270.4).abs() < 0.5)
        .collect();
    assert_eq!(mine.len(), 1, "{m:#?}");
    assert!(mine[0]["reason"]
        .as_str()
        .unwrap()
        .starts_with("same ion split by the alignment"));
    assert_eq!(
        mine[0]["absorbed"]["areas"].as_array().unwrap().len(),
        on.lc.len()
    );
    assert_eq!(joined[0]["table_row"], mine[0]["kept"]["row"]);
    assert_eq!(s["settings"]["merge_rt_min"], 0.3);
}

#[test]
fn every_point_carries_its_snr_and_whether_it_is_a_detected_peak() {
    let (files, idxs) = series();
    let s = sum(&dig(&files, &idxs));
    let parent = &s["rows"][0];
    let pts = parent["points"].as_array().unwrap();
    assert_eq!(pts.len(), TIMES.len() + 1);
    for p in pts
        .iter()
        .filter(|p| p["role"] == "sample" || p["role"] == "t0")
    {
        assert_eq!(p["state"], "detected", "{p}");
        assert!(p["snr"].as_f64().unwrap() >= 5.0, "{p}");
    }
    let blank = pts.iter().find(|p| p["role"] == "blank").unwrap();
    assert_ne!(blank["state"], "detected");
    assert_eq!(parent["k_points"], TIMES.len());
}

/// A product at m/z 270.1 that forms in the treated samples; the blank holds only a weak, broad trace of it (`weak` = its amplitude).
fn weak_in_blank(weak: f64) -> impl Fn(Option<f64>) -> Vec<(f64, f64, f64, f64)> {
    move |t| match t {
        Some(k) => vec![(270.1, 6.0, 0.05, 1.0e6 * (1.0 - (-0.08 * k).exp()))],
        None => vec![(270.1, 6.0, 0.5, weak)],
    }
}

#[test]
fn a_weak_trace_found_only_by_forced_integration_is_never_detected() {
    // amplitudes that give the S/N 3-5 of forced noise integration in a blank
    for (w, lo, hi) in [(1.2e4, 3.0, 4.0), (2.0e4, 4.0, 5.0)] {
        let (files, idxs) = series_with(&weak_in_blank(w));
        let s = sum(&dig(&files, &idxs));
        let rows = rows_near(&s, 270.1);
        assert_eq!(rows.len(), 1, "{rows:#?}");
        let r = &rows[0];
        let pts = r["points"].as_array().unwrap();
        assert!(pts
            .iter()
            .all(|p| EVIDENCE.contains(&p["state"].as_str().unwrap())));
        let b = pts.iter().find(|p| p["role"] == "blank").unwrap();
        let snr = b["snr"].as_f64().unwrap();
        assert_eq!(b["state"], "forced", "{b}");
        assert!((lo..hi).contains(&snr), "{b}");
        assert!(
            b["area"].as_f64().unwrap() > 0.0,
            "the area stays readable: {b}"
        );
        assert_eq!(
            r["controls"]["blank"], 0.0,
            "a forced area is no peak in the blank"
        );
        assert_eq!(r["k_points"], 6);
    }
    // the limits are the user's: at S/N 3 and a FWHM up to 2 min the same broad blank peak is detected and counts as a control
    let (files, idxs) = series_with(&weak_in_blank(1.2e4));
    let s = sum(&dig_with(
        &files,
        PARENT,
        r#"{"min_snr": 3, "max_fwhm_s": 120}"#,
        &idxs,
    ));
    let r = &rows_near(&s, 270.1)[0];
    let b = r["points"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["role"] == "blank")
        .unwrap();
    assert_eq!(b["state"], "detected", "{b}");
    assert!(r["controls"]["blank"].as_f64().unwrap() > 0.0, "{r}");
}

/// Two isobaric products at m/z 270.1, the second `d` min later: formed together, one handing over to the other (both present in the middle
/// times), or one after the other with no time in common (the pattern a split ion also has).
fn isobars(kind: &str, d: f64) -> impl Fn(Option<f64>) -> Vec<(f64, f64, f64, f64)> + '_ {
    move |t| {
        let Some(k) = t else { return Vec::new() };
        let (a, b) = match kind {
            "together" => (
                1.0e6 * (1.0 - (-0.08 * k).exp()),
                6.0e5 * (1.0 - (-0.03 * k).exp()),
            ),
            "hand-off" => (
                2.0e6 * (-0.06 * k).exp() * (1.0 - (-0.3 * k).exp()),
                1.0e6 * (1.0 - (-0.02 * k).exp()).powi(2),
            ),
            _ if k <= 15.0 => (1.0e6 * (1.0 - (-0.08 * k).exp()), 0.0),
            _ => (0.0, 1.0e6 * (1.0 - (-0.08 * k).exp())),
        };
        [(6.0, a), (6.0 + d, b)]
            .into_iter()
            .filter(|p| p.1 > 0.0)
            .map(|(rt, amp)| (270.1, rt, 0.05, amp))
            .collect()
    }
}

#[test]
fn isobaric_products_0_3_to_0_5_min_apart_are_never_merged() {
    // the evidence for the default merge_rt_min 0.3 (docs/agenti/mzfinder-lr.md): two real products this far apart stay two rows
    for kind in ["together", "hand-off", "apart"] {
        for d in [0.3, 0.4, 0.5] {
            let (files, idxs) = series_with(&isobars(kind, d));
            let s = sum(&dig(&files, &idxs));
            let mine: Vec<&Value> = s["merged"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| (x["kept"]["mz"].as_f64().unwrap() - 270.4).abs() < 0.5)
                .collect();
            assert!(mine.is_empty(), "{kind} {d}: {mine:#?}");
            if kind != "together" {
                assert_eq!(rows_near(&s, 270.1).len(), 2, "{kind} {d}");
            }
        }
    }
}
