//! The blocks of a `.wiff.scan`. A block starts `PAYLOAD_SKIP` bytes after the offset of its index row.
//! A scan block holds `MARKER`, eight f64 `[start1, end1, step, scale1, start2, end2, step2, scale2]` (two intensity scales: counts above
//! `start2` use `scale2`), a u32 point count `n` and the points as a nibble stream; an MRM block holds one f32 (cps) per transition.
//! The points are bins of width `step` from `start1` with a count each; the m/z is `start1 + bin * step` and the intensity
//! `count * scale`. Zero bins are not stored. Format found on 3200 QTRAP data; the stream codec is described in `docs/agenti/wiff.md`.

use crate::error::Result;
use crate::idxread::ReadAt;

pub const PAYLOAD_SKIP: u64 = 44;
pub const MARKER: [u8; 8] = [0xfe, 0xff, 0xff, 0xff, 2, 0, 0, 0];
const HEAD: usize = 76;

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

fn f64_at(b: &[u8], at: usize) -> f64 {
    let mut t = [0u8; 8];
    t.copy_from_slice(&b[at..at + 8]);
    f64::from_le_bytes(t)
}

/// The scan whose block starts at `at` (the payload) and whose data end before `end`; `None` when the block is not a scan.
/// An empty scan gives empty vectors.
pub fn read_scan<R: ReadAt>(
    scan: &mut R,
    scan_len: u64,
    at: u64,
    end: u64,
) -> Result<Option<Points>> {
    let end = end.min(scan_len);
    if at + HEAD as u64 > end {
        return Ok(None);
    }
    let mut head = [0u8; HEAD];
    scan.read_at(at, &mut head)?;
    if head[..8] != MARKER {
        return Ok(None);
    }
    let h: Vec<f64> = (0..8).map(|i| f64_at(&head, 8 + 8 * i)).collect();
    let n = u32::from_le_bytes([head[72], head[73], head[74], head[75]]) as usize;
    // at most 10 bytes for a long skip and 10 for a long count per point
    let mut data =
        vec![0u8; ((end - at) as usize - HEAD).min(n.saturating_mul(20).saturating_add(16))];
    if n > 0 {
        scan.read_at(at + HEAD as u64, &mut data)?;
    }
    let pts = if n > 0 { decode(&data, n) } else { Vec::new() };
    let mz: Vec<f64> = pts.iter().map(|&(b, _)| h[0] + b as f64 * h[2]).collect();
    let intensity = pts
        .iter()
        .zip(&mz)
        .map(|(&(_, c), &m)| c as f64 * if m > h[4] { h[7] } else { h[3] })
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
    if at + HEAD as u64 > end.min(scan_len) {
        return Ok(None);
    }
    let mut head = [0u8; HEAD];
    scan.read_at(at, &mut head)?;
    Ok((head[..8] == MARKER).then(|| u32::from_le_bytes([head[72], head[73], head[74], head[75]])))
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
