//! The mzML files `mzlab-cli file convert` makes from its inputs: one per dataset, or one per sample of a dataset with several (`NAME-ID.mzML`),
//! the names that would clash keeping their extension; a sample with no scans is marked so that its progress line says it is empty.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::readers::{selection, Format, Limits};

use crate::flow_set::file_set;

/// One mzML to make: the dataset, the name of the plan input (`NAME` or `NAME@ID`) and the file written.
pub(crate) struct Entry {
    pub(crate) input: PathBuf,
    pub(crate) label: String,
    pub(crate) out: String,
    /// a sample with no scans and no chromatograms (an injection that acquired nothing): its mzML is written empty and its progress line says so
    pub(crate) empty: bool,
    /// the dataset is a SCIEX `.wiff`: the progress line then says why the sample is empty (`warn.wiff.emptySample`)
    pub(crate) wiff: bool,
}

/// The ids of the samples of a dataset with more than one (each with whether it has no scans), else none (a dataset the reader cannot open is converted as one and fails there).
/// With `sel`, only those samples are looked at (the others are never read) and one the dataset does not have is `err.sample.unknown`.
fn sample_ids(p: &Path, sel: Option<&[u32]>) -> Result<Vec<(u32, bool, bool)>> {
    let opened = file_set(p).and_then(|fs| mzlab_lettori::apri(fs, &Limits::desktop()));
    let found: Vec<u32> = opened
        .as_ref()
        .map_or(Vec::new(), |l| l.samples().iter().map(|s| s.id).collect());
    // a selection names ids of a dataset with several samples; a dataset with one is converted as it is
    let keep = match sel.filter(|_| found.len() > 1) {
        Some(w) => Some(selection::resolve(&found, w)?),
        None => None,
    };
    let ids: Vec<(u32, bool, bool)> = opened.map_or(Vec::new(), |mut l| {
        // one calm line per dataset, never per sample
        let (found, skipped) = l.unindexed();
        if let Some(n) = crate::convert::unindexed_sections(found, skipped) {
            eprintln!("note: {} ({})", crate::error_text::warning(&n), p.display());
        }
        let wiff = l.format() == Format::SciexWiff;
        let ids: Vec<(u32, u32, usize)> = l
            .samples()
            .iter()
            .enumerate()
            .map(|(c, s)| (s.id, c as u32, s.scansioni))
            .filter(|(id, _, _)| keep.as_ref().is_none_or(|k| k.contains(id)))
            .collect();
        // empty = no scans and no chromatograms (an MRM sample has chromatograms only), and nothing lost: a damaged sample is not an empty acquisition
        ids.into_iter()
            .map(|(id, c, scans)| {
                let c = c as usize;
                let empty = scans == 0
                    && l.chromatograms(c).is_ok_and(|v| v.is_empty())
                    && l.losses(c).is_ok_and(|x| !x.partial());
                (id, empty, wiff)
            })
            .collect()
    });
    // one sample left by a selection still gets its `NAME-ID.mzML`: the bytes are those of the same sample of an unrestricted run
    Ok(if ids.len() > 1 || keep.is_some() {
        ids
    } else {
        Vec::new()
    })
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
pub(crate) fn entries(units: &[PathBuf], sel: Option<&[u32]>) -> Result<Vec<Entry>> {
    let mut all: Vec<(PathBuf, String, Option<u32>)> = Vec::new();
    let mut empty = Vec::new();
    let mut wiff = Vec::new();
    for p in units {
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let ids = sample_ids(p, sel)?;
        if ids.is_empty() {
            all.push((p.clone(), name.clone(), None));
            empty.push(false);
            wiff.push(false);
        }
        for (id, none, is_wiff) in ids {
            all.push((p.clone(), name.clone(), Some(id)));
            empty.push(none);
            wiff.push(is_wiff);
        }
    }
    if sel.is_some() && all.iter().all(|(_, _, id)| id.is_none()) {
        return Err(Error::new("err.sample.single"));
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
            wiff: wiff[k],
        });
    }
    Ok(made)
}

impl Entry {
    /// The name in the progress line: the plan input, and «no scans» for an injection that acquired nothing.
    pub(crate) fn shown(&self) -> String {
        if self.empty {
            let why = if self.wiff {
                crate::error_text::catalog_text("warn.wiff.emptySample")
            } else {
                None
            };
            format!(
                "{} (no scans: {})",
                self.label,
                why.unwrap_or("an empty injection")
            )
        } else {
            self.label.clone()
        }
    }
}
