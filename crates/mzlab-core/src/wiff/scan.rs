//! The blocks of a `.wiff.scan`. A block starts `PAYLOAD_SKIP` bytes after the offset of its index row.
//! A scan block holds the marker `fe ff ff ff`, the version `V` (u32, 1 to 3; the 3200 QTRAP writes 2), `V` ranges of four f64 `[start, end, step, scale]`
//! (counts above the start of a later range use its scale), a u32 point count `n` and the points as a nibble stream; a block of another version or the `ffffffff` of
//! the TOF instruments is a damaged block, never a scan or an MRM block read by mistake (`Tally`); an MRM block holds one f32 (cps) per transition.
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
    /// The version of the block's head (1 to 3): a 3200 writes 2 for survey scans and 3 for enhanced product ion scans; another one marks the sample.
    pub version: usize,
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
    decode_used(buf, n).0
}

/// As `decode`, with the bytes of `buf` the points took (a final half byte counts whole).
fn decode_used(buf: &[u8], n: usize) -> (Vec<(u64, u32)>, usize) {
    let mut out = Vec::with_capacity(n.min(buf.len().saturating_mul(2)));
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
    (out, r.at.div_ceil(2))
}

/// Bytes a decoded point holds at once: `(bin, count)` (16), m/z and intensity (16), the f32 and f64 intensities of the spectrum (12).
const POINT_BYTES: usize = 44;
/// The memory one scan may take: the budget of the target (the site 192 MB, the app 1 GB).
const BUDGET: usize = if cfg!(target_family = "wasm") {
    crate::budget::WASM_BUDGET_MB
} else {
    crate::budget::NATIVE_BUDGET_MB
} << 20;

/// `n` zero bytes, or `error.memoryBudget` when the memory does not hold them: `n` is a length read from the file.
fn zeroed(n: usize) -> Result<Vec<u8>> {
    let mut v = crate::budget::try_vec(n)?;
    v.resize(n, 0);
    Ok(v)
}

/// Longest header: marker (8) + three ranges (96) + the point count (4).
pub const HEAD_MAX: usize = 108;
const TOF_START: [u8; 4] = [0xff; 4];

pub(super) fn bad(detail: impl ToString) -> Error {
    Error::new("err.file.wiff").with("detail", detail)
}

/// Why a block is `Block::Damaged`: an error only when a whole sample has nothing else (`Tally::check`), or for `read_scan`.
#[derive(Clone, Copy)]
pub enum Cause {
    Tof,
    Version(usize),
    /// The bytes contradict the head: a head longer than the block, fewer points than it declares, points that do not fill the block.
    Bytes,
}

impl Cause {
    fn error(self) -> Error {
        match self {
            Cause::Tof => bad("TOF block"),
            Cause::Version(v) => bad(format!("scan block version {v}")),
            Cause::Bytes => bad("the bytes contradict the scan head"),
        }
    }
}

/// A block read: `Ok(None)` when it is not a scan, `Err` when its bytes make it a damaged one.
type Shape<T> = std::result::Result<Option<T>, Cause>;

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

fn head(scan: &mut dyn ReadAt, scan_len: u64, at: u64, end: u64) -> Result<Shape<Head>> {
    let avail =
        usize::try_from(end.min(scan_len).saturating_sub(at)).map_or(HEAD_MAX, |a| a.min(HEAD_MAX));
    if avail < 8 {
        return Ok(Ok(None));
    }
    let mut b = [0u8; HEAD_MAX];
    let Some(head_bytes) = b.get_mut(..avail) else {
        return Ok(Ok(None));
    };
    scan.read_at(at, head_bytes)?;
    if b[..4] == TOF_START {
        return Ok(Err(Cause::Tof));
    }
    if b[..4] != MARKER {
        return Ok(Ok(None));
    }
    let v = u32_at(&b, 4);
    if !(1..=3).contains(&v) {
        return Ok(Err(Cause::Version(v)));
    }
    let len = 12 + 32 * v;
    if avail < len {
        return Ok(Err(Cause::Bytes));
    }
    let ranges = (0..v)
        .map(|k| [0, 1, 2, 3].map(|j| f64_at(&b, 8 + 32 * k + 8 * j)))
        .collect();
    let n = u32_at(&b, len - 4);
    Ok(Ok(Some(Head { ranges, n, len })))
}

/// Most bytes a block keeps after its points: the 3200 QTRAP files pad a stream of `u` bytes to `u + 4 - u % 4` (1 to 4), the reference writer adds none.
const PADDING: usize = 4;

/// The scan whose block starts at `at` (the payload) and whose data end before `end`; `None` when the block is not a scan.
/// An empty scan gives empty vectors; a block whose stream holds fewer points than its head declares is `err.file.wiff`.
pub fn read_scan<R: ReadAt>(
    scan: &mut R,
    scan_len: u64,
    at: u64,
    end: u64,
) -> Result<Option<Points>> {
    read_sized(scan, scan_len, at, end, 0)?.map_err(Cause::error)
}

