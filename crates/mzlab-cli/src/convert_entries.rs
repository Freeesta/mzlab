//! The mzML files `mzlab-cli file convert` makes from its inputs: one per dataset, or one per sample of a dataset with several (`NAME-ID.mzML`),
//! the names that would clash keeping their extension; a sample with no scans is marked so that its progress line says it is empty.
//! With `--name sample` the file is `SAMPLE.mzML`, SAMPLE being the sample name cleaned to a safe file name; names that clash get `-ID` (or a number), logged.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::readers::{selection, Format, Limits};
use mzlab_core::samples::{mzml_names, NameItem};

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
    /// the line that says why this file got a name other than the plain sample name (an empty name, a clash): logged by the caller
    pub(crate) note: Option<String>,
}

/// The samples of a dataset with more than one: id, whether it has no scans, whether the dataset is a `.wiff`, and the sample name.
type Row = (u32, bool, bool, String);

/// The rows of the samples of a dataset with more than one, else none (a dataset the reader cannot open is converted as one and fails there); the second
/// value is the name of the only sample of a dataset that has one (what `--name sample` writes).
/// With `sel`, only those samples are looked at (the others are never read) and one the dataset does not have is `err.sample.unknown`.
fn sample_ids(p: &Path, sel: Option<&[u32]>) -> Result<(Vec<Row>, Option<String>)> {
    let opened = file_set(p).and_then(|fs| mzlab_lettori::open(fs, &Limits::desktop()));
    let found: Vec<u32> = opened
        .as_ref()
        .map_or(Vec::new(), |l| l.samples().iter().map(|s| s.id).collect());
    // a selection names ids of a dataset with several samples; a dataset with one is converted as it is
    let keep = match sel.filter(|_| found.len() > 1) {
        Some(w) => Some(selection::resolve(&found, w)?),
        None => None,
    };
    let only = opened
        .as_ref()
        .ok()
        .filter(|l| l.samples().len() == 1)
        .map(|l| l.samples()[0].name.clone());
    let ids: Vec<Row> = opened.map_or(Vec::new(), |mut l| {
        // one calm line per dataset, never per sample
        let (found, skipped) = l.unindexed();
        if let Some(n) = crate::convert::unindexed_sections(found, skipped) {
            eprintln!("note: {} ({})", crate::error_text::warning(&n), p.display());
        }
        let wiff = l.format() == Format::SciexWiff;
        let ids: Vec<(u32, u32, usize, String)> = l
            .samples()
            .iter()
            .enumerate()
            .map(|(c, s)| (s.id, c as u32, s.scansioni, s.name.clone()))
            .filter(|(id, _, _, _)| keep.as_ref().is_none_or(|k| k.contains(id)))
            .collect();
        // empty = no scans and no chromatograms (an MRM sample has chromatograms only), and nothing lost: a damaged sample is not an empty acquisition
        ids.into_iter()
            .map(|(id, c, scans, name)| {
                let c = c as usize;
                let empty = scans == 0
                    && l.chromatograms(c).is_ok_and(|v| v.is_empty())
                    && l.losses(c).is_ok_and(|x| !x.partial());
                (id, empty, wiff, name)
            })
            .collect()
    });
    // one sample left by a selection still gets its `NAME-ID.mzML`: the bytes are those of the same sample of an unrestricted run
    let ids = if ids.len() > 1 || keep.is_some() {
        ids
    } else {
        Vec::new()
    };
    Ok((ids, only))
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
/// With `by_sample` (`--name sample`) each file is `SAMPLE.mzML`: the cleaned sample name; two equal names (case aside, as on Windows and macOS) become
/// `SAMPLE-ID.mzML` (a dataset with one sample: `SAMPLE-N.mzML`, N the order of the clash), a name that cleans to nothing keeps the default name, each with its `note`.
pub(crate) fn entries(
    units: &[PathBuf],
    sel: Option<&[u32]>,
    by_sample: bool,
) -> Result<Vec<Entry>> {
    // input, file name, sample id, sample name
    let mut all: Vec<(PathBuf, String, Option<u32>, Option<String>)> = Vec::new();
    let mut empty = Vec::new();
    let mut wiff = Vec::new();
    for p in units {
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let (ids, only) = sample_ids(p, sel)?;
        if ids.is_empty() {
            all.push((p.clone(), name.clone(), None, only));
            empty.push(false);
            wiff.push(false);
        }
        for (id, none, is_wiff, sample) in ids {
            all.push((p.clone(), name.clone(), Some(id), Some(sample)));
            empty.push(none);
            wiff.push(is_wiff);
        }
    }
    if sel.is_some() && all.iter().all(|(_, _, id, _)| id.is_none()) {
        return Err(Error::new("err.sample.single"));
    }
    let short: Vec<String> = all
        .iter()
        .map(|(_, n, id, _)| out_name(n, *id, false))
        .collect();
    let clash = |k: usize| short.iter().filter(|s| **s == short[k]).count() > 1;
    let mut outs: Vec<String> = (0..all.len())
        .map(|k| out_name(&all[k].1, all[k].2, clash(k)))
        .collect();
    let mut notes: Vec<Option<String>> = vec![None; all.len()];
    if by_sample {
        rename_by_sample(&all, &mut outs, &mut notes);
    }
    let mut made = Vec::new();
    for (k, (input, name, id, _)) in all.iter().enumerate() {
        let plain = id.map_or(name.clone(), |i| format!("{name}@{i}"));
        let unique = all
            .iter()
            .filter(|(_, n, i, _)| (n, i) == (name, id))
            .count()
            == 1;
        // two inputs of one name (two folders) keep their position in it, or the second file would replace the first
        let taken = outs
            .iter()
            .filter(|o| o.to_lowercase() == outs[k].to_lowercase())
            .count()
            > 1;
        let (lead, mark) = (format!("{k}/"), format!("{k}-"));
        made.push(Entry {
            input: input.clone(),
            label: format!("{}{plain}", if unique { "" } else { &lead }),
            out: format!("{}{}", if taken { &mark } else { "" }, outs[k]),
            empty: empty[k],
            wiff: wiff[k],
            note: notes[k].take().or_else(|| {
                taken.then(|| {
                    format!(
                        "note: {plain}: another input has the same file name: written as {}{}",
                        mark, outs[k]
                    )
                })
            }),
        });
    }
    Ok(made)
}

/// The notes of the automatic renames (a clash with either naming, an unsafe sample name with `--name sample`), one line each.
pub(crate) fn notes(list: &[Entry]) -> String {
    let n = list.iter().filter_map(|e| e.note.as_deref());
    n.map(|n| format!("{n}\n")).collect()
}

/// The outcome with the naming notes first on its stderr.
pub(crate) fn lead(naming: &str, mut o: crate::Outcome) -> crate::Outcome {
    o.stderr = format!("{naming}{}", o.stderr);
    o
}

/// `--name sample`: the names of `mzlab_core::samples::mzml_names` (the one function of the CLI, the app and the site) in `outs`, each with its note.
fn rename_by_sample(
    all: &[(PathBuf, String, Option<u32>, Option<String>)],
    outs: &mut [String],
    notes: &mut [Option<String>],
) {
    let who: Vec<String> = all
        .iter()
        .map(|(_, n, id, _)| id.map_or(n.clone(), |i| format!("{n}@{i}")))
        .collect();
    let items: Vec<NameItem> = (0..all.len())
        .map(|k| NameItem {
            who: &who[k],
            plain: &outs[k],
            sample: all[k].3.as_deref(),
            id: all[k].2,
        })
        .collect();
    let named = mzml_names(&items);
    for (k, n) in named.into_iter().enumerate() {
        outs[k] = n.out;
        notes[k] = n.note;
    }
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
