//! The blocks of a `.wiff.scan` for the readers module (feature `estesi`): everything of `scan.rs` plus the sparse (scheduled) MRM blocks.

#[allow(dead_code)]
#[path = "scan.rs"]
mod base;
pub use base::*;

use super::chromatograms::Mrm;
use crate::error::Result;
use crate::idxread::ReadAt;

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

/// The MRM block of `bytes` bytes starting at `at`. A dense block is one f32 (cps) per transition, negative noise included. A sparse block starts with `-(N + 0.01)` (`sparse_head`), N being the
/// transitions of the experiment, and goes on with the cps of consecutive transitions and `-(k + 0.01)` for a run of k transitions that measured zero, in
/// order: every transition gets its point, zeros included, as the vendor library gives them. A block that is neither (a negative or non-finite count, a
/// sparse block that does not add up to N, a NaN or infinite cps) is `Mrm::Damaged`: the reader leaves that cycle out and counts it, the others are read.
pub(crate) fn mrm_cols<R: ReadAt>(
    scan: &mut R,
    scan_len: u64,
    at: u64,
    bytes: usize,
) -> Result<Mrm<Cols>> {
    let Some(v) = base::read_mrm(scan, scan_len, at, bytes / 4)? else {
        return Ok(Mrm::Absent);
    };
    Ok(cols_of(v).map_or(Mrm::Damaged, Mrm::Read))
}

fn cols_of(v: Vec<f64>) -> Option<Cols> {
    if v.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let Some(&first) = v.first().filter(|x| base::sparse_head(**x)) else {
        return Some(Cols {
            width: v.len(),
            points: v.into_iter().enumerate().collect(),
        });
    };
    let width = (-first).floor();
    if !(width >= 1.0 && count(width) <= WIDTH_MAX) {
        return None;
    }
    let (mut pos, mut points) = (0usize, Vec::new());
    for &x in v.iter().skip(1) {
        if x < 0.0 {
            let run = count((-x).floor());
            if pos.saturating_add(run) > count(width) {
                return None;
            }
            points.extend((pos..pos + run).map(|j| (j, 0.0)));
            pos += run;
        } else {
            points.push((pos, x));
            pos = pos.saturating_add(1);
        }
    }
    (pos == count(width)).then_some(Cols { width: pos, points })
}

#[cfg(test)]
mod tests {
    use super::{mrm_cols, Mrm};

    fn block(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    // a skip count that saturates the position, followed by a point, is an error and not an overflow
    #[test]
    fn sparse_block_with_a_saturating_skip_is_an_error() {
        let mut b = block(&[-4.01, -1.0e30, 5.0, 6.0]);
        let n = b.len();
        assert!(matches!(mrm_cols(&mut b, n as u64, 0, n), Ok(Mrm::Damaged)));
    }

    fn cols(v: &[f32]) -> Option<(usize, Vec<(usize, f64)>)> {
        let mut b = block(v);
        let n = b.len();
        match mrm_cols(&mut b, n as u64, 0, n).ok()? {
            Mrm::Read(c) => Some((c.width, c.points)),
            _ => None,
        }
    }

    // 4 transitions: a cps, a run of two zeros, a cps; every transition gets its point
    #[test]
    fn sparse_block_fills_the_zero_runs() {
        assert_eq!(
            cols(&[-4.01, 7.0, -2.01, 9.0]),
            Some((4, vec![(0, 7.0), (1, 0.0), (2, 0.0), (3, 9.0)]))
        );
        // a block that ends short of its transitions, a width below one and a dense block
        assert_eq!(cols(&[-4.01, 7.0, -1.01]), None);
        assert_eq!(cols(&[-0.01, 7.0]), None);
        assert_eq!(cols(&[3.0, 4.0]), Some((2, vec![(0, 3.0), (1, 4.0)])));
        // a dense block whose first cps is negative noise, not of the `-(N + 0.01)` shape, stays dense
        assert_eq!(cols(&[-0.5, 7.0]), Some((2, vec![(0, -0.5), (1, 7.0)])));
        assert_eq!(cols(&[-4.0, 7.0]), Some((2, vec![(0, -4.0), (1, 7.0)])));
    }
}
