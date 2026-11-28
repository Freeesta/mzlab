//! `raw isotopes --figure PATH`: writes the SVG each result answered (the bytes of the route `/api/rawiso?figure=1`) and builds the `figures` member of
//! the `raw-isotopes/1` document: file, sample, path, bytes and sha256 of each; a path that cannot be written is exit 6.

use std::path::PathBuf;

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::Error;
use mzlab_core::json::Out;

use crate::extract::{jstr, Parsed};
use crate::raw_cmd::choice;
use crate::Outcome;

/// `PATH` for one figure; for several the number of the result goes in front of the extension (`fig.svg` -> `fig-2.svg`).
fn figure_path(base: &str, i: usize, n: usize) -> PathBuf {
    let path = PathBuf::from(base);
    if n < 2 {
        return path;
    }
    let stem = path
        .file_stem()
        .map_or(String::new(), |s| s.to_string_lossy().into_owned());
    let ext = path
        .extension()
        .map_or(String::new(), |e| format!(".{}", e.to_string_lossy()));
    path.with_file_name(format!("{stem}-{}{ext}", i + 1))
}

/// Writes the figures of `--figure` and returns the `"figures":[..],` member (empty without the flag): per figure the file, sample, path, bytes and sha256.
/// A path that cannot be written is `err.cli.out`, exit 6.
pub(super) fn write_figures(
    p: &Parsed,
    figs: &[(String, Option<u32>, String)],
    choices: &mut Vec<String>,
) -> std::result::Result<String, Outcome> {
    let Some(base) = p.get("--figure") else {
        return Ok(String::new());
    };
    let mut rows = Vec::new();
    for (i, (file, sample, svg)) in figs.iter().enumerate() {
        let path = figure_path(base, i, figs.len());
        std::fs::write(&path, svg).map_err(|e| {
            Outcome::fail_error(
                &Error::new("err.cli.out").with("path", format!("{}: {e}", path.display())),
            )
        })?;
        let mut o = Out::default();
        o.raw("{\"file\":").str(file).raw(",\"sample\":");
        o.raw(&sample.map_or("null".into(), |s| s.to_string()));
        o.raw(",\"path\":").str(&path.to_string_lossy());
        o.raw(",\"bytes\":").int(svg.len() as i64);
        o.raw(",\"sha256\":")
            .str(&sha256_hex(svg.as_bytes()))
            .raw("}");
        rows.push(o.0);
    }
    if figs.len() > 1 {
        choices.push(choice(
            "figure",
            &jstr(base),
            "several results: each figure is written to PATH with its number in front of the extension, in the order of `results`",
        ));
    }
    Ok(format!("\"figures\":[{}],", rows.join(",")))
}
