//! MS-Numpress (Teleman et al., Mol Cell Proteomics 2014; the reference is the public C++ of the `ms-numpress` project, whose format is written down in its documentation):
//! the three codecs an mzML array may carry, linear prediction (MS:1002312), positive integer (MS:1002313) and short logged float (MS:1002314), alone or followed by zlib
//! (MS:1002746, MS:1002747, MS:1002748). Served only by the readers modules (feature `estesi`): the engine of the page keeps the error `err.file.numpress`.
//! Every count that comes from the file is bounded by `max`, the number of values the spectrum declares: a hostile array is an error with a key, never an allocation.
//! The encoders follow the reference byte for byte; `mzmlwrite` writes with the `_into` forms (linear prediction and short logged float), the others serve the tests.

use crate::error::{Error, Result};

/// The codec of a numpress array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Np {
    Linear,
    Pic,
    Slof,
}

/// The codec an accession names and whether zlib follows it: `Some((codec, zlib))` for MS:1002312/13/14 and MS:1002746/47/48.
pub fn of_accession(acc: &str) -> Option<(Np, bool)> {
    Some(match acc {
        "MS:1002312" => (Np::Linear, false),
        "MS:1002313" => (Np::Pic, false),
        "MS:1002314" => (Np::Slof, false),
        "MS:1002746" => (Np::Linear, true),
        "MS:1002747" => (Np::Pic, true),
        "MS:1002748" => (Np::Slof, true),
        _ => return None,
    })
}

/// One array as the mzML writes it: base64, zlib when `zlib`, then the codec (at most `expected` values; 8 M when `expected` is 0).
pub fn decode_text(np: Np, zlib: bool, text: &str, expected: usize) -> Result<Vec<f64>> {
    use base64::Engine;
    use std::io::Read;
    let clean: Vec<u8> = text.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let raw = base64::engine::general_purpose::STANDARD
        .decode(&clean)
        .map_err(|e| Error::new("err.file.base64").with("detail", e))?;
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    if !zlib {
        return decode(np, &raw, expected);
    }
    let limit = max_bytes(if expected == 0 { 8 << 20 } else { expected }) as u64;
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(&raw[..])
        .take(limit + 1)
        .read_to_end(&mut out)
        .map_err(|e| Error::new("err.file.zlib").with("detail", e))?;
    if out.len() as u64 > limit {
        return Err(Error::memory_budget());
    }
    decode(np, &out, expected)
}

/// The base64 of little-endian `f64` values (a plain 64-bit mzML array).
pub fn plain_base64(values: &[f64]) -> String {
    use base64::Engine;
    let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn bad(why: &str) -> Error {
    Error::new("err.file.numpress").with("detail", why)
}

/// The most bytes a numpress stream of `n` values can have (before zlib): 4.5 bytes per value for the integer codecs, 2 for slof, plus the 16 of the headers.
pub fn max_bytes(n: usize) -> usize {
    n.saturating_mul(5).saturating_add(16)
}

/// The values of a numpress stream: `expected` is the number the array declares (0: not said, then at most 8 M), and more than that is an error.
/// The reference encoder ends an odd number of half bytes with a padding half byte 0x8 and its decoder stops at a lone 0x8, which is also the head of a zero (a zero difference
/// at the very end, as in a perfectly linear axis): when the declared length is one more than the values read, that lone half byte is the last value, a zero.
pub fn decode(np: Np, data: &[u8], expected: usize) -> Result<Vec<f64>> {
    let max = if expected == 0 { 8 << 20 } else { expected };
    match np {
        Np::Linear => linear(data, max, expected),
        Np::Pic => pic(data, max, expected),
        Np::Slof => slof(data, max),
    }
}

/// The fixed point of the first 8 bytes (a big-endian `f64`).
fn fixed_point(data: &[u8]) -> Result<f64> {
    let b: [u8; 8] = data
        .get(..8)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| bad("fixed point"))?;
    let f = f64::from_be_bytes(b);
    if f.is_finite() && f != 0.0 {
        Ok(f)
    } else {
        Err(bad("fixed point"))
    }
}

fn push(out: &mut Vec<f64>, v: f64, max: usize) -> Result<()> {
    if out.len() >= max {
        return Err(bad("more values than declared"));
    }
    out.push(v);
    Ok(())
}

/// The stream of half bytes of a numpress integer sequence: high half first.
struct Nibbles<'a> {
    data: &'a [u8],
    at: usize,
}

