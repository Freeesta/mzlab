//! Warm-up of the pointed folder (docs/agenti/riscaldamento.md): `lr_warm` builds the index of a file the student has not opened yet, with the same builder as
//! `lr_index`; the file is not in the store, so the list of the page never shows it. `lr_warm_claim` (the page, when the file is opened) puts it in the store
//! and keeps that index if the file is still the same; the page then skips `lr_index`.

use std::path::PathBuf;

use mzlab_core::Result;
use serde_json::{json, Value};

use crate::engine::State;
use crate::rpc::{text, Answer};
use crate::store::{clean, stamp};

impl State {
    /// The files of the store the page lists: the ones built ahead are not there until they are opened.
    pub(crate) fn visible_list(&self) -> Vec<(String, crate::store::Entry, bool)> {
        let warm = self.warm.lock().unwrap_or_else(|e| e.into_inner());
        let list = self.store.lock().unwrap_or_else(|e| e.into_inner()).list();
        list.into_iter()
            .filter(|(n, _, _)| !warm.contains_key(n))
            .collect()
    }

    /// Forgets the index built ahead of `name` (every one when None) and removes it from the folder of the indexes.
    pub(crate) fn warm_forget(&self, name: Option<&str>) {
        let gone: Vec<String> = {
            let mut w = self.warm.lock().unwrap_or_else(|e| e.into_inner());
            match name {
                Some(n) => w.remove(n).map(|_| n.to_string()).into_iter().collect(),
                None => w.drain().map(|(n, _)| n).collect(),
            }
        };
        if let Ok(b) = self.indexer() {
            for n in gone {
                b.remove(&n);
            }
        }
    }
}

impl State {
    /// `lr_warm`: the index of the mzML `fid` under the name `name`. `{"skipped":true}` when one built from this very file is already there.
    pub(crate) fn lr_warm(&self, p: &Value) -> Result<Answer> {
        let name = clean(p["name"].as_str().unwrap_or(""))?;
        let pk = self.picked(p["fid"].as_u64().unwrap_or(0) as u32)?;
        let (size, mtime) = stamp(&pk.path)
            .ok_or_else(|| mzlab_core::Error::new("err.file.unknown").with("detail", "gone"))?;
        let now = Warm {
            path: pk.path.clone(),
            size,
            mtime,
        };
        let b = self.indexer()?;
        if self
            .warm
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            == Some(&now)
            && b.size(name).is_some()
        {
            return text("{\"skipped\":true}");
        }
        if self
            .store
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .is_some()
        {
            return text("{\"skipped\":true}"); // the student has it open already
        }
        let t0 = std::time::Instant::now();
        b.build(&pk.path, name, None)?;
        self.warm
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(name.to_string(), now);
        text(json!({ "ms": t0.elapsed().as_millis() as u64, "size": size }).to_string())
    }

    /// `lr_warm_claim`: `{"hit":true}` when the index built ahead belongs to this file as it is now; the file is then in the store with that index.
    pub(crate) fn lr_warm_claim(&self, p: &Value) -> Result<Answer> {
        let name = clean(p["name"].as_str().unwrap_or(""))?;
        let Some(w) = self
            .warm
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(name)
        else {
            return text("{\"hit\":false}");
        };
        let pk = self.picked(p["fid"].as_u64().unwrap_or(0) as u32)?;
        let b = self.indexer()?;
        if pk.path != w.path || stamp(&w.path) != Some((w.size, w.mtime)) || b.size(name).is_none()
        {
            b.remove(name);
            return text("{\"hit\":false}");
        }
        let e = self
            .store
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .put(name, w.path)?;
        text(json!({ "hit": true, "size": e.size, "id": self.pick_path(&e.path) }).to_string())
    }
}

/// What an index built ahead was built from.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Warm {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: u64,
}
