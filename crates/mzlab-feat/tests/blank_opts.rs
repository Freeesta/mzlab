//! The options of the blank rule (quantity, blank statistic, scope, subtraction) and the QC CV that counts the missing files as 0.

use mzlab_core::json::parse;
use mzlab_feat::filters::{self, Rule};
use mzlab_feat::filters_blank::{qc_cv_percent, BlankOpts, Quantity, Scope, Stat};
use mzlab_feat::norm::Method;
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::view;

/// Files: two blanks, two samples, three QC. `cells[row][file]` = (area, height).
fn table(kinds: &[&str], cells: &[Vec<(f64, f32)>]) -> (Aligned, Vec<Sample>) {
    let names: Vec<(String, String)> = (0..kinds.len())
        .map(|k| (format!("f{k}.mzML"), "00".repeat(32)))
        .collect();
    let mut s = propose(&names);
    for (x, k) in s.iter_mut().zip(kinds) {
        x.kind = (*k).into();
    }
    let mut t = Aligned::with_capacity(kinds.len(), cells.len());
    for (row, r) in cells.iter().enumerate() {
        t.mz.push(200.0 + row as f64);
        t.rt.push(5.0);
        for &(a, h) in r {
            t.area.push(a);
            t.height.push(h);
            t.apex_rt.push(5.0);
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    (t, s)
}

fn opts(quantity: Quantity, stat: Stat, scope: Scope, subtract: bool) -> BlankOpts {
    BlankOpts {
        quantity,
        stat,
        scope,
        subtract,
    }
}

const KINDS: [&str; 4] = ["blank", "blank", "sample", "sample"];

#[test]
fn blank_max_versus_mean() {
    // blanks 100 and 0: mean 50, max 100; best sample 250
    let (t, s) = table(
        &KINDS,
        &[vec![(100.0, 1.0), (0.0, 0.0), (250.0, 1.0), (10.0, 1.0)]],
    );
    let run = |st| {
        filters::apply(
            &t,
            &s,
            &[Rule::BlankWith(
                3.0,
                opts(Quantity::Area, st, Scope::MaxFeature, false),
            )],
        )
        .unwrap()
    };
    assert_eq!(run(Stat::Mean), vec![0]); // 250 >= 3 * 50
    assert_eq!(run(Stat::Max), vec![1]); // 250 < 3 * 100
}

#[test]
fn height_versus_area() {
    // by area the sample is 3 times the blank (kept), by height it is not
    let (t, s) = table(
        &KINDS,
        &[vec![
            (100.0, 50.0),
            (100.0, 50.0),
            (300.0, 60.0),
            (0.0, 0.0),
        ]],
    );
    let run = |q| {
        filters::apply(
            &t,
            &s,
            &[Rule::BlankWith(
                3.0,
                opts(q, Stat::Mean, Scope::MaxFeature, false),
            )],
        )
        .unwrap()
    };
    assert_eq!(run(Quantity::Area), vec![0]);
    assert_eq!(run(Quantity::Height), vec![1]);
}

#[test]
fn fold_three_and_fold_one_and_the_floor() {
    let (t, s) = table(
        &KINDS,
        &[vec![(100.0, 1.0), (100.0, 1.0), (200.0, 1.0), (0.0, 0.0)]],
    );
    let run = |f| {
        filters::apply(
            &t,
            &s,
            &[Rule::BlankWith(
                f,
                opts(Quantity::Area, Stat::Max, Scope::MaxFeature, false),
            )],
        )
        .unwrap()
    };
    assert_eq!(run(1.0), vec![0]);
    assert_eq!(run(3.0), vec![1]);
    assert_eq!(run(0.2), vec![0], "fold under 1 is read as 1");
    assert_eq!(run(2.0), vec![0], ">= at the limit");
}

#[test]
fn each_feature_needs_every_sample_to_clear_the_blank() {
    let (t, s) = table(
        &KINDS,
        &[vec![(100.0, 1.0), (100.0, 1.0), (500.0, 1.0), (150.0, 1.0)]],
    );
    let run = |sc| {
        filters::apply(
            &t,
            &s,
            &[Rule::BlankWith(
                2.0,
                opts(Quantity::Area, Stat::Mean, sc, false),
            )],
        )
        .unwrap()
    };
    assert_eq!(run(Scope::MaxFeature), vec![0]);
    assert_eq!(run(Scope::EachFeature), vec![1]);
}

#[test]
fn subtraction_reaches_zero_and_leaves_the_blanks() {
    let (t, s) = table(
        &KINDS,
        &[vec![(100.0, 1.0), (100.0, 1.0), (350.0, 1.0), (250.0, 1.0)]],
    );
    let rule = Rule::BlankWith(
        2.0,
        opts(Quantity::Area, Stat::Mean, Scope::MaxFeature, true),
    );
    let v = view::build(&t, &s, &Method::None, &[rule]).unwrap();
    assert_eq!(v.rows, vec![0]);
    assert_eq!(v.area_at(&t, 0, 2), Some(150.0));
    assert_eq!(v.area_at(&t, 0, 3), Some(50.0));
    assert_eq!(v.area_at(&t, 0, 0), Some(100.0));
    let rule = Rule::BlankWith(
        3.0,
        opts(Quantity::Area, Stat::Mean, Scope::MaxFeature, true),
    );
    let v = view::build(&t, &s, &Method::None, &[rule]).unwrap();
    assert_eq!(v.area_at(&t, 0, 3), Some(0.0), "never below 0");
    let h = Rule::BlankWith(
        3.0,
        opts(Quantity::Height, Stat::Mean, Scope::MaxFeature, true),
    );
    assert!(view::build(&t, &s, &Method::None, &[h]).is_err());
}

#[test]
fn defaults_are_the_plain_rules_in_json() {
    let v = parse(
        br#"[{"regola":"bianco","valore":3},{"regola":"qc_cv_max","valore":20,"qc_min_files":3}]"#,
    )
    .unwrap();
    let rules = filters::from_json(&v).unwrap();
    assert_eq!(rules, vec![Rule::Blank(3.0), Rule::QcCv(20.0, 3)]);
    let again = parse(filters::to_json(&rules).as_bytes()).unwrap();
    assert_eq!(filters::from_json(&again).unwrap(), rules);
    let v = parse(br#"[{"regola":"bianco","valore":0.5,"statistica":"massimo","sottrai":true},{"regola":"qc_cv_max","valore":20,"qc_zero":true}]"#).unwrap();
    let rules = filters::from_json(&v).unwrap();
    assert_eq!(
        rules[0],
        Rule::BlankWith(
            1.0,
            opts(Quantity::Area, Stat::Max, Scope::MaxFeature, true)
        )
    );
    assert_eq!(rules[1], Rule::QcCvZero(20.0, 3));
    let again = parse(filters::to_json(&rules).as_bytes()).unwrap();
    assert_eq!(filters::from_json(&again).unwrap(), rules);
}

#[test]
fn qc_cv_counts_a_missing_file_as_zero_and_needs_two_qc() {
    let kinds = ["qc", "qc", "qc", "sample"];
    let (t, _) = table(
        &kinds,
        &[vec![(100.0, 1.0), (100.0, 1.0), (0.0, 0.0), (1.0, 1.0)]],
    );
    // values 100, 100, 0: mean 66.67, sample sd 57.74 -> 86.6 %
    let cv = qc_cv_percent(&t, 0, &[0, 1, 2]).unwrap();
    assert!((cv - 86.602_540).abs() < 1e-4, "{cv}");
    assert_eq!(
        qc_cv_percent(&t, 0, &[0]),
        None,
        "one QC file: empty, not 0"
    );
    assert_eq!(qc_cv_percent(&t, 0, &[]), None);
    let (t, s) = table(
        &kinds,
        &[vec![(100.0, 1.0), (100.0, 1.0), (0.0, 0.0), (1.0, 1.0)]],
    );
    assert_eq!(
        filters::apply(&t, &s, &[Rule::QcCvZero(80.0, 3)]).unwrap(),
        vec![1]
    );
    assert_eq!(
        filters::apply(&t, &s, &[Rule::QcCvZero(90.0, 3)]).unwrap(),
        vec![0]
    );
    // the old rule needs 3 peaks and rejects the row regardless
    assert_eq!(
        filters::apply(&t, &s, &[Rule::QcCv(90.0, 3)]).unwrap(),
        vec![1]
    );
}
