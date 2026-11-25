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
        l.unwrap_or_else(|| self.column_of(&a.label.clone().unwrap_or_default()))
    }

    /// Fixes the column of every `NAME@ID` input of `plan` that is a sample of a file with several (`sample_label`, over all of them at once so that a tie is
    /// broken as in `features --sample`). Called once, before the first job; any other input keeps its own name.
    pub(crate) fn name_columns(&self, plan: &mzlab_core::flow::Plan) -> &Self {
        let mut rows: Vec<(String, String, u32, String)> = Vec::new();
        for f in &plan.inputs {
            let Some(id) = crate::flow_set::split_sample(&f.name).1 else {
                continue;
            };
            let path = self
                .inputs
                .get(&f.name)
                .cloned()
                .unwrap_or_else(|| self.dir.join(crate::flow_set::dataset_name(&f.name)));
            let Ok(all) = crate::sample_select::named(&path) else {
                continue;
            };
            if all.len() < 2 {
                continue;
            }
            let Some((_, sample)) = all.iter().find(|(i, _)| *i == id) else {
                continue;
            };
            let file = path
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned());
            rows.push((f.name.clone(), file, id, sample.clone()));
        }
        let members: Vec<crate::sample_label::Member> = rows
            .iter()
            .map(|(_, f, id, s)| crate::sample_label::Member {
                file: f,
                id: *id,
                sample: s,
            })
            .collect();
        let columns = crate::sample_label::labels(&members);
        let map = rows
            .iter()
            .zip(columns)
            .map(|((k, ..), (c, _))| (k.clone(), c))
            .collect();
        let _ = self.columns.set(map);
        self
    }

    fn column_of(&self, label: &str) -> String {
        self.columns
            .get()
            .and_then(|m| m.get(label))
            .cloned()
            .unwrap_or_else(|| label.to_string())
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
