//! The MSn part of the Dig of mzFinder in the app: the tree of the direct infusion of the standard, its calibration, the library of the fragments and the localisation of
//! the products on the structure of the parent. The same functions as the `finder_msn` `.wasm` (`mzlab_feat_wasm::msn::FinderMsn`, `mzlab_feat::legacy::msn`) and the same
//! adapter that `tpmine-worker.js` writes around it (the `tree` and `calibration` of the summary, `region` and `margin` of the rows, `localization` of the detail), here in
//! Rust so that the page computes nothing. The session stays in `FeatState` for `finder/detail`, `msn_tree`, `frag_tree` and `frag_tree_compare`.

use std::collections::HashMap;

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_feat::legacy::fragtree::{elements_of, vec_of};
use mzlab_feat::legacy::msn::{self, json, library, localize, MsnTree};
use mzlab_feat::legacy::series;
use serde_json::Value;

pub(crate) struct Msn {
    tree: MsnTree,
    smiles: Option<String>,
    /// The ion of the parent over the elements of the tree: the reference of the localisation.
    reference: Option<Vec<i64>>,
    lib: Option<library::Library>,
    locs: HashMap<u32, localize::Localization>,
    /// The library has been rebuilt with the first MS2 of the parent in the series.
    parent_set: bool,
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

impl Msn {
    /// The tree of the MSn file `ix` for the parent of `parent_json`, with its library.
    pub(crate) fn new<R: ReadAt>(ix: &mut Index<R>, parent_json: &str) -> Result<Msn, String> {
        let scans = msn::scans_of(ix)?;
        let p = series::parent(parent_json)?;
        let ion = p.ion_formula();
        let tree = msn::build_tree(&scans, &ion, p.smiles.as_deref(), "auto")?;
        let reference = vec_of(&ion, &tree.els);
        let lib = tree.subs.as_ref().map(|_| library::build(&tree, None));
        Ok(Msn {
            tree,
            smiles: p.smiles,
            reference,
            lib,
            locs: HashMap::new(),
            parent_set: false,
        })
    }

    pub(crate) fn msn_tree(&self) -> String {
        json::tree_summary(&self.tree)
    }

    pub(crate) fn frag_tree(&self) -> String {
        json::frag_tree(&self.tree, self.smiles.as_deref())
    }

    pub(crate) fn frag_tree_compare(&self, id: u32) -> String {
        json::compare(&self.tree, self.locs.get(&id))
    }

    /// `FinderMsn::localize`: "null" without a structure, a library or with an element the parent has not.
    fn localize_raw(
        &mut self,
        id: u32,
        formula: &str,
        mz: &[f64],
        rel: &[f64],
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
        let loc = localize::localize(&tp, reference, els, mz, rel, lib, subs, tp_mz);
        let out = json::localization(&loc);
        self.locs.insert(id, loc);
        out
    }
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect())
        .unwrap_or_default()
}

impl Msn {
    /// The localisation of a candidate from its detail (kept for `frag_tree_compare`); the first MS2 of the parent rebuilds the library.
    fn localize(&mut self, d: &Value) -> Option<Value> {
        let formula = d["row"]["formula"].as_str().filter(|f| !f.is_empty())?;
        if !truthy(&d["ms2"]) {
            return None;
        }
        if !self.parent_set {
            if self.lib.is_some() {
                let (mz, rel) = (
                    floats(&d["ms2"]["parent"]["mz"]),
                    floats(&d["ms2"]["parent"]["rel"]),
                );
                self.lib = Some(library::build(&self.tree, Some((&mz, &rel))));
            }
            self.parent_set = true;
        }
        let c = &d["ms2"]["candidate"];
        let tp_mz = d["row"]["mz"].as_f64().unwrap_or(f64::NAN);
        let id = d["id"].as_u64().unwrap_or(0) as u32;
        let raw = self.localize_raw(id, formula, &floats(&c["mz"]), &floats(&c["rel"]), tp_mz);
        serde_json::from_str::<Value>(&raw)
            .ok()
            .filter(|v| !v.is_null())
    }

    /// `api.detail` with the localisation of the MSn tree (`detail` is the JSON of the Dig).
    pub(crate) fn detail(&mut self, raw: &str) -> String {
        let Ok(mut d) = serde_json::from_str::<Value>(raw) else {
            return raw.to_string();
        };
        if let Some(loc) = self.localize(&d) {
            d["ms2"]["evidence"] = loc.get("evidence").cloned().unwrap_or(Value::Array(vec![]));
            d["localization"] = loc;
        }
        d.to_string()
    }

    /// The summary of the Dig with the MSn tree: tree, calibration, region and margin of the rows (`detail(id)` is the detail of the Dig).
    pub(crate) fn summary(
        &mut self,
        s: &str,
        detail: impl Fn(i64) -> Result<String, String>,
    ) -> Result<String, String> {
        let mut v: Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
        v["tree"] = serde_json::from_str(&self.msn_tree()).map_err(|e| e.to_string())?;
        v["calibration"] =
            serde_json::from_str(&json::calibration(&self.tree.cal)).map_err(|e| e.to_string())?;
        let ids: Vec<(usize, i64)> = v["rows"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .enumerate()
                    .filter(|(_, r)| truthy(&r["ms2_scans"]) && truthy(&r["formula"]))
                    .map(|(i, r)| (i, r["id"].as_i64().unwrap_or(0)))
                    .collect()
            })
            .unwrap_or_default();
        for (i, id) in ids {
            let d: Value = serde_json::from_str(&detail(id)?).map_err(|e| e.to_string())?;
            let loc = self
                .localize(&d)
                .filter(|l| l["ok"].as_bool() == Some(true));
            v["rows"][i]["region"] = loc.as_ref().map_or(Value::Null, |l| l["region"].clone());
            v["rows"][i]["margin"] = loc.as_ref().map_or(Value::Null, |l| {
                let m = (l["margin"].as_f64().unwrap_or(0.0) * 100.0).round() / 100.0;
                serde_json::Number::from_f64(m).map_or(Value::Null, Value::Number)
            });
        }
        Ok(v.to_string())
    }
}
