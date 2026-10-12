//! Decoding of one block of a `.wiff.scan` file: a zero-suppressed token stream (gap / 1-5 byte intensity), ended by `ff ff ff ff` at a token
//! boundary. Format after OpenSXRaw (Sigilweaver, Apache-2.0, `src/raw/scan.rs`), re-written here; see `docs/agenti/wiff.md`.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;

/// Bytes at the start of a block that belong to the previous block's tail and a sync region.
pub const BLOCK_HEAD: u64 = 56;
/// Slack after the declared end of a block, for the terminator that lies in the head window of the next one.
const SLACK: u64 = 64;
/// No real block comes near this; it bounds the allocation for a damaged index.
const MAX_BLOCK: u64 = 64 << 20;

/// A decoded point: the accumulated bin of the detector axis (not an m/z, see `wiff.md`) and the raw count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub bin: u32,
    pub count: u32,
}

pub fn decode(p: &[u8]) -> Vec<Point> {
    let (mut out, mut bin, mut i) = (Vec::new(), 0u32, 0usize);
    while i < p.len() {
        let b = p[i];
        let n = match b {
            0xfc..=0xff => (b - 0xfb) as usize,
            _ => 0,
        };
        if b == 0xff && p.get(i..i + 4) == Some(&[0xff; 4]) {
            break;
        }
        match b {
            0..=0x7f => bin = bin.wrapping_add(b as u32),
            0x80..=0xfb => out.push(Point { bin, count: (b & 0x7f) as u32 }),
            _ => {
                let Some(v) = p.get(i + 1..i + 1 + n) else { break };
                let count = v.iter().rev().fold(0u32, |a, &x| (a << 8) | x as u32);
                out.push(Point { bin, count });
            }
        }
        i += 1 + n;
    }
    out
}

/// The points of the block that starts at `offset` (declared `size`); `next` is the offset of the following block (or the file length).
pub fn read_block<R: ReadAt>(scan: &mut R, len: u64, offset: u32, size: u32, next: u64) -> Result<Vec<Point>> {
    let start = offset as u64 + BLOCK_HEAD;
    let end = (next + SLACK).min(offset as u64 + size as u64 + SLACK).min(len).min(start + MAX_BLOCK);
    if end <= start {
        return Ok(vec![]);
    }
    let mut buf = vec![0u8; (end - start) as usize];
    scan.read_at(start, &mut buf).map_err(|_| Error::new("err.file.wiff").with("detail", "scan block"))?;
    Ok(decode(&buf))
}
