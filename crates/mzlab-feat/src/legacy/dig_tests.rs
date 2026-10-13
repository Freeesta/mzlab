//! Tests of `legacy::dig`.

use super::*;

/// A table without TP view: the parent is the row at its m/z, not row 0 (the default `parent_row` of an empty `tp/1` block).
#[test]
fn parent_row_without_tp_view() {
    let p = super::super::series::parent(r#"{"name": "p", "neutral": "C21H32N2O2"}"#).unwrap();
    let t = Aligned {
        n_files: 2,
        mz: vec![200.1, p.mz + 0.0003, p.mz - 0.0002],
        rt: vec![2.62, 1.0, 4.0],
        area: vec![9e9, 9e9, 3e8, 2e8, 1e5, 1e5],
        ..Aligned::default()
    };
    assert!(t.tp.is_empty() && t.tp.parent_row == 0);
    assert_eq!(parent_row(&t, &p), Some(1));
}
