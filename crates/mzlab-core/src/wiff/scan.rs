//! The blocks of a `.wiff.scan`. A block starts `PAYLOAD_SKIP` bytes after the offset of its index row.
//! A scan block holds the marker `fe ff ff ff`, the version `V` (u32, 1 to 3; the 3200 QTRAP writes 2), `V` ranges of four f64 `[start, end, step, scale]`
//! (counts above the start of a later range use its scale), a u32 point count `n` and the points as a nibble stream; a block of another version or the `ffffffff` of
//! the TOF instruments is `err.file.wiff`, never a scan or an MRM block read by mistake; an MRM block holds one f32 (cps) per transition.
//! The points are bins of width `step` from `start1` with a count each; the m/z is `start1 + bin * step` and the intensity
//! `count * scale`. Zero bins are not stored. Format found on 3200 QTRAP data; the stream codec is described in `docs/agenti/wiff.md`.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;

pub const PAYLOAD_SKIP: u64 = 44;
pub const MARKER: [u8; 4] = [0xfe, 0xff, 0xff, 0xff];

/// m/z and intensity of one scan (profile points, increasing m/z).
pub struct Points {
    pub mz: Vec<f64>,
    pub intensity: Vec<f64>,
}

struct Nibbles<'a> {
    buf: &'a [u8],
    at: usize,
}

impl Nibbles<'_> {
    fn nib(&mut self) -> Option<u32> {
        let b = *self.buf.get(self.at >> 1)?;
        let v = if self.at & 1 == 0 { b >> 4 } else { b & 15 };
        self.at += 1;
        Some(v as u32)
    }

    /// The next whole byte (the rest of the current one is dropped); 0 means a u32 follows.
    fn long(&mut self) -> Option<u32> {
        self.at += self.at & 1;
        let i = self.at >> 1;
        let v = *self.buf.get(i)? as u32;
        self.at += 2;
        if v != 0 {
            return Some(v);
        }
        let w = self.buf.get(i + 1..i + 5)?;
        self.at += 8;
        Some(u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
    }
}

/// `(bin, count)` of up to `n` points: nibble 1..7 a point of that count; 0 a point with a long count; 8 a long skip; 9..15 a skip of nibble - 8.
pub fn decode(buf: &[u8], n: usize) -> Vec<(u64, u32)> {
    let mut out = Vec::with_capacity(n.min(buf.len() * 2));
    let (mut r, mut pos) = (Nibbles { buf, at: 0 }, 0u64);
    while out.len() < n {
        let Some(t) = r.nib() else { break };
        match t {
            0 => match r.long() {
                Some(c) => out.push((pos, c)),
                None => break,
            },
            1..=7 => out.push((pos, t)),
            8 => match r.long() {
                Some(s) => {
                    pos += s as u64;
                    continue;
                }
                None => break,
            },
            _ => {
                pos += (t - 8) as u64;
                continue;
            }
        }
        pos += 1;
    }
    out
}

/// Longest header: marker (8) + three ranges (96) + the point count (4).
pub(super) const HEAD_MAX: usize = 108;
const TOF_START: [u8; 4] = [0xff; 4];

pub(super) fn bad(detail: impl ToString) -> Error {
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
    if b[..4] != MARKER {
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

/// The cps of each of `n` transitions of an MRM block starting at `at`.
pub fn read_mrm<R: ReadAt>(
    scan: &mut R,
    scan_len: u64,
    at: u64,
    n: usize,
) -> Result<Option<Vec<f64>>> {
    if at + 4 * n as u64 > scan_len {
        return Ok(None);
    }
    let mut b = vec![0u8; 4 * n];
    scan.read_at(at, &mut b)?;
    Ok(Some(
        b.chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f64)
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn nibble_stream() {
        // a point of 3, a skip of 2, a point of 5, a point with the long count 300 (byte 0, then a u32) after the skipped low nibble
        assert_eq!(
            decode(&[0x3a, 0x50, 0x00, 0x2c, 0x01, 0x00, 0x00], 3),
            vec![(0, 3), (3, 5), (4, 300)]
        );
        // a long skip of 20 bins, then a point of 1
        assert_eq!(decode(&[0x80, 0x14, 0x10], 1), vec![(20, 1)]);
        // a stream that ends early gives the points it has
        assert_eq!(decode(&[0x11], 5).len(), 2);
    }
}
