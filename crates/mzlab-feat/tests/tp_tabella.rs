//! The table of the products (`tptab/1`): which rows it keeps, the numbers of a row, the controls read from the roles of the samples, and the error without the block.

use mzlab_core::json::parse;
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::tp::*;

const MINUTES: [f64; 5] = [f64::NAN, 0.0, 5.0, 10.0, 20.0];

fn samples() -> Vec<Sample> {
    let names = [
        "blank_1.mzML",
        "t0_a.mzML",
        "t5.mzML",
        "t10.mzML",
        "t20.mzML",
    ];
    propose(
        &names
            .iter()
            .map(|n| (n.to_string(), String::new()))
            .collect::<Vec<_>>(),
    )
}

/// Rows: the parent, a product that forms, one in the blank, one that is also in the t0 and grows.
fn table() -> Aligned {
    let rows: [(f64, f64, [f64; 5]); 4] = [
        (300.0, 5.0, [0.0, 1e7, 8e6, 5e6, 2e6]),
        (316.0, 4.0, [0.0, 0.0, 1e5, 4e5, 9e5]),
        (330.0, 6.0, [5e5, 4e5, 5e5, 6e5, 7e5]),
        (332.0, 4.5, [0.0, 1e4, 2e5, 5e5, 8e5]),
    ];
    let mut t = Aligned::with_capacity(5, rows.len());
    for (mz, rt, a) in rows {
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

fn with_block() -> Aligned {
    let mut t = table();
    let p = TpParams::default();
    let inp = TpInputs {
        aligned: &t,
        minutes: &MINUTES,
        hyps: &[],
        signs: &[[0; 10]; 4],
        parent: (300.0, 5.0),
        fragments: None,
        params: &p,
        rules: None,
    };
    let b = build(&inp).unwrap().expect("parent in the table");
    t.tp = b;
    t
}

#[test]
fn rows_numbers_and_controls() {
    let t = with_block();
    let v = parse(table_json(&t, &samples(), None).unwrap().as_bytes()).unwrap();
    assert_eq!(v.get("schema").and_then(|x| x.str()), Some("tptab/1"));
    assert_eq!(v.get("parent").and_then(|x| x.f64()), Some(1.0));
    let times: Vec<f64> = v
        .get("times")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|x| x.f64())
        .collect();
    assert_eq!(times, vec![5.0, 10.0, 20.0]);
    let rows = v.get("rows").unwrap().arr();
    let by = |id: f64| {
        rows.iter()
            .find(|r| r.get("id").and_then(|x| x.f64()) == Some(id))
    };
    // the product of row 2 forms: maximum at 20 min, absent from t0 and blank
    let p = by(2.0).expect("product kept");
    assert_eq!(p.get("max_area").and_then(|x| x.f64()), Some(9e5));
    assert_eq!(p.get("tmax").and_then(|x| x.f64()), Some(20.0));
    assert_eq!(p.get("in_t0").and_then(|x| x.bool()), Some(false));
    assert_eq!(p.get("in_blank").and_then(|x| x.bool()), Some(false));
    let s: Vec<f64> = p
        .get("series")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|x| x.f64())
        .collect();
    assert_eq!(s, vec![1e5, 4e5, 9e5]);
    // the contaminant of the blank is not a product (the gate drops it) so it is not kept
    assert!(by(3.0).is_none());
    // row 4 also in the t0 (small) still forms: kept, and flagged as in the t0
    let q = by(4.0).expect("product also in t0");
    assert_eq!(q.get("in_t0").and_then(|x| x.bool()), Some(true));
    assert_eq!(q.get("in_blank").and_then(|x| x.bool()), Some(false));
}

#[test]
fn without_block_is_an_error() {
    let e = table_json(&table(), &samples(), None).unwrap_err();
    assert_eq!(e.key, "error.tpNoBlock");
}

#[test]
fn roles_have_one_reader() {
    let s = samples();
    let r: Vec<Role> = s.iter().map(role_of).collect();
    assert_eq!(
        r,
        vec![
            Role::Blank,
            Role::T0,
            Role::Treated(5.0),
            Role::Treated(10.0),
            Role::Treated(20.0)
        ]
    );
}

#[test]
fn given_rows_need_no_block() {
    let v = parse(
        table_json(&table(), &samples(), Some(&[2, 1]))
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    let rows = v.get("rows").unwrap().arr();
    assert_eq!(rows.len(), 2);
    // in the order asked, the contaminant of the blank is there and says so, the status is null without the block
    assert_eq!(rows[0].get("row").and_then(|x| x.f64()), Some(2.0));
    assert_eq!(rows[0].get("in_blank").and_then(|x| x.bool()), Some(true));
    assert!(rows[0].get("status").is_some_and(|x| x.is_null()));
    assert!(v.get("parent").is_some_and(|x| x.is_null()));
    assert_eq!(
        table_json(&table(), &samples(), Some(&[9]))
            .unwrap_err()
            .key,
        "error.badView"
    );
}
