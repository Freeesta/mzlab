//! The frame blocks of `analysis.tdf_bin`: at `Frames.TimsId` a block starts with its length (u32) and the number of scans (u32). Compression type 2 (zstd): the payload is `num_scans + 2 * num_peaks`
//! little-endian words stored as four byte planes; the first `num_scans` words are the peak counts (the count of scan `s` sits at word `s + 1`, the last scan takes the rest), then each peak is a
//! (TOF index delta, intensity) pair and the TOF index restarts at every scan (the first delta is added to -1). Compression type 1 (older files): a table of `num_scans + 1` offsets and one LZF
//! blob per scan, read as signed words (a positive word is an intensity, a negative one skips TOF indices). Written from the public descriptions of the format (OpenTIMS, opentimstdf, AlphaTims).

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::Limits;

fn broken(m: &str) -> Error {
    Error::new("err.file.brukerDamaged").with("detail", format!("tdf_bin: {m}"))
}

/// The peaks of a frame, in scan order: the points of scan `s` (0-based) are `inizio[s]..inizio[s + 1]`, each a TOF index and an intensity.
#[derive(Debug, Clone, PartialEq)]
pub struct Peaks {
    pub start: Vec<usize>,
    pub tof: Vec<u32>,
    pub int: Vec<u32>,
}

impl Default for Peaks {
    fn default() -> Peaks {
        Peaks {
            start: vec![0],
            tof: Vec::new(),
            int: Vec::new(),
        }
    }
}

impl Peaks {
    /// Room for `ns` scans and `np` points (both already checked against `Limiti`).
    fn con(ns: usize, np: usize) -> Peaks {
        let mut start = Vec::with_capacity(ns + 1);
        start.push(0);
        Peaks {
            start,
            tof: Vec::with_capacity(np),
            int: Vec::with_capacity(np),
        }
    }
    pub fn len(&self) -> usize {
        self.tof.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tof.is_empty()
    }
    fn push(&mut self, tof: u32, int: u32) {
        self.tof.push(tof);
        self.int.push(int);
    }
    /// Closes the current scan.
    fn fine_scan(&mut self) {
        self.start.push(self.tof.len());
    }
    /// The points of the scans `a..b` (cut to the scans of the frame).
    pub fn points(&self, a: u32, b: u32) -> std::ops::Range<usize> {
        let n = self.start.len().saturating_sub(1);
        let (a, b) = ((a as usize).min(n), (b as usize).min(n));
        if a >= b {
            return 0..0;
        }
        self.start[a]..self.start[b]
    }
}

