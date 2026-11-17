//! `fragtree.hr` of a Dig: the tree of the DDA MS2 of the parent. It reads the Dig, so it stays in `legacy` while the tree itself lives in
//! `crate::finder_base::fragtree`; `legacy::fragtree::of_dig` re-exports it.

use crate::finder_base::fragtree::{elements_of, two_level, vec_of, TwoLevel};

/// `fragtree.hr` of a Dig (no MSn file): the tree of the DDA MS2 of the parent, or the note when there is none.
pub fn of_dig(d: &super::dig::Dig) -> String {
    let Some(p2) = d.parent_ms2.as_ref().filter(|c| !c.mz.is_empty()) else {
        return r#"{"ok":false,"mode":"hr","note":"no MSn file and no MS2 of the parent: no fragmentation tree","nodes":[]}"#.into();
    };
    let ion = d.parent.ion_formula();
    let els = elements_of(&ion).unwrap_or_default();
    let vec = vec_of(&ion, &els).filter(|_| !els.is_empty());
    let t = TwoLevel {
        ion: vec.as_deref(),
        els: &els,
        mz0: d.parent.mz,
        mzs: &p2.mz,
        rel: &p2.rel,
        smiles: d.parent.smiles.as_deref(),
        ppm: 5.0,
        abs_da: 0.002,
        supposed: false,
        mode: "hr",
        n_scans: Some(p2.n_scans as i64),
    };
    two_level(&t, &super::edge::Iimn::new().losses)
}
