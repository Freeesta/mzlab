//! The lineage a Flow run keeps for the files it makes: by SHA-256 (what a feature step hands on) and by place, the SHA-256 each
//! (instance, item) made, so two items with the same bytes keep their own place in a series.

use std::collections::HashMap;
use std::path::PathBuf;

use mzlab_core::flow::Artifact;

use super::{FileRunner, Lineage};

/// By SHA-256 and by (instance, item): same bytes, not same place.
#[derive(Default)]
pub(crate) struct Traced {
    by_sha: HashMap<String, Lineage>,
    by_place: HashMap<(String, usize), String>,
}

impl FileRunner {
    pub(crate) fn trace(&self, a: &Artifact, l: Lineage) {
        let mut map = self.lineage.lock().unwrap_or_else(|e| e.into_inner());
        map.by_place
            .extend(l.item.map(|k| ((l.instance.clone(), k), a.sha256.clone())));
        map.by_sha.insert(a.sha256.clone(), l);
    }

    pub(crate) fn lineage(&self, a: &Artifact) -> Option<Lineage> {
        let map = self.lineage.lock().unwrap_or_else(|e| e.into_inner());
        map.by_sha.get(&a.sha256).cloned()
    }

    /// The name of the file an index was made from, else the file name of `a`.
    pub(crate) fn source_name(&self, a: &Artifact) -> String {
        let l = self.lineage(a).map(|l| l.name).filter(|n| !n.is_empty());
        l.unwrap_or_else(|| a.label.clone().unwrap_or_default())
    }

    /// The first index (item 0) made by the instance that made the index `a`.
    pub(crate) fn first_of_series(&self, a: &Artifact) -> Option<PathBuf> {
        let me = self.lineage(a)?;
        let map = self.lineage.lock().unwrap_or_else(|e| e.into_inner());
        let first = map.by_place.get(&(me.instance, 0))?;
        let made = self.made.lock().unwrap_or_else(|e| e.into_inner());
        made.get(first).cloned()
    }
}