impl Nibbles<'_> {
    fn left(&self) -> usize {
        self.data.len() * 2 - self.at
    }
    fn next(&mut self) -> Result<u32> {
        let b = *self
            .data
            .get(self.at / 2)
            .ok_or_else(|| bad("end of data"))?;
        let h = if self.at.is_multiple_of(2) {
            b >> 4
        } else {
            b & 0xf
        };
        self.at += 1;
        Ok(u32::from(h))
    }
    /// One integer: a head half byte (0..=8: that many zero halves in front; 9..=15: that many minus 8 halves of ones), then the others, least significant first.
    fn int(&mut self) -> Result<u32> {
        let head = self.next()?;
        let (n, mut x) = if head <= 8 {
            (head, 0u32)
        } else {
            let n = head - 8;
            (n, if n >= 8 { !0u32 } else { !0u32 << (32 - 4 * n) })
        };
        if n >= 8 {
            return Ok(x);
        }
        if self.left() < (8 - n) as usize {
            return Err(bad("end of data"));
        }
        for i in n..8 {
            x |= self.next()? << (4 * (i - n));
        }
        Ok(x)
    }
    /// True when the half byte left is the head of a zero (0x8).
    fn left_is_zero_head(&self) -> bool {
        self.data.last().is_some_and(|b| b & 0xf == 0x8)
    }
    /// True when only the padding half byte is left (the reference writes 0x8; a 0 is read the same, it cannot start an integer).
    fn only_padding(&self) -> bool {
        if self.left() != 1 {
            return false;
        }
        let b = self.data[self.data.len() - 1] & 0xf;
        b == 0x8 || b == 0
    }
}

fn linear(data: &[u8], max: usize, expected: usize) -> Result<Vec<f64>> {
    let mut out = Vec::new();
    if data.len() < 8 {
        return Err(bad("short"));
    }
    let fp = fixed_point(data)?;
    let rest = &data[8..];
    if rest.is_empty() {
        return Ok(out);
    }
    let word = |i: usize| {
        rest.get(i..i + 4)
            .map(|b| i64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])))
    };
    let Some(a) = word(0) else {
        return Err(bad("short"));
    };
    push(&mut out, a as f64 / fp, max)?;
    if rest.len() == 4 {
        return Ok(out);
    }
    let Some(b) = word(4) else {
        return Err(bad("short"));
    };
    push(&mut out, b as f64 / fp, max)?;
    let (mut p0, mut p1) = (a, b);
    let mut n = Nibbles {
        data: &rest[8..],
        at: 0,
    };
    while n.left() > 0 {
        if n.only_padding() {
            if expected == out.len() + 1 && n.left_is_zero_head() {
                push(&mut out, (p1 + (p1 - p0)) as f64 / fp, max)?;
            }
            break;
        }
        let diff = i64::from(n.int()? as i32);
        let y = p1 + (p1 - p0) + diff;
        push(&mut out, y as f64 / fp, max)?;
        (p0, p1) = (p1, y);
    }
    Ok(out)
}

fn pic(data: &[u8], max: usize, expected: usize) -> Result<Vec<f64>> {
    let mut out = Vec::new();
    let mut n = Nibbles { data, at: 0 };
    while n.left() > 0 {
        if n.only_padding() {
            if expected == out.len() + 1 && n.left_is_zero_head() {
                push(&mut out, 0.0, max)?;
            }
            break;
        }
        let v = n.int()?;
        push(&mut out, f64::from(v), max)?;
    }
    Ok(out)
}

fn slof(data: &[u8], max: usize) -> Result<Vec<f64>> {
    let fp = fixed_point(data)?;
    let body = &data[8..];
    if !body.len().is_multiple_of(2) {
        return Err(bad("odd length"));
    }
    if body.len() / 2 > max {
        return Err(bad("more values than declared"));
    }
    Ok(body
        .chunks_exact(2)
        .map(|c| (f64::from(u16::from_le_bytes([c[0], c[1]])) / fp).exp() - 1.0)
        .collect())
}

// ---- encoders (the reference, for the tests; `mzmlwrite` writes with the `_into` forms) ----

/// The half bytes of an integer sequence, packed two to a byte (high first) straight into the output; `end` pads an odd number with the half byte 0x8.
struct Halves<'a> {
    out: &'a mut Vec<u8>,
    pending: Option<u8>,
}

impl Halves<'_> {
    fn push(&mut self, h: u8) {
        match self.pending.take() {
            Some(hi) => self.out.push((hi << 4) | h),
            None => self.pending = Some(h),
        }
    }
    fn end(self) {
        if let Some(hi) = self.pending {
            self.out.push((hi << 4) | 0x8);
        }
    }
}

