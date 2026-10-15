//! The blocks of a `.wiff.scan` for the readers module (feature `estesi`): everything of `scan.rs` plus the scan blocks of versions 1 to 3.
//! After the marker `fe ff ff ff` comes the version `V` (u32), `V` ranges of four f64 `[start, end, step, scale]`, the point count (u32) and the
//! nibble stream of `scan.rs`. The m/z is `start1 + bin * step1`; the intensity is `count * scale` of the last range whose start is below the m/z
//! (the 3200 writes V=2, the other QTRAPs V=1..3). A block that starts like a scan but is not one of these (another version, the `ffffffff` of the
//! TOF instruments) is an error, never a scan or an MRM block read by mistake.

#[allow(dead_code)]
#[path = "scan.rs"]
mod base;
pub use base::*;

use crate::error::{Error, Result};
use crate::idxread::ReadAt;

/// Longest header: marker (8) + three ranges (96) + the point count (4).
const HEAD_MAX: usize = 108;
const TOF_START: [u8; 4] = [0xff; 4];

fn bad(detail: impl ToString) -> Error {
    Error::new("err.file.wiff").with("detail", detail)
}

fn f64_at(b: &[u8], at: usize) -> f64 {
    b.get(at..at + 8)
        .and_then(|s| <[u8; 8]>::try_from(s).ok())
        .map_or(0.0, f64::from_le_bytes)
}

fn u32_at(b: &[u8], at: usize) -> usize {
    b.get(at..at + 4)
        .and_then(|s| <[u8; 4]>::try_from(s).ok())
        .map_or(0, |a| u32::from_le_bytes(a) as usize)
}

/// A count read from the file as a float (finite and not negative where it matters): saturates, the callers compare it with their limits.
#[allow(clippy::cast_possible_truncation)]
fn count(x: f64) -> usize {
    x as usize
}

/// The head of a scan block: ranges `[start, end, step, scale]`, the point count and the length of the head. `None` when the block is not a scan.
struct Head {
    ranges: Vec<[f64; 4]>,
    n: usize,
    len: usize,
}

fn head<R: ReadAt>(scan: &mut R, scan_len: u64, at: u64, end: u64) -> Result<Option<Head>> {
    let avail =
        usize::try_from(end.min(scan_len).saturating_sub(at)).map_or(HEAD_MAX, |a| a.min(HEAD_MAX));
    if avail < 8 {
        return Ok(None);
    }
    let mut b = [0u8; HEAD_MAX];
    let Some(head_bytes) = b.get_mut(..avail) else {
        return Ok(None);
    };
    scan.read_at(at, head_bytes)?;
    if b[..4] == TOF_START {
        return Err(bad("TOF block"));
    }
    if b[..4] != MARKER[..4] {
        return Ok(None);
    }
    let v = u32_at(&b, 4);
    if !(1..=3).contains(&v) {
        return Err(bad(format!("scan block version {v}")));
    }
    let len = 12 + 32 * v;
    if avail < len {
        return Err(bad("short scan block"));
    }
    let ranges = (0..v)
        .map(|k| [0, 1, 2, 3].map(|j| f64_at(&b, 8 + 32 * k + 8 * j)))
        .collect();
    let n = u32_at(&b, len - 4);
    Ok(Some(Head { ranges, n, len }))
}

/// The scan whose block starts at `at` (the payload) and whose data end before `end`; `None` when the block is not a scan.
/// An empty scan gives empty vectors.
pub fn read_scan<R: ReadAt>(
    scan: &mut R,
    scan_len: u64,
    at: u64,
    end: u64,
) -> Result<Option<Points>> {
    let Some(h) = head(scan, scan_len, at, end)? else {
        return Ok(None);
    };
    let end = end.min(scan_len);
    // at most 10 bytes for a long skip and 10 for a long count per point
    let mut data = vec![
        0u8;
        usize::try_from(end - at)
            .unwrap_or(usize::MAX)
            .saturating_sub(h.len)
            .min(h.n.saturating_mul(20).saturating_add(16))
    ];
    if h.n > 0 {
        scan.read_at(at + h.len as u64, &mut data)?;
    }
    let pts = if h.n > 0 {
        decode(&data, h.n)
    } else {
        Vec::new()
    };
    let Some(&r0) = h.ranges.first() else {
        return Ok(None);
    };
    let mz: Vec<f64> = pts.iter().map(|&(b, _)| r0[0] + b as f64 * r0[2]).collect();
    let intensity = pts
        .iter()
        .zip(&mz)
        .map(|(&(_, c), &m)| {
            c as f64
                * h.ranges
                    .iter()
                    .skip(1)
                    .rev()
                    .find(|r| m > r[0])
                    .map_or(r0[3], |r| r[3])
        })
        .collect();
    Ok(Some(Points { mz, intensity }))
}

/// The number of points of the scan block at `at`, `None` when the block is not a scan.
pub fn point_count<R: ReadAt>(
    scan: &mut R,
    scan_len: u64,
    at: u64,
    end: u64,
) -> Result<Option<u32>> {
    Ok(head(scan, scan_len, at, end)?.map(|h| u32::try_from(h.n).unwrap_or(u32::MAX)))
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
