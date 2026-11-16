//! The mzML files `mzlab-cli convert` makes from its inputs: one per dataset, or one per sample of a dataset with several (`NAME-ID.mzML`),
//! the names that would clash keeping their extension; a sample with no scans is marked so that its progress line says it is empty.

use std::path::{Path, PathBuf};

use mzlab_core::readers::{Format, Limits};

use crate::flow_run::fail;
use crate::flow_set::file_set;
use crate::Outcome;
use mzlab_core::json::Out;

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

/// With `--samples`, the samples of a SCIEX pair listed without building any (a `.wiff` of many injections would read each one's blocks), and whether the chosen ones are empty;
/// `None` for another dataset or with no choice.
fn listed_only(p: &Path, only: &[u32]) -> Option<Vec<(u32, bool, bool)>> {
    let open =
        |c: usize| file_set(p).and_then(|fs| mzlab_lettori::apri_solo(fs, &Limits::desktop(), c));
    let l = open(usize::MAX)
        .ok()
        .filter(|l| !only.is_empty() && l.format() == Format::SciexWiff)?;
    let ids: Vec<u32> = l.samples().iter().map(|s| s.id).collect();
    if ids.len() < 2 {
        return None;
    }
    Some(
        ids.iter()
            .enumerate()
            .map(|(c, &id)| {
                let empty = only.contains(&id)
                    && open(c).is_ok_and(|mut r| {
                        r.samples().get(c).is_some_and(|s| s.scansioni == 0)
                            && r.chromatograms(c).is_ok_and(|v| v.is_empty())
                            && r.losses(c).is_ok_and(|x| !x.partial())
                    });
                (id, empty, true)
            })
            .collect(),
    )
}

/// The ids of the samples of a dataset with more than one (each with whether it has no scans), else none (a dataset the reader cannot open is converted as one and fails there).
fn sample_ids(p: &Path, only: &[u32]) -> Vec<(u32, bool, bool)> {
    if let Some(v) = listed_only(p, only) {
        return v;
    }
    let opened = file_set(p).and_then(|fs| mzlab_lettori::apri(fs, &Limits::desktop()));
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
pub(crate) fn entries(units: &[PathBuf], only: &[u32]) -> Vec<Entry> {
    let mut all: Vec<(PathBuf, String, Option<u32>)> = Vec::new();
    let mut empty = Vec::new();
    let mut wiff = Vec::new();
    for p in units {
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let ids = sample_ids(p, only);
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
    made
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

/// `--samples`: keeps the entries of the sample ids asked for; an id no input holds is the exit 4. A dataset of one sample (its entry names no id) is the sample 1.
pub(crate) fn chosen(list: &mut Vec<Entry>, ids: &[u32]) -> Result<(), Outcome> {
    if ids.is_empty() {
        return Ok(());
    }
    let held = |e: &Entry| crate::flow_set::split_sample(&e.label).1.or(Some(1));
    if let Some(id) = ids
        .iter()
        .find(|id| !list.iter().any(|e| held(e) == Some(**id)))
    {
        return Err(fail(
            4,
            &format!("--samples: no input holds the sample {id}"),
            "the samples of a dataset are listed with mzlab-cli samples or in the .wiff list of the page",
        ));
    }
    list.retain(|e| held(e).is_some_and(|id| ids.contains(&id)));
    Ok(())
}

/// The plan of the one step: every entry an input of the port `raw`.
pub(crate) fn plan_text(entries: &[Entry], version: u32) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":\"mzflow/2\",\"inputs\":[");
    for (i, e) in entries.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(&e.label)
            .raw(",\"type\":\"raw\"}");
    }
    o.raw("],\"steps\":[{\"id\":\"conv\",\"step\":")
        .str(&format!("convert.mzml@{version}"));
    o.raw(",\"in\":{\"raw\":[");
    for (i, e) in entries.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .str(&format!("file:{}", e.label));
    }
    o.raw("]}}]}");
    o.0
}

/// `--samples 3,7`: the sample ids as numbers.
pub(crate) fn samples_arg(v: &str) -> Result<Vec<u32>, String> {
    v.split(',')
        .map(|id| {
            id.trim().parse().map_err(|_| {
                format!("--samples {v}: expected whole sample numbers separated by commas")
            })
        })
        .collect()
}