/// What the bytes of a block hold (`read_block`): no scan head (an MRM block), a scan, or a scan head they contradict.
pub enum Block {
    Other,
    Scan(Points),
    Damaged,
}

impl Block {
    /// The point count the index takes: `None` with no scan head, 0 for a damaged block (left out, never read as MRM values).
    pub fn count(&self) -> Option<u32> {
        match self {
            Block::Scan(p) => Some(u32::try_from(p.mz.len()).unwrap_or(u32::MAX)),
            Block::Other => None,
            Block::Damaged => Some(0),
        }
    }
}

/// The damaged blocks of a sample: each is left out and counted; a sample with no other block (no scan, no MRM block) is refused with the reason of the
/// first, since nothing of it can be read.
#[derive(Default)]
pub struct Tally {
    usable: bool,
    first: Option<Cause>,
    pub damaged: usize,
}

impl Tally {
    /// The count of damaged blocks, or the refusal of a sample that has nothing else (with that count: `blocks`).
    pub fn check(self) -> Result<usize> {
        match self.first {
            Some(c) if !self.usable => Err(c.error().with("blocks", self.damaged)),
            _ => Ok(self.damaged),
        }
    }
}

/// The block of the index row `r`, of `size` bytes (0: not known). Its points must take those bytes, up to `PADDING` left; a TOF block, a head of
/// an unknown version, or one whose length or point count the bytes contradict is `Block::Damaged`, which the callers leave out and count
/// (`warn.wiff.damaged`, `Tally`): one odd block never refuses the sample. The index row's value at +26 is not a point count on every file (sparse product-ion scans), so the block is judged by its bytes.
pub fn read_block(
    scan: &mut dyn ReadAt,
    scan_len: u64,
    r: &crate::wiff::Record,
    tally: Option<&mut Tally>,
) -> Result<Block> {
    // a block is never read past its own length (the last block of a file runs to the end of the file)
    let b = match read_sized(scan, scan_len, r.at, r.data_end(), r.size as usize)? {
        Ok(Some(p)) => Block::Scan(p),
        Ok(None) => Block::Other,
        Err(c) => {
            if let Some(t) = tally {
                t.damaged += 1;
                t.first.get_or_insert(c);
            }
            return Ok(Block::Damaged);
        }
    };
    if let Some(t) = tally {
        t.usable = true;
    }
    Ok(b)
}

// one copy of the decoder whatever reader serves the bytes (the site's window and its member, the app's file): `dyn`, not generic
fn read_sized(
    scan: &mut dyn ReadAt,
    scan_len: u64,
    at: u64,
    end: u64,
    size: usize,
) -> Result<Shape<Points>> {
    let h = match head(scan, scan_len, at, end)? {
        Ok(Some(h)) => h,
        Ok(None) => return Ok(Ok(None)),
        Err(c) => return Ok(Err(c)),
    };
    let end = end.min(scan_len);
    // at most 10 bytes for a long skip and 10 for a long count per point; the block may still claim more than the memory holds (a calm error, never a trap)
    let len = usize::try_from(end - at)
        .unwrap_or(usize::MAX)
        .saturating_sub(h.len)
        .min(h.n.saturating_mul(20).saturating_add(16));
    // a byte holds at most two points: the count of the head is believed only up to that, and the points must fit the memory budget of the target
    let n = h.n.min(len.saturating_mul(2));
    if n.saturating_mul(POINT_BYTES) > BUDGET {
        return Err(Error::memory_budget());
    }
    let mut data = zeroed(len)?;
    if n > 0 {
        scan.read_at(at + h.len as u64, &mut data)?;
    }
    let (pts, used) = if n > 0 {
        decode_used(&data, n)
    } else {
        (Vec::new(), 0)
    };
    // a stream that ends before the points its head declares is a damaged block: never a scan read short
    if pts.len() != h.n {
        return Ok(Err(Cause::Bytes));
    }
    let left = size.saturating_sub(h.len).saturating_sub(used);
    if size > 0 && (h.len + used > size || left > PADDING) {
        return Ok(Err(Cause::Bytes));
    }
    let Some(&r0) = h.ranges.first() else {
        return Ok(Ok(None));
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
    Ok(Ok(Some(Points {
        mz,
        intensity,
        version: h.ranges.len(),
    })))
}

/// The cps of each of `n` transitions of an MRM block starting at `at`.
pub fn read_mrm<R: ReadAt>(
    scan: &mut R,
    scan_len: u64,
    at: u64,
    n: usize,
) -> Result<Option<Vec<f64>>> {
    let Some(len) = n
        .checked_mul(4)
        .filter(|&l| at.saturating_add(l as u64) <= scan_len)
    else {
        return Ok(None);
    };
    let mut b = zeroed(len)?;
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
