//! The duplicate rule of the filters (`duplicati`): of the rows within the m/z and RT window only the strongest stays, a chain of near rows keeps
//! its strong ends, and the rule goes to JSON and back.

use mzlab_core::json::parse;
use mzlab_feat::filters::{self, Rule};
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

/// One sample file; rows `(m/z, RT min, area)`.
fn table(rows: &[(f64, f64, f64)]) -> (Aligned, Vec<Sample>) {
    let mut s = propose(&[("f.mzML".to_string(), "00".repeat(32))]);
    s[0].kind = "sample".into();
    let mut t = Aligned::with_capacity(1, rows.len());
    for &(mz, rt, area) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        t.area.push(area);
        t.height.push((area / 10.0) as f32);
        t.apex_rt.push(rt as f32);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    (t, s)
}

#[test]
fn only_the_strongest_of_a_near_group_stays() {
    let (t, s) = table(&[
        (300.1000, 5.00, 1e5),
        (300.1010, 5.05, 9e5), // 3.3 ppm and 0.05 min from row 0: the strongest
        (300.1000, 6.00, 2e5), // same m/z, RT too far
        (310.0000, 5.00, 3e5), // m/z too far
    ]);
    let why = filters::apply(&t, &s, &[Rule::Duplicates(5.0, 0.1)]).unwrap();
    assert_eq!(why, vec![1, 0, 0, 0]);
    // a narrow window keeps everything
    assert_eq!(
        filters::apply(&t, &s, &[Rule::Duplicates(1.0, 0.1)]).unwrap(),
        vec![0; 4]
    );
}

#[test]
fn a_chain_keeps_both_ends_and_a_tie_keeps_the_first_row() {
    // 0 and 2 are 1.2 min apart: not duplicates; 1 sits between and loses to the stronger 0
    let (t, s) = table(&[(200.0, 5.0, 5e5), (200.0, 5.6, 4e5), (200.0, 6.2, 3e5)]);
    let why = filters::apply(&t, &s, &[Rule::Duplicates(5.0, 0.7)]).unwrap();
    assert_eq!(why, vec![0, 1, 0]);
    let (t, s) = table(&[(200.0, 5.0, 1e5), (200.0, 5.0, 1e5)]);
    assert_eq!(
        filters::apply(&t, &s, &[Rule::Duplicates(5.0, 0.1)]).unwrap(),
        vec![0, 1]
    );
}

#[test]
fn the_rule_goes_to_json_and_back() {
    let rules = vec![Rule::Duplicates(5.0, 0.1)];
    let text = filters::to_json(&rules);
    assert!(text.contains("\"duplicati\""));
    assert_eq!(
        filters::from_json(&parse(text.as_bytes()).unwrap()).unwrap(),
        rules
    );
}
