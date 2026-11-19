//! `--sample 1,3,5` / `2-4` of the commands that open a multi-sample file (`convert`, `channels`, `features`, `sample select`): the text is parsed and
//! checked against the ids the file lists by `mzlab_core::readers::selection` (the very code of the site's `sample_selection`). Only the container and the
//! index are read to check it. Exit codes: 2 for a text that is not a list, 5 for an id the file does not have (the ids present are named).

use std::path::Path;

use mzlab_core::error::{Error, Result};
use mzlab_core::readers::{selection, Limits};

use crate::Outcome;

/// The ids of `text`, ascending; `err.sample.list` when it is not a list.
pub(crate) fn parse(text: &str) -> Result<Vec<u32>> {
    selection::parse(text)
}

/// The sample ids of the dataset at `path`, from its container and index alone.
pub(crate) fn present(path: &Path) -> Result<Vec<u32>> {
    let fs = crate::flow_set::file_set(path)?;
    let l = mzlab_lettori::apri_listing(fs, &Limits::desktop())?;
    Ok(l.samples().iter().map(|s| s.id).collect())
}

/// The ids and section bytes (0 when the reader gives none) of the samples of `path`, from the container and index alone.
pub(crate) fn sized(path: &Path) -> Result<(Vec<u32>, Vec<u64>)> {
    let fs = crate::flow_set::file_set(path)?;
    let l = mzlab_lettori::apri_listing(fs, &Limits::desktop())?;
    let s = l.samples();
    Ok((
        s.iter().map(|x| x.id).collect(),
        s.iter().map(|x| x.size.unwrap_or(0)).collect(),
    ))
}

/// `wanted` among the samples of `path`, in the order of the file; `err.sample.unknown` when one is missing.
pub(crate) fn resolve(path: &Path, wanted: &[u32]) -> Result<Vec<u32>> {
    selection::resolve(&present(path)?, wanted)
}

/// The exit code of a selection error: 5 for an unknown id, else 2 (usage).
pub(crate) fn code_of(e: &Error) -> u8 {
    crate::exit::code_of(e)
}

/// The failed `Outcome` of a selection error: the text on stderr, the key for `error/1` and the way out.
pub(crate) fn fail(e: &Error) -> Outcome {
    let o = Outcome::fail_error(e);
    if e.key.starts_with("err.sample.") {
        o.with_hint("list the samples with `mzlab-cli sample list FILE --json`")
    } else {
        o
    }
}

#[cfg(test)]
#[path = "sample_select_tests.rs"]
mod tests;
