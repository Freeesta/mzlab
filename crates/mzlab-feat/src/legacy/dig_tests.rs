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

/// The adduct of the parent sets its ion formula and m/z; the default is [M+H]+ and an unknown one is refused.
#[test]
fn parent_adducts() {
    let par = |a: &str| {
        super::super::series::parent(&format!(r#"{{"neutral": "C10H18O5", "adduct": "{a}"}}"#))
    };
    let h = par("[M+H]+").unwrap();
    assert_eq!(h.ion_formula(), "C10H19O5");
    let na = par("[M+Na]+").unwrap();
    assert_eq!(na.ion_formula(), "C10H18NaO5");
    assert!((na.mz - h.mz - 21.98194).abs() < 1e-4, "{} {}", na.mz, h.mz);
    assert_eq!(par("[M+NH4]+").unwrap().ion_formula(), "C10H22NO5");
    assert_eq!(par("[M-H]-").unwrap().ion_formula(), "C10H17O5");
    assert_eq!(par("[M+HCOO]-").unwrap().ion_formula(), "C11H19O7");
    assert!(par("[M+Xx]+").is_err());
    let d = super::super::series::parent(r#"{"neutral": "C10H18O5"}"#).unwrap();
    assert_eq!((d.adduct.as_str(), d.mz), ("[M+H]+", h.mz));
}
