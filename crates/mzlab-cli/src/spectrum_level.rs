//! The MS level `spectrum get` starts from when `--ms-level` is absent: the one level of the file (logged on stderr and in `evidence.level_auto`), the
//! default 1 when the file has it, else `err.scan.level` naming the levels and the flag to give.

use mzlab_core::error::{Error, Result};

use super::At;
use crate::extract::Sample;

/// The level `spectrum get` uses when `--ms-level` is absent, with the auto-choice to log: `(level, Some(levels))` when the file's only MS level is not
/// the documented default 1. A file whose levels exclude 1 and number several is `err.scan.level` naming them and the flag to give; one that has level 1 (or no
/// scan at all, left to the usual "no scan" answer) keeps the default 1.
fn default_level(s: &Sample) -> Result<(u8, Option<Vec<u8>>)> {
    let mut levels: Vec<u8> = s.routes.idx.scans.iter().map(|x| x.level).collect();
    levels.sort_unstable();
    levels.dedup();
    match levels.as_slice() {
        [only] if *only != 1 => Ok((*only, Some(levels))),
        [first, _, ..] if *first != 1 => Err(Error::new("err.scan.level")
            .with(
                "levels",
                levels
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .with("hint", format!("--ms-level {first}"))),
        _ => Ok((1, None)),
    }
}

/// The level a request starts from and the `level_auto` evidence when the file's only level was chosen: the one given, 1 for a scan given by number (its own
/// level replaces it), else `default_level`, logged on stderr when it is not the documented default.
pub(super) fn start(s: &mut Sample, at: &At, given: Option<u8>) -> Result<(u8, Option<String>)> {
    if let Some(l) = given {
        return Ok((l, None));
    }
    if matches!(at, At::Scan(_)) {
        return Ok((1, None));
    }
    let (l, only) = default_level(s)?;
    let Some(levels) = only else {
        return Ok((l, None));
    };
    s.note += &format!(
        "note: {}\n",
        crate::error_text::warning(&crate::convert::Notice {
            key: "warn.scan.levelOnly",
            params: vec![("name", s.name.clone()), ("level", l.to_string())],
        })
    );
    let list = levels
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let why = "the only MS level in the file";
    Ok((
        l,
        Some(format!(
            "{{\"ms_level\":{l},\"levels\":[{list}],\"why\":\"{why}\"}}"
        )),
    ))
}