fn u32_le(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// Reads the block at `offset` of a `tdf_bin` of `dim` bytes: the whole block, header included.
pub fn read_block(d: &mut dyn ReadAt, dim: u64, offset: u64, limits: &Limits) -> Result<Vec<u8>> {
    let mut h = [0u8; 8];
    if offset.checked_add(8).is_none_or(|e| e > dim) {
        return Err(broken("block past the end of the file"));
    }
    d.read_at(offset, &mut h)?;
    let len = u64::from(u32_le(&h, 0).unwrap_or(0));
    if len < 8 {
        return Ok(h.to_vec());
    }
    if offset + len > dim || len > limits.byte {
        return Err(broken("block length"));
    }
    let mut b = vec![0u8; len as usize];
    d.read_at(offset, &mut b)?;
    Ok(b)
}

/// The peaks of a block. `num_scans` and `num_peaks` come from the `Frames` row and are checked against the block and the limits.
pub fn decode(
    block: &[u8],
    kind: u32,
    num_scans: u64,
    num_peaks: u64,
    limits: &Limits,
) -> Result<Peaks> {
    limits.ok_entries(num_scans)?;
    limits.ok_entries(num_peaks)?;
    if block.len() <= 8 {
        return if num_peaks == 0 {
            Ok(Peaks::default())
        } else {
            Err(broken("empty block with peaks"))
        };
    }
    let ns = u64::from(u32_le(block, 4).unwrap_or(u32::MAX));
    limits.ok_entries(ns)?;
    // type 2 repeats the scan count of the frame table; type 1 lists the whole ramp
    if kind == 2 && ns != num_scans {
        return Err(broken("scan count differs from the frame table"));
    }
    let num_scans = ns;
    match kind {
        2 => decodifica2(&block[8..], num_scans as usize, num_peaks as usize, limits),
        1 => decodifica1(block, num_scans as usize, num_peaks as usize, limits),
        _ => Err(broken("unknown compression type")),
    }
}

/// The window a zstd frame announces for its decoder (None: not a zstd frame). A hostile header cannot make the decoder allocate more than the limit.
fn zstd_window(z: &[u8]) -> Option<u64> {
    if z.get(..4)? != [0x28, 0xb5, 0x2f, 0xfd] {
        return None;
    }
    let d = *z.get(4)?;
    let single = d & 0x20 != 0;
    let fcs = match d >> 6 {
        0 => u64::from(single),
        1 => 2,
        2 => 4,
        _ => 8,
    } as usize;
    let mut o = 5;
    let w = if single {
        None
    } else {
        o += 1;
        let b = *z.get(5)?;
        let base = 1u64 << (10 + u32::from(b >> 3));
        Some(base + base / 8 * u64::from(b & 7))
    };
    o += if d & 3 == 3 { 4 } else { usize::from(d & 3) };
    let mut v = 0u64;
    for (i, x) in z.get(o..o + fcs)?.iter().enumerate() {
        v |= u64::from(*x) << (8 * i);
    }
    Some(w.unwrap_or(if fcs == 2 { v + 256 } else { v }))
}

/// One zstd frame that must decompress to exactly `attesi` bytes (the frame header and the output are bounded by `limiti` before any allocation).
pub fn zstd(z: &[u8], expected: usize, limits: &Limits) -> Result<Vec<u8>> {
    use std::io::Read;
    if expected as u64 > limits.byte || zstd_window(z).is_none_or(|w| w > limits.byte) {
        return Err(broken("zstd frame"));
    }
    let mut dec = ruzstd::decoding::StreamingDecoder::new(z).map_err(|_| broken("zstd"))?;
    let mut inner = Vec::with_capacity(expected);
    (&mut dec)
        .take(expected as u64 + 1)
        .read_to_end(&mut inner)
        .map_err(|_| broken("zstd data"))?;
    if inner.len() != expected {
        return Err(broken("decompressed size"));
    }
    Ok(inner)
}

fn decodifica2(z: &[u8], ns: usize, np: usize, limits: &Limits) -> Result<Peaks> {
    let words = ns
        .checked_add(np.checked_mul(2).ok_or_else(|| broken("peaks"))?)
        .ok_or_else(|| broken("peaks"))?;
    let expected = words.checked_mul(4).ok_or_else(|| broken("peaks"))?;
    let inner = zstd(z, expected, limits)?;
    let word = |i: usize| {
        u32::from(inner[i])
            | u32::from(inner[words + i]) << 8
            | u32::from(inner[2 * words + i]) << 16
            | u32::from(inner[3 * words + i]) << 24
    };
    // the zstd frame gave exactly the announced words, so the counts below are bounded by the block
    let mut out = Peaks::con(ns, np);
    let mut read_items = 0usize;
    for scan in 0..ns {
        let n = if scan + 1 < ns {
            (word(scan + 1) / 2) as usize
        } else {
            np.checked_sub(out.len())
                .ok_or_else(|| broken("peak counts"))?
        };
        let fine = read_items
            .checked_add(n.checked_mul(2).ok_or_else(|| broken("peak counts"))?)
            .filter(|&f| f <= words - ns)
            .ok_or_else(|| broken("peak counts"))?;
        let mut tof = u32::MAX;
        while read_items < fine {
            tof = tof.wrapping_add(word(ns + read_items));
            out.push(tof, word(ns + read_items + 1));
            read_items += 2;
        }
        out.fine_scan();
    }
    if out.len() != np {
        return Err(broken("peak total"));
    }
    Ok(out)
}

fn decodifica1(b: &[u8], ns: usize, np: usize, limits: &Limits) -> Result<Peaks> {
    let tab = (ns + 1) * 4;
    let start = 8 + tab;
    if start > b.len() {
        return Err(broken("offset table"));
    }
    let mut off = Vec::with_capacity(ns + 1);
    for i in 0..=ns {
        let o = u32_le(b, 8 + 4 * i).ok_or_else(|| broken("offset table"))? as usize;
        if o < start || o > b.len() || off.last().is_some_and(|&p| o < p) {
            return Err(broken("scan offset"));
        }
        off.push(o);
    }
    let mut out = Peaks::con(ns, 0);
    let mut buf = Vec::new();
    for scan in 0..ns {
        if off[scan] == off[scan + 1] {
            out.fine_scan();
            continue;
        }
        buf.clear();
        lzf(&b[off[scan]..off[scan + 1]], &mut buf, limits.byte as usize)?;
        if buf.len() % 4 != 0 {
            return Err(broken("scan size"));
        }
        // a negative word skips that many TOF bins; a non-negative one is an intensity at the current bin, and the bin moves on
        let mut tof = 0u32;
        for w in buf.chunks_exact(4) {
            let v = i32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            if v >= 0 {
                if out.len() >= np {
                    return Err(broken("more peaks than the frame table"));
                }
                out.push(tof, v as u32);
                tof = tof.wrapping_add(1);
            } else {
                tof = tof.wrapping_add(v.unsigned_abs());
            }
        }
        out.fine_scan();
    }
    if out.len() != np {
        return Err(broken("peak total"));
    }
    Ok(out)
}

/// libLZF decompression of `z` into `out`, never past `max` bytes.
fn lzf(z: &[u8], out: &mut Vec<u8>, max: usize) -> Result<()> {
    let mut i = 0;
    while i < z.len() {
        let c = usize::from(z[i]);
        i += 1;
        if c < 32 {
            let n = c + 1;
            let s = z.get(i..i + n).ok_or_else(|| broken("lzf literal"))?;
            if out.len() + n > max {
                return Err(broken("lzf size"));
            }
            out.extend_from_slice(s);
            i += n;
        } else {
            let mut n = c >> 5;
            if n == 7 {
                n += usize::from(*z.get(i).ok_or_else(|| broken("lzf length"))?);
                i += 1;
            }
            n += 2;
            let dist =
                ((c & 0x1f) << 8) | usize::from(*z.get(i).ok_or_else(|| broken("lzf distance"))?);
            i += 1;
            let da = out
                .len()
                .checked_sub(dist + 1)
                .ok_or_else(|| broken("lzf reference"))?;
            if out.len() + n > max {
                return Err(broken("lzf size"));
            }
            for k in 0..n {
                out.push(out[da + k]);
            }
        }
    }
    Ok(())
}
