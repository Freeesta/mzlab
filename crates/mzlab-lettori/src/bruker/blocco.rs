//! The frame blocks of `analysis.tdf_bin`: at `Frames.TimsId` a block starts with its length (u32) and the number of scans (u32). Compression type 2 (zstd): the payload is `num_scans + 2 * num_peaks`
//! little-endian words stored as four byte planes; the first `num_scans` words are the peak counts (the count of scan `s` sits at word `s + 1`, the last scan takes the rest), then each peak is a
//! (TOF index delta, intensity) pair and the TOF index restarts at every scan (the first delta is added to -1). Compression type 1 (older files): a table of `num_scans + 1` offsets and one LZF
//! blob per scan, read as signed words (a positive word is an intensity, a negative one skips TOF indices). Written from the public descriptions of the format (OpenTIMS, opentimstdf, AlphaTims).

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;

fn guasto(m: &str) -> Error {
    Error::new("err.dam.unreadable").with("message", format!("tdf_bin: {m}"))
}

/// The peaks of a frame, in scan order: scan index (0-based), TOF index, intensity.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Picchi {
    pub scan: Vec<u32>,
    pub tof: Vec<u32>,
    pub int: Vec<u32>,
}

impl Picchi {
    pub fn len(&self) -> usize {
        self.tof.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tof.is_empty()
    }
    fn push(&mut self, scan: u32, tof: u32, int: u32) {
        self.scan.push(scan);
        self.tof.push(tof);
        self.int.push(int);
    }
}

