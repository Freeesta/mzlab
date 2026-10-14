//! `api.frag_tree` and `api.frag_tree_compare` of a low-resolution Dig (`fragtree.lr`, `fragtree.compare_lr`): the supposed tree of the parent from the
//! MS2 scans of the `ms2` files, and the nodes of it found in the fragments of a product. Without the `chem` feature, or without MS2 of the parent,
//! the answer is the note "no MS2". `open(k)` gives the index of file `k` of the columns then the MS2 files.

use mzlab_core::idxread::{Index, ReadAt};

use super::{ms2, Entry, LrDig};
use crate::legacy::Res;

const NO_PARENT_MS2: &str =
    r#"{"ok":false,"mode":"lr","note":"no MS2 of the parent in the files","nodes":[]}"#;

fn find(d: &LrDig, id: i64) -> Res<&Entry> {
    d.entries
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| "candidate not found".to_string())
}

/// `api.frag_tree`: the supposed tree of the parent.
#[cfg(feature = "chem")]
pub fn frag_tree<R: ReadAt>(
    d: &LrDig,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<String> {
    use crate::legacy::lrisf::tree::{lr_tree, ParentMs2};
    let Some(par) = d.entries.first() else {
        return Ok(NO_PARENT_MS2.into());
    };
    let pool = ms2::pool(d, par, ms2::TOP_TREE, open)?;
    let mz: Vec<f64> = pool.frags.iter().map(|f| f.mz).collect();
    let rel: Vec<f64> = pool.frags.iter().map(|f| f.rel).collect();
    let m = ParentMs2 {
        mz: &mz,
        rel: &rel,
        scans: pool.scans,
        precursor: pool.precursor,
    };
    Ok(lr_tree(
        Some(&m),
        par.mz_x,
        d.parent.ion_formula.as_deref(),
        d.parent.smiles.as_deref(),
    ))
}

#[cfg(not(feature = "chem"))]
pub fn frag_tree<R: ReadAt>(
    _d: &LrDig,
    _open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<String> {
    Ok(NO_PARENT_MS2.into())
}

/// `api.frag_tree_compare(id)`: the nodes of the tree of the parent found in the MS2 of the product `id`.
#[cfg(feature = "chem")]
pub fn frag_tree_compare<R: ReadAt>(
    d: &LrDig,
    id: i64,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<String> {
    use crate::legacy::lrisf::compare::compare_lr;
    let e = find(d, id)?;
    let tree =
        mzlab_core::json::parse(frag_tree(d, open)?.as_bytes()).map_err(|x| x.to_string())?;
    let tp = ms2::ms2(d, e, open)?;
    let mz: Vec<f64> = tp.frags.iter().map(|f| f.mz).collect();
    let dm = e
        .neutral_mass
        .zip(d.entries.first().and_then(|p| p.neutral_mass))
        .map(|(a, b)| a - b);
    Ok(compare_lr(&tree, &mz, dm))
}

#[cfg(not(feature = "chem"))]
pub fn frag_tree_compare<R: ReadAt>(
    d: &LrDig,
    id: i64,
    _open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<String> {
    find(d, id)?;
    Ok(r#"{"ok":false,"note":"no MS2 of the product","marks":{}}"#.into())
}
