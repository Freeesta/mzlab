//! The MSn part of the Dig of mzFinder (`mzlab_feat::legacy::msn`): the tree of the direct infusion of the standard, its calibration, the library of the
//! fragments and the localisation of a product on the structure of the parent. Built only into the `.wasm` of the feature `finder_msn`, loaded by the
//! worker of mzFinder only when a file of the series is of kind `msn`.

use std::collections::HashMap;

use js_sys::Function;
use mzlab_core::idxread::Index;
use mzlab_feat::legacy::fragtree::{elements_of, vec_of};
use mzlab_feat::legacy::msn::{self, json, library, localize, MsnTree};
use mzlab_feat::legacy::series;
use wasm_bindgen::prelude::*;

use crate::JsReader;

fn user(e: String) -> JsValue {
    js_sys::Error::new(&e).into()
}

#[wasm_bindgen]
pub struct FinderMsn {
    tree: MsnTree,
    smiles: Option<String>,
    /// The ion of the parent over the elements of the tree: the reference of the localisation.
    reference: Option<Vec<i64>>,
    lib: Option<library::Library>,
    locs: HashMap<u32, localize::Localization>,
}

/// The tree of the MSn file (`.mzidx` of `size` bytes read with `read(0, offset, length)`) for the parent of `parent_json`, with its library.
#[wasm_bindgen]
pub fn finder_msn(read: Function, size: f64, parent_json: &str) -> Result<FinderMsn, JsValue> {
    if size.is_nan() || size < 0.0 {
        return Err(user("no index of the file: open it again".into()));
    }
    let r = JsReader { read, k: 0 };
    let mut ix = Index::open(r, size as u64).map_err(|e| user(e.to_string()))?;
    let scans = msn::scans_of(&mut ix).map_err(user)?;
    let p = series::parent(parent_json).map_err(user)?;
    let ion = p.ion_formula();
    let tree = msn::build_tree(&scans, &ion, p.smiles.as_deref(), "auto").map_err(user)?;
    let reference = vec_of(&ion, &tree.els);
    let lib = tree.subs.as_ref().map(|_| library::build(&tree, None));
    Ok(FinderMsn {
        tree,
        smiles: p.smiles,
        reference,
        lib,
        locs: HashMap::new(),
    })
}

#[wasm_bindgen]
impl FinderMsn {
    /// `summary.tree` (the eight strongest peaks of every node) and `api.msn_tree`.
    pub fn msn_tree(&self) -> String {
        json::tree_summary(&self.tree)
    }
    /// `summary.calibration`.
    pub fn calibration(&self) -> String {
        json::calibration(&self.tree.cal)
    }
    /// The library again with the peaks of the MS2 of the parent in the series (`build_library(tree, dda_peaks)`).
    pub fn set_parent_ms2(&mut self, mz: Vec<f64>, rel: Vec<f64>) {
        if self.lib.is_some() {
            self.lib = Some(library::build(&self.tree, Some((&mz, &rel))));
        }
    }
    /// `detail.localization` of candidate `id` (neutral formula `formula`, ion at `tp_mz`) from its MS2 (`mz`, `rel`); "null" without a structure,
    /// without a library or with an element the parent has not.
    pub fn localize(
        &mut self,
        id: u32,
        formula: &str,
        mz: Vec<f64>,
        rel: Vec<f64>,
        tp_mz: f64,
    ) -> String {
        let (Some(lib), Some(subs), Some(reference)) =
            (&self.lib, &self.tree.subs, &self.reference)
        else {
            return "null".into();
        };
        let els = &self.tree.els;
        let known = elements_of(formula).is_some_and(|e| e.iter().all(|x| els.contains(x)));
        let Some(mut tp) = vec_of(formula, els).filter(|_| known) else {
            return "null".into();
        };
        let Some(h) = els.iter().position(|e| *e == "H") else {
            return "null".into();
        };
        tp[h] += 1;
        let tp_mz = Some(tp_mz).filter(|v| v.is_finite());
        let loc = localize::localize(&tp, reference, els, &mz, &rel, lib, subs, tp_mz);
        let out = json::localization(&loc);
        self.locs.insert(id, loc);
        out
    }
    /// `api.frag_tree` on the MSn tree (`fragtree._hr_msn`).
    pub fn frag_tree(&self) -> String {
        json::frag_tree(&self.tree, self.smiles.as_deref())
    }
    /// `api.frag_tree_compare(id)` from the localisation of candidate `id` (`fragtree.compare_hr`).
    pub fn frag_tree_compare(&self, id: u32) -> String {
        json::compare(&self.tree, self.locs.get(&id))
    }
}
