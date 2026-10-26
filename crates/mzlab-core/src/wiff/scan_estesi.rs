//! The blocks of a `.wiff.scan` for the readers module (feature `estesi`): everything of `scan.rs` plus the sparse (scheduled) MRM blocks.

#[allow(dead_code)]
#[path = "scan.rs"]
mod base;
pub use base::*;

use crate::error::Result;
use crate::idxread::ReadAt;
use base::bad;

/// A count read from the file as a float: saturates, the callers compare it with their limits.
#[allow(clippy::cast_possible_truncation)]
fn count(x: f64) -> usize {
    x as usize
}

/// The transitions of an MRM block: `width` transitions in the experiment, and the `(transition, cps)` the block holds.
pub struct Cols {
    pub width: usize,
    pub points: Vec<(usize, f64)>,
}

/// Most transitions one experiment may have (a count read from the file).
const WIDTH_MAX: usize = 100_000;

/// The MRM block of `bytes` bytes starting at `at`. A dense block is one f32 (cps) per transition. A scheduled (sMRM) block starts with `-(N + 0.01)`, N being the
/// transitions of the experiment, and goes on with the cps of the transitions that were acquired and `-(k + 0.01)` for k transitions that were not, in order. A block that is neither
/// (a negative or non-finite count, a sparse block that does not add up to N) is an error.
pub fn read_mrm_cols<R: ReadAt>(
    scan: &mut R,
    scan_len: u64,
    at: u64,
    bytes: usize,
) -> Result<Option<Cols>> {
    let Some(v) = base::read_mrm(scan, scan_len, at, bytes / 4)? else {
        return Ok(None);
    };
    if let Some(&first) = v.first().filter(|x| **x < 0.0) {
        let width = (-first).floor();
        if !(first.is_finite() && width >= 1.0 && count(width) <= WIDTH_MAX) {
            return Err(bad("sparse MRM block"));
        }
        let (mut pos, mut points) = (0usize, Vec::new());
        for &x in v.iter().skip(1) {
            if x < 0.0 {
                pos = pos.saturating_add(count((-x).floor()));
            } else if x.is_finite() {
                points.push((pos, x));
                pos += 1;
            } else {
                return Err(bad("sparse MRM block"));
            }
        }
        if pos != count(width) {
            return Err(bad("sparse MRM block"));
        }
        return Ok(Some(Cols { width: pos, points }));
    }
    if v.iter().any(|x| !x.is_finite()) {
        return Err(bad("MRM block"));
    }
    Ok(Some(Cols {
        width: v.len(),
        points: v.into_iter().enumerate().collect(),
    }))
}