fn put_int(x: u32, halves: &mut Halves) {
    let mask = 0xf000_0000u32;
    let init = x & mask;
    if init == 0 {
        let l = (0..8).find(|&i| x & (mask >> (4 * i)) != 0).unwrap_or(8);
        halves.push(l as u8);
        (l..8).for_each(|i| halves.push(((x >> (4 * (i - l))) & 0xf) as u8));
    } else if init == mask {
        let l = (0..8)
            .find(|&i| x & (mask >> (4 * i)) != mask >> (4 * i))
            .unwrap_or(7);
        halves.push((l + 8) as u8);
        (l..8).for_each(|i| halves.push(((x >> (4 * (i - l))) & 0xf) as u8));
    } else {
        halves.push(0);
        (0..8).for_each(|i| halves.push(((x >> (4 * i)) & 0xf) as u8));
    }
}

/// The best fixed point for linear prediction of values up to `max` (the reference: the largest that keeps the integers in 31 bits).
pub fn linear_fixed_point(values: &[f64]) -> f64 {
    let max = values.iter().cloned().fold(1.0, f64::max);
    (f64::from(i32::MAX) / max)
        .floor()
        .max(1.0)
        .min(2f64.powi(30))
}

/// Linear prediction encoding with fixed point `fp`.
pub fn encode_linear(values: &[f64], fp: f64) -> Vec<u8> {
    let mut out = Vec::new();
    encode_linear_into(&mut out, values, fp);
    out
}

/// `encode_linear` appended to `out` (same bytes, no intermediate vectors).
pub fn encode_linear_into(out: &mut Vec<u8>, values: &[f64], fp: f64) {
    out.reserve(8 + 8 + values.len() * 2);
    out.extend_from_slice(&fp.to_be_bytes());
    let q = |v: f64| (v * fp + 0.5) as i64;
    for v in values.iter().take(2) {
        out.extend_from_slice(&(q(*v) as u32).to_le_bytes());
    }
    let mut halves = Halves { out, pending: None };
    if values.len() > 2 {
        let (mut a, mut b) = (q(values[0]), q(values[1]));
        for v in &values[2..] {
            let c = q(*v);
            put_int((c - (b + (b - a))) as i32 as u32, &mut halves);
            (a, b) = (b, c);
        }
    }
    halves.end();
}

/// Positive integer encoding (each value rounded to the nearest integer).
pub fn encode_pic(values: &[f64]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut halves = Halves {
        out: &mut out,
        pending: None,
    };
    for v in values {
        put_int((v + 0.5) as u32, &mut halves);
    }
    halves.end();
    out
}

/// Short logged float encoding with fixed point `fp`.
pub fn encode_slof(values: &[f64], fp: f64) -> Vec<u8> {
    let mut out = Vec::new();
    encode_slof_into(&mut out, values, fp);
    out
}

/// `encode_slof` appended to `out`.
pub fn encode_slof_into(out: &mut Vec<u8>, values: &[f64], fp: f64) {
    slof_into(out, values.iter().copied(), values.len(), fp);
}

/// `encode_slof_into` of 32-bit values (each widened as it is encoded, no copy of the array).
pub fn encode_slof_f32_into(out: &mut Vec<u8>, values: &[f32], fp: f64) {
    slof_into(out, values.iter().map(|&x| f64::from(x)), values.len(), fp);
}

fn slof_into(out: &mut Vec<u8>, values: impl Iterator<Item = f64>, n: usize, fp: f64) {
    out.reserve(8 + 2 * n);
    out.extend_from_slice(&fp.to_be_bytes());
    for v in values {
        let x = ((v + 1.0).ln() * fp + 0.5) as u16;
        out.extend_from_slice(&x.to_le_bytes());
    }
}

/// The fixed point of slof for values up to `max`: the largest that keeps `ln(max+1) * fp` inside 16 bits.
pub fn slof_fixed_point(values: &[f64]) -> f64 {
    slof_fp(values.iter().cloned().fold(0.0, f64::max))
}

/// `slof_fixed_point` of 32-bit values.
pub fn slof_fixed_point_f32(values: &[f32]) -> f64 {
    slof_fp(values.iter().map(|&x| f64::from(x)).fold(0.0, f64::max))
}

fn slof_fp(max: f64) -> f64 {
    (65535.0 / (max + 1.0).ln().max(1e-9))
        .floor()
        .clamp(1.0, 65535.0)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_into;
