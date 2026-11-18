//! `--sample 1,3,5` / `2-4` of the commands that open a multi-sample file (`convert`, `channels`, `features`, `samples select`): the text is parsed and
//! checked against the ids the file lists by `mzlab_core::readers::selection` (the very code of the site's `sample_selection`). Only the container and the
//! index are read to check it. Exit codes: 2 for a text that is not a list, 5 for an id the file does not have (the ids present are named).

use std::path::Path;

use mzlab_core::error::{Error, Result};
use mzlab_core::readers::{selection, Limits};

use crate::Outcome;

/// The exit code of an id the file does not have.
pub(crate) const UNKNOWN: u8 = 5;

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

/// `wanted` among the samples of `path`, in the order of the file; `err.sample.unknown` when one is missing.
pub(crate) fn resolve(path: &Path, wanted: &[u32]) -> Result<Vec<u32>> {
    selection::resolve(&present(path)?, wanted)
}

/// The exit code of a selection error: 5 for an unknown id, else 2 (usage).
pub(crate) fn code_of(e: &Error) -> u8 {
    if e.key == "err.sample.unknown" {
        UNKNOWN
    } else {
        2
    }
}

/// The failed `Outcome` of a selection error: the text on stderr and, with `json`, an `errore/1` on stdout.
pub(crate) fn fail(e: &Error, json: bool) -> Outcome {
    let (code, msg) = (code_of(e), crate::error_text::human_error(e));
    let stdout = if json {
        crate::flow_json::error_json(
            code,
            e,
            &msg,
            "list the samples with `mzlab-cli samples list FILE --json`",
            &[],
        )
        .into_bytes()
    } else {
        Vec::new()
    };
    Outcome {
        code,
        stdout,
        stderr: format!("error: {msg}\n"),
    }
}

#[cfg(test)]
#[path = "sample_select_tests.rs"]
mod tests;
