//! The members of a series when `--sample LIST` is given (`features`, `quant targets`): a file with several samples gives one member per chosen sample, labelled
//! `COLUMN@ID` (`sample_label`: the sample name, the file only breaks a tie); any other file is one member as it is. One code for every command, so the same
//! names and the same stderr notes (`note.features.column*`). Without `--sample`, the first sample of such a file is read and a note names the way to the others.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};

use crate::sample_label::{labels, Kind, Member};

/// One file of the series: the dataset, the label that picks its sample (`COLUMN@ID`; empty for a file taken as it is) and what the label was made of.
pub(crate) struct Picked {
    pub path: PathBuf,
    pub label: String,
    /// The column / table name of a chosen sample (empty for a file taken as it is).
    pub column: String,
    pub id: Option<u32>,
    pub sample: String,
    pub kind: Kind,
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned())
}

/// A file the listing is not asked about: an mzML, an mzXML or an index holds one sample.
fn plain(p: &Path) -> bool {
    let low = p.to_string_lossy().to_lowercase();
    [".mzml", ".mzml.gz", ".mzxml", ".mzidx"]
        .iter()
        .any(|e| low.ends_with(e))
}

/// The members of `files` for the samples `want`: every sample of a file with several that `want` names (ids as `sample list` gives them, `err.sample.unknown`
/// for one it has not), every other file as it is. `err.sample.single` when no file has several samples, so a typo is never silent.
pub(crate) fn expand(files: &[PathBuf], want: &[u32]) -> Result<Vec<Picked>> {
    let mut out: Vec<Picked> = Vec::new();
    let mut applied = false;
    for f in files {
        let present = if plain(f) {
            Vec::new()
        } else {
            crate::sample_select::named(f)?
        };
        if present.len() < 2 {
            out.push(Picked {
                path: f.clone(),
                label: String::new(),
                column: String::new(),
                id: None,
                sample: String::new(),
                kind: Kind::Own,
            });
            continue;
        }
        applied = true;
        let have: Vec<u32> = present.iter().map(|(i, _)| *i).collect();
        for id in mzlab_core::readers::selection::resolve(&have, want)? {
            out.push(Picked {
                path: f.clone(),
                label: String::new(),
                column: String::new(),
                id: Some(id),
                sample: present
                    .iter()
                    .find(|(i, _)| *i == id)
                    .map(|(_, n)| n.clone())
                    .unwrap_or_default(),
                kind: Kind::Own,
            });
        }
    }
    if !applied {
        return Err(Error::new("err.sample.single"));
    }
    // the labels are made over every chosen sample at once, so a tie across files is broken as the Flow and the site do
    let fnames: Vec<String> = out.iter().map(|m| file_name(&m.path)).collect();
    let chosen: Vec<usize> = (0..out.len()).filter(|k| out[*k].id.is_some()).collect();
    let members: Vec<Member> = chosen
        .iter()
        .map(|k| Member {
            file: &fnames[*k],
            id: out[*k].id.unwrap_or(0),
            sample: &out[*k].sample,
        })
        .collect();
    let columns = labels(&members);
    for (k, (column, kind)) in chosen.into_iter().zip(columns) {
        out[k].label = format!("{column}@{}", out[k].id.unwrap_or(0));
        out[k].column = column;
        out[k].kind = kind;
    }
    Ok(out)
}

/// One `note:` line per chosen sample: its label and how it was made (every automatic choice is on stderr).
pub(crate) fn notes(members: &[Picked]) -> String {
    let mut s = String::new();
    for m in members {
        let Some(id) = m.id else { continue };
        let key = match m.kind {
            Kind::Own => "note.features.column",
            Kind::Tie => "note.features.columnTie",
            Kind::Numbered => "note.features.columnNumbered",
        };
        s += &format!(
            "note: {}\n",
            crate::error_text::warning(&crate::convert::Notice {
                key,
                params: vec![
                    ("column", m.column.clone()),
                    ("id", id.to_string()),
                    ("sample", m.sample.clone()),
                    ("file", file_name(&m.path)),
                ],
            })
        );
    }
    s
}

/// `--sample` not given: a file with several samples is read as its first, and the note names the command that chooses (`key`: `note.features.firstSample`,
/// `note.quant.firstSample`). Empty when there is nothing to say.
pub(crate) fn first_sample_note(f: &Path, key: &'static str) -> String {
    let Some(count) = crate::sample_select::several(f) else {
        return String::new();
    };
    format!(
        "note: {}\n",
        crate::error_text::warning(&crate::convert::Notice {
            key,
            params: vec![
                ("file", f.display().to_string()),
                ("count", count.to_string()),
            ],
        })
    )
}

/// The stderr hint of a command that reads one sample of a file with several, when no sample was asked for: it names the command, how many samples the file has
/// and the way to another one. `choose`: the command takes `--sample ID` (`note.sample.firstChoose`); else it reads the first sample by design and the way
/// is `convert --sample LIST` (`note.sample.firstOnly`). Empty when the file has one sample (or is an mzML, an index, or cannot be listed): never an error.
pub(crate) fn first_hint(f: &Path, command: &str, choose: bool) -> String {
    let Some(count) = crate::sample_select::several(f) else {
        return String::new();
    };
    let key = if choose {
        "note.sample.firstChoose"
    } else {
        "note.sample.firstOnly"
    };
    format!(
        "hint: {}\n",
        crate::error_text::warning(&crate::convert::Notice {
            key,
            params: vec![
                ("command", command.to_string()),
                ("file", f.display().to_string()),
                ("count", count.to_string()),
            ],
        })
    )
}

#[cfg(test)]
#[path = "first_sample_tests.rs"]
mod tests;
