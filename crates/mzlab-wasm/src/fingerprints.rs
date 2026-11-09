//! The fingerprints of the members of a dataset for `build_mzidx_native`: given by the page (taken while the members were copied, `Passata`),
//! given late by a function the build calls once the first pass is done (the copy's hash still running, as `with_parallel_prints` in the app),
//! or taken here. A late value that is missing, empty or does not match: the members are read again here, never an index without them.

use js_sys::Function;
use mzlab_core::readers::FileSet;
use mzlab_core::upload::{self, Fingerprint};
use wasm_bindgen::prelude::*;

use crate::error_json;

pub(crate) struct Fingerprints {
    now: Option<Vec<Fingerprint>>,
    later: Option<Function>,
}

impl Fingerprints {
    /// From `prints` (the flat `[sha256, sha1, probe]` of each member, a function, or nothing) for the set `fs` the reader opens; `tell(1)` when
    /// the members are read here now. With a function the set carries an empty SHA-1 (`without_sha1`): the table does not read it.
    pub(crate) fn new(
        fs: &mut FileSet,
        prints: &JsValue,
        tell: &dyn Fn(u32),
    ) -> Result<Fingerprints, JsValue> {
        if let Some(f) = prints.dyn_ref::<Function>() {
            upload::without_sha1(fs);
            return Ok(Fingerprints {
                now: None,
                later: Some(f.clone()),
            });
        }
        let now = match upload::impronte_note(fs, &prints_of(prints)) {
            Some(i) => i,
            None => {
                tell(1);
                upload::impronte(fs).map_err(|e| error_json(&e))?
            }
        };
        Ok(Fingerprints {
            now: Some(now),
            later: None,
        })
    }

    /// The fingerprints when the table is done; `set` makes a fresh set of the members (read here when the late value is not usable).
    pub(crate) fn given(&self, set: impl Fn() -> FileSet) -> mzlab_core::Result<Vec<Fingerprint>> {
        if let Some(i) = &self.now {
            return Ok(i.clone());
        }
        let v = self
            .later
            .as_ref()
            .and_then(|f| f.call0(&JsValue::NULL).ok());
        let mut fs = set();
        match upload::impronte_note(&mut fs, &prints_of(&v.unwrap_or(JsValue::NULL))) {
            Some(i) => Ok(i),
            None => upload::impronte(&mut fs),
        }
    }
}

/// `[sha256, sha1, probe]` of each member from the flat array of the page; nothing when it is not one or a value is empty.
fn prints_of(v: &JsValue) -> Vec<(String, String, String)> {
    let Some(a) = v.dyn_ref::<js_sys::Array>() else {
        return vec![];
    };
    let s: Vec<String> = a
        .iter()
        .map(|v| v.as_string().unwrap_or_default())
        .collect();
    if s.iter().any(String::is_empty) {
        return vec![];
    }
    s.chunks_exact(3)
        .map(|c| (c[0].clone(), c[1].clone(), c[2].clone()))
        .collect()
}
