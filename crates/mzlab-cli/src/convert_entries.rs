//! The mzML files `mzlab-cli convert` makes from its inputs: one per dataset, or one per sample of a dataset with several (`NAME-ID.mzML`),
//! the names that would clash keeping their extension; a sample with no scans is marked so that its progress line says it is empty.

use std::path::{Path, PathBuf};

use mzlab_core::lettori::Limiti;

use crate::flow_set::file_set;

/// One mzML to make: the dataset, the name of the plan input (`NAME` or `NAME@ID`) and the file written.
pub(crate) struct Entry {
    pub(crate) input: PathBuf,
    pub(crate) label: String,
    pub(crate) out: String,
    /// a sample with no scans (an injection that acquired nothing): its mzML is written empty and its progress line says so
    pub(crate) empty: bool,
}

/// The ids of the samples of a dataset with more than one (each with whether it has no scans), else none (a dataset the reader cannot open is converted as one and fails there).
fn sample_ids(p: &Path) -> Vec<(u32, bool)> {
    let opened = file_set(p).and_then(|fs| mzlab_lettori::apri(fs, &Limiti::desktop()));
    let ids: Vec<(u32, bool)> = opened.map_or(Vec::new(), |l| {
        l.campioni()
            .iter()
            .map(|s| (s.id, s.scansioni == 0))
            .collect()
    });
    if ids.len() > 1 {
        ids
    } else {
        Vec::new()
    }
}

/// `NAME.mzML` or `NAME-ID.mzML`, NAME without its extension unless `long`.
fn out_name(name: &str, id: Option<u32>, long: bool) -> String {
    let name = crate::convert_report::sans_gz(name);
    let base = if long {
        name
    } else {
        name.rsplit_once('.').map_or(name, |p| p.0)
    };
    id.map_or(format!("{base}.mzML"), |i| format!("{base}-{i}.mzML"))
}

/// The mzML files of the datasets: one per sample of a dataset with several; two that would get the same name keep the extension of their input.
pub(crate) fn entries(units: &[PathBuf]) -> Vec<Entry> {
    let mut all: Vec<(PathBuf, String, Option<u32>)> = Vec::new();
    let mut empty = Vec::new();
    for p in units {
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let ids = sample_ids(p);
        if ids.is_empty() {
            all.push((p.clone(), name.clone(), None));
            empty.push(false);
        }
        for (id, none) in ids {
            all.push((p.clone(), name.clone(), Some(id)));
            empty.push(none);
        }
    }
    let short: Vec<String> = all
        .iter()
        .map(|(_, n, id)| out_name(n, *id, false))
        .collect();
    let clash = |k: usize| short.iter().filter(|s| **s == short[k]).count() > 1;
    let outs: Vec<String> = (0..all.len())
        .map(|k| out_name(&all[k].1, all[k].2, clash(k)))
        .collect();
    let mut made = Vec::new();
    for (k, (input, name, id)) in all.iter().enumerate() {
        let plain = id.map_or(name.clone(), |i| format!("{name}@{i}"));
        let unique = all.iter().filter(|(_, n, i)| (n, i) == (name, id)).count() == 1;
        // two inputs of one name (two folders) keep their position in it, or the second file would replace the first
        let taken = outs.iter().filter(|o| **o == outs[k]).count() > 1;
        let (lead, mark) = (format!("{k}/"), format!("{k}-"));
        made.push(Entry {
            input: input.clone(),
            label: format!("{}{plain}", if unique { "" } else { &lead }),
            out: format!("{}{}", if taken { &mark } else { "" }, outs[k]),
            empty: empty[k],
        });
    }
    made
}

impl Entry {
    /// The name in the progress line: the plan input, and «no scans» for an injection that acquired nothing.
    pub(crate) fn shown(&self) -> String {
        if self.empty {
            format!("{} (no scans: an empty injection)", self.label)
        } else {
            self.label.clone()
        }
    }
}