fn u32_le(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// Reads the block at `offset` of a `tdf_bin` of `dim` bytes: the whole block, header included.
pub fn leggi_blocco(d: &mut dyn ReadAt, dim: u64, offset: u64, limiti: &Limiti) -> Result<Vec<u8>> {
    let mut h = [0u8; 8];
    if offset.checked_add(8).is_none_or(|e| e > dim) {
        return Err(guasto("block past the end of the file"));
    }
    d.read_at(offset, &mut h)?;
    let len = u64::from(u32_le(&h, 0).unwrap_or(0));
    if len < 8 {
        return Ok(h.to_vec());
    }
    if offset + len > dim || len > limiti.byte {
        return Err(guasto("block length"));
    }
    let mut b = vec![0u8; len as usize];
    d.read_at(offset, &mut b)?;
    Ok(b)
}

/// The peaks of a block. `num_scans` and `num_peaks` come from the `Frames` row and are checked against the block and the limits.
pub fn decodifica(
    blocco: &[u8],
    tipo: u32,
    num_scans: u64,
    num_peaks: u64,
    limiti: &Limiti,
) -> Result<Picchi> {
    limiti.voci_ok(num_scans)?;
    limiti.voci_ok(num_peaks)?;
    if blocco.len() <= 8 {
        return if num_peaks == 0 {
            Ok(Picchi::default())
        } else {
            Err(guasto("empty block with peaks"))
        };
    }
    let ns = u64::from(u32_le(blocco, 4).unwrap_or(u32::MAX));
    limiti.voci_ok(ns)?;
    // type 2 repeats the scan count of the frame table; type 1 lists the whole ramp
    if tipo == 2 && ns != num_scans {
        return Err(guasto("scan count differs from the frame table"));
    }
    let num_scans = ns;
    match tipo {
        2 => decodifica2(&blocco[8..], num_scans as usize, num_peaks as usize, limiti),
        1 => decodifica1(blocco, num_scans as usize, num_peaks as usize, limiti),
        _ => Err(guasto("unknown compression type")),
    }
}

/// The window a zstd frame announces for its decoder (None: not a zstd frame). A hostile header cannot make the decoder allocate more than the limit.
fn finestra_zstd(z: &[u8]) -> Option<u64> {
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
pub fn zstd(z: &[u8], attesi: usize, limiti: &Limiti) -> Result<Vec<u8>> {
    use std::io::Read;
    if attesi as u64 > limiti.byte || finestra_zstd(z).is_none_or(|w| w > limiti.byte) {
        return Err(guasto("zstd frame"));
    }
    let mut dec = ruzstd::decoding::StreamingDecoder::new(z).map_err(|_| guasto("zstd"))?;
    let mut inner = Vec::with_capacity(attesi);
    (&mut dec)
        .take(attesi as u64 + 1)
        .read_to_end(&mut inner)
        .map_err(|_| guasto("zstd data"))?;
    if inner.len() != attesi {
        return Err(guasto("decompressed size"));
    }
    Ok(inner)
}

fn decodifica2(z: &[u8], ns: usize, np: usize, limiti: &Limiti) -> Result<Picchi> {
    let parole = ns
        .checked_add(np.checked_mul(2).ok_or_else(|| guasto("peaks"))?)
        .ok_or_else(|| guasto("peaks"))?;
    let attesi = parole.checked_mul(4).ok_or_else(|| guasto("peaks"))?;
    let inner = zstd(z, attesi, limiti)?;
    let parola = |i: usize| {
        u32::from(inner[i])
            | u32::from(inner[parole + i]) << 8
            | u32::from(inner[2 * parole + i]) << 16
            | u32::from(inner[3 * parole + i]) << 24
    };
    let mut out = Picchi::default();
    let mut letti = 0usize;
    for scan in 0..ns {
        let n = if scan + 1 < ns {
            (parola(scan + 1) / 2) as usize
        } else {
            np.checked_sub(out.len())
                .ok_or_else(|| guasto("peak counts"))?
        };
        let fine = letti
            .checked_add(n.checked_mul(2).ok_or_else(|| guasto("peak counts"))?)
            .filter(|&f| f <= parole - ns)
            .ok_or_else(|| guasto("peak counts"))?;
        let mut tof = u32::MAX;
        while letti < fine {
            tof = tof.wrapping_add(parola(ns + letti));
            out.push(scan as u32, tof, parola(ns + letti + 1));
            letti += 2;
        }
    }
    if out.len() != np {
        return Err(guasto("peak total"));
    }
    Ok(out)
}

fn decodifica1(b: &[u8], ns: usize, np: usize, limiti: &Limiti) -> Result<Picchi> {
    let tab = (ns + 1) * 4;
    let inizio = 8 + tab;
    if inizio > b.len() {
        return Err(guasto("offset table"));
    }
    let mut off = Vec::with_capacity(ns + 1);
    for i in 0..=ns {
        let o = u32_le(b, 8 + 4 * i).ok_or_else(|| guasto("offset table"))? as usize;
        if o < inizio || o > b.len() || off.last().is_some_and(|&p| o < p) {
            return Err(guasto("scan offset"));
        }
        off.push(o);
    }
    let mut out = Picchi::default();
    let mut buf = Vec::new();
    for scan in 0..ns {
        if off[scan] == off[scan + 1] {
            continue;
        }
        buf.clear();
        lzf(&b[off[scan]..off[scan + 1]], &mut buf, limiti.byte as usize)?;
        if buf.len() % 4 != 0 {
            return Err(guasto("scan size"));
        }
        // a negative word skips that many TOF bins; a non-negative one is an intensity at the current bin, and the bin moves on
        let mut tof = 0u32;
        for w in buf.chunks_exact(4) {
            let v = i32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            if v >= 0 {
                if out.len() >= np {
                    return Err(guasto("more peaks than the frame table"));
                }
                out.push(scan as u32, tof, v as u32);
                tof = tof.wrapping_add(1);
            } else {
                tof = tof.wrapping_add(v.unsigned_abs());
            }
        }
    }
    if out.len() != np {
        return Err(guasto("peak total"));
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
            let s = z.get(i..i + n).ok_or_else(|| guasto("lzf literal"))?;
            if out.len() + n > max {
                return Err(guasto("lzf size"));
            }
            out.extend_from_slice(s);
            i += n;
        } else {
            let mut n = c >> 5;
            if n == 7 {
                n += usize::from(*z.get(i).ok_or_else(|| guasto("lzf length"))?);
                i += 1;
            }
            n += 2;
            let dist =
                ((c & 0x1f) << 8) | usize::from(*z.get(i).ok_or_else(|| guasto("lzf distance"))?);
            i += 1;
            let da = out
                .len()
                .checked_sub(dist + 1)
                .ok_or_else(|| guasto("lzf reference"))?;
            if out.len() + n > max {
                return Err(guasto("lzf size"));
            }
            for k in 0..n {
                out.push(out[da + k]);
            }
        }
    }
    Ok(())
}
