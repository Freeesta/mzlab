//! Normalisation, filters and the view of a table: factors per file, rows rejected by the first rule that fails, the parameters in JSON and back,
//! the same view for the same inputs.

use mzlab_core::json::parse;
use mzlab_feat::filters::{self, Rule};
use mzlab_feat::norm::{self, Method, Reference};
use mzlab_feat::samples::propose;
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED, CELL_FILLED};
use mzlab_feat::view;

const NAMES: [&str; 4] = ["blank.mzML", "s_t0.mzML", "s_t10.mzML", "s_t20.mzML"];

/// Rows (mz, rt, areas of the four files; 0 = absent): a standard, a product, a contaminant that is in the blank, and noise seen once.
fn table() -> Aligned {
    let rows: [(f64, f64, [f64; 4]); 4] = [
        (200.0, 5.0, [1e6, 1e6, 2e6, 4e6]),
        (250.1, 6.0, [0.0, 0.0, 5e5, 9e5]),
        (300.2, 7.0, [8e5, 9e5, 8e5, 9e5]),
        (350.3, 8.0, [0.0, 0.0, 0.0, 1e3]),
    ];
    let mut t = Aligned::with_capacity(4, rows.len());
    for (mz, rt, areas) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for a in areas {
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state.push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

fn samples() -> Vec<mzlab_feat::samples::Sample> {
    let files: Vec<(String, String)> = NAMES.iter().map(|n| (n.to_string(), "00".repeat(32))).collect();
    propose(&files)
}

#[test]
fn factors_scale_each_file_to_the_target() {
    let (t, s) = (table(), samples());
    assert_eq!(norm::factors(&t, &s, &Method::None).unwrap(), vec![1.0; 4]);
    let sum = norm::factors(&t, &s, &Method::Sum).unwrap();
    // the target is the mean total of the files that decide (not the blank): after the scale they have it
    let total = |k: usize| (0..t.len()).filter_map(|r| norm::normalised(&t, &sum, r, k)).sum::<f64>();
    assert!((total(1) - total(2)).abs() < 1e-6 && (total(2) - total(3)).abs() < 1e-6);
    let med = norm::factors(&t, &s, &Method::Median).unwrap();
    assert!(med.iter().all(|f| f.is_finite() && *f > 0.0));
}

#[test]
fn the_standard_is_a_row_or_the_nearest_ion() {
    let (t, s) = (table(), samples());
    let by_id = norm::factors(&t, &s, &Method::Istd(Reference::Id(3))).unwrap();
    let near = Reference::Nearest { mz: 300.2001, rt: 7.05, ppm: 5.0, rt_tol: 0.2 };
    assert_eq!(by_id, norm::factors(&t, &s, &Method::Istd(near)).unwrap());
    // the standard is the same in the files that decide, so their normalised area is the same
    assert!((norm::normalised(&t, &by_id, 2, 1).unwrap() - norm::normalised(&t, &by_id, 2, 3).unwrap()).abs() < 1e-6);
    // a standard that does not exist is refused, and so is one that is not found in a file: NaN, not a guess
    assert!(norm::factors(&t, &s, &Method::Istd(Reference::Id(99))).is_err());
    let missing = norm::factors(&t, &s, &Method::Istd(Reference::Id(2))).unwrap();
    assert!(missing[0].is_nan() && missing[1].is_nan() && missing[2].is_finite());
}

#[test]
fn a_row_is_rejected_by_the_first_rule_it_fails() {
    let (t, s) = (table(), samples());
    let rules = [Rule::MinDetected(2), Rule::Blank(3.0), Rule::MinArea(1e5)];
    let why = filters::apply(&t, &s, &rules).unwrap();
    // 1: kept; 2: in only two files, kept; 3: a contaminant (as high in the samples as in the blank), rejected by the blank rule; 4: once
    assert_eq!(why, vec![0, 0, 2, 1]);
    let v = view::build(&t, &s, &Method::Sum, &rules).unwrap();
    assert_eq!(v.rows, vec![0, 1]);
    assert_eq!(v.area(&t, 1, 0), None, "the blank has no peak there");
    assert!(v.area(&t, 0, 2).is_some());
    assert!(v.counts_json().contains("\"tenute\":2") && v.counts_json().starts_with("{\"righe\":4"));
}

#[test]
fn a_rule_that_cannot_be_read_is_an_error() {
    let (t, mut s) = (table(), samples());
    assert!(filters::apply(&t, &s, &[Rule::Ms2]).is_err(), "no summaries in a table made by hand");
    s[0].kind = "sample".into();
    assert!(filters::apply(&t, &s, &[Rule::Blank(3.0)]).is_err(), "no blank");
    assert!(filters::apply(&t, &s[..3], &[]).is_err(), "samples and files do not match");
}

#[test]
fn filled_cells_count_as_measured_and_the_spec_round_trips() {
    let (mut t, s) = (table(), samples());
    t.state[3 + 3] = CELL_FILLED; // row 2, file 3
    assert!(norm::measured(&t, 1, 3).is_some());
    let rules = vec![Rule::Mz(100.0, 300.1), Rule::Rt(4.0, 7.5), Rule::MinHeight(1.0)];
    let v = view::build(&t, &s, &Method::Median, &rules).unwrap();
    let again = view::from_spec(&t, &s, v.spec_json().as_bytes()).unwrap();
    assert_eq!(v, again);
    assert_eq!(v.digest(&t), again.digest(&t));
    let other = view::build(&t, &s, &Method::Sum, &rules).unwrap();
    assert_ne!(v.digest(&t), other.digest(&t));
    assert!(parse(v.spec_json().as_bytes()).is_ok());
    assert!(view::from_spec(&t, &s, br#"{"filtra":[{"regola":"inventata"}]}"#).is_err());
    assert_eq!(view::from_spec(&t, &s, b"{}").unwrap().rows.len(), 4);
}

#[test]
fn qc_and_report_hold_only_numbers() {
    use mzlab_feat::{qc, report};
    let (t, mut s) = (table(), samples());
    // two replicates of one condition: the files 2 and 3 share the type, time and concentration
    s[2].time_min = Some(10.0);
    s[3].time_min = Some(10.0);
    let files = qc::per_file(&t);
    assert_eq!((files[0].detected, files[0].absent), (2, 2));
    assert!((files[3].total - (4e6 + 9e5 + 9e5 + 1e3)).abs() < 1e-6);
    let g = qc::groups(&t, &s, None);
    assert_eq!(g.len(), 1);
    // the rows with a peak in both: 1, 2 and 3; their cv: 1/3, 0.28.., 0.058..
    assert_eq!(g[0].rows, 3);
    assert!(g[0].cv_median > 0.2 && g[0].cv_median < 0.3, "{}", g[0].cv_median);
    let v = view::build(&t, &s, &Method::Istd(Reference::Id(3)), &[Rule::MinDetected(2)]).unwrap();
    let sd = qc::standard_cv(&t, &s, &v.method).unwrap();
    assert!(sd > 0.0 && sd < 0.1);
    let doc = report::to_json(&t, &s, Some(&v), &"ab".repeat(32));
    let d = parse(doc.as_bytes()).unwrap();
    assert_eq!(d.get("schema").and_then(|x| x.str()), Some("report/1"));
    assert_eq!(d.get("righe").and_then(|x| x.f64()), Some(4.0));
    assert_eq!(d.get("celle").unwrap().get("rilevate").and_then(|x| x.f64()), Some(11.0));
    assert_eq!(doc, report::to_json(&t, &s, Some(&v), &"ab".repeat(32)));
    assert!(report::to_json(&t, &s, None, "x").contains("\"vista\":null"));
    assert!(report::header_lines(&t, &s, Some(&v), "x").iter().all(|l| l.starts_with("# ")));
}
