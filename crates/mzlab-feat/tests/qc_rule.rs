//! The QC rule of the filters (`qc_cv_max`): the CV of the areas over the `qc` files, kept or rejected per row, the error with too few QC files, and
//! the rule in JSON and back.

use mzlab_core::json::parse;
use mzlab_feat::filters::{self, Rule};
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};

/// `n_qc` QC files followed by one sample; rows: stable QC (CV about 2 %), drifting QC (about 40 %), QC seen in one file only, QC absent.
fn series(n_qc: usize) -> (Aligned, Vec<Sample>) {
    let n = n_qc + 1;
    let names: Vec<(String, String)> = (0..n)
        .map(|k| (format!("f{k}.mzML"), "00".repeat(32)))
        .collect();
    let mut s = propose(&names);
    for (k, x) in s.iter_mut().enumerate() {
        x.kind = if k < n_qc { "qc" } else { "sample" }.into();
    }
    let area = |row: usize, k: usize| -> f64 {
        let wiggle = if k.is_multiple_of(2) { 1.0 } else { -1.0 };
        match row {
            0 => 1e6 * (1.0 + 0.02 * wiggle),
            1 => 1e6 * (1.0 + 0.5 * wiggle),
            2 if k == 0 => 1e6,
            _ => 0.0,
        }
    };
    let mut t = Aligned::with_capacity(n, 4);
    for row in 0..4 {
        t.mz.push(200.0 + row as f64);
        t.rt.push(5.0);
        for k in 0..n {
            let a = area(row, k);
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(5.0);
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    (t, s)
}

#[test]
fn six_qc_files_keep_the_stable_row_and_say_why_the_others_go() {
    let (t, s) = series(6);
    let why = filters::apply(&t, &s, &[Rule::QcCv(20.0, 3)]).unwrap();
    // stable kept; drifting and the one seen in one QC file and the absent one rejected by rule 1
    assert_eq!(why, vec![0, 1, 1, 1]);
    // a looser limit keeps the drifting row; the rule after the QC rule counts the rows the QC rule let through
    let why = filters::apply(&t, &s, &[Rule::QcCv(80.0, 3), Rule::MinArea(2e6)]).unwrap();
    assert_eq!(why, vec![2, 2, 1, 1]);
}

#[test]
fn too_few_qc_files_is_an_error_not_a_number() {
    for n_qc in [0, 2] {
        let (t, s) = series(n_qc);
        let e = filters::apply(&t, &s, &[Rule::QcCv(20.0, 3)]).unwrap_err();
        assert_eq!(e.key, "error.qc.tooFew", "{n_qc} QC files");
    }
    // two QC files are enough when the rule asks for two
    let (t, s) = series(2);
    assert!(filters::apply(&t, &s, &[Rule::QcCv(20.0, 2)]).is_ok());
    // one QC file cannot give a CV whatever the rule asks
    let (t, s) = series(1);
    assert_eq!(
        filters::apply(&t, &s, &[Rule::QcCv(20.0, 0)])
            .unwrap_err()
            .key,
        "error.qc.tooFew"
    );
    // without the rule nothing is checked
    let (t, s) = series(0);
    assert!(filters::apply(&t, &s, &[Rule::MinArea(1.0)]).is_ok());
}

#[test]
fn the_rule_goes_to_json_and_back() {
    let rules = vec![Rule::QcCv(25.5, 4), Rule::MinArea(1e5)];
    let j = filters::to_json(&rules);
    assert!(
        j.contains("\"regola\":\"qc_cv_max\",\"valore\":25.5,\"qc_min_files\":4"),
        "{j}"
    );
    assert_eq!(
        filters::from_json(&parse(j.as_bytes()).unwrap()).unwrap(),
        rules
    );
    // the minimum is 3 when the operation does not say
    let v = parse(br#"[{"regola":"qc_cv_max","valore":30}]"#).unwrap();
    assert_eq!(filters::from_json(&v).unwrap(), vec![Rule::QcCv(30.0, 3)]);
}
