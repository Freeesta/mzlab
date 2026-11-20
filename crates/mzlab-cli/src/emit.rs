//! `emit`: where the text of a command goes, the standard output or the file of `--out` (a failed write is `err.cli.out`, exit 6).

use std::path::PathBuf;

use mzlab_core::error::Error;

use crate::Outcome;

/// The text to the standard output, or to the file of `--out`.
pub(crate) fn emit(t: impl Into<Vec<u8>>, out: &Option<PathBuf>) -> Outcome {
    let t: Vec<u8> = t.into();
    match out {
        None => Outcome::ok(t),
        Some(p) => std::fs::write(p, t).map_or_else(
            |e| {
                Outcome::fail_error(
                    &Error::new("err.cli.out").with("path", format!("{}: {e}", p.display())),
                )
            },
            |_| Outcome::ok(Vec::new()),
        ),
    }
}
