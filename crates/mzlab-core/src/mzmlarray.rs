//! Decoding of one binary data array of an mzML: base64, zlib (one `Decompress` and one buffer reused by the whole parse) and the number type.
//! Errors are those of the reader before this module: `err.file.base64`, `err.file.zlib` (with the `flate2::read` messages), `error.memoryBudget`.

use std::io::ErrorKind;

use base64::Engine;
use flate2::{Decompress, FlushDecompress, Status};

use crate::error::{Error, Result};
use crate::numarray::Num;

/// What a caller keeps of a decoded array: the other width is never allocated.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Want {
    F64,
    F32,
    Both,
}

/// The inflater of the parse: one `Decompress` and one output buffer reused by every array. The buffer is zeroed only when it grows (`decompress_vec`
/// would zero all its spare room on every call).
pub(crate) struct Inflate {
    dec: Decompress,
    buf: Vec<u8>,
    /// Bytes of `buf` that hold the last array.
    n: usize,
}

impl Inflate {
    pub(crate) fn new() -> Self {
        Inflate {
            dec: Decompress::new(true),
            buf: Vec::new(),
            n: 0,
        }
    }

    /// What the last `run` inflated.
    fn out(&self) -> &[u8] {
        &self.buf[..self.n]
    }

    /// Inflates the zlib stream into `self.buf` (at most `limit` bytes), with the errors of `flate2::read::ZlibDecoder`.
    fn run(&mut self, raw: &[u8], limit: usize) -> Result<()> {
        self.dec.reset(true);
        self.n = 0;
        // never more than `limit + 1` bytes: one over the limit is the proof of a bomb
        let max = limit.saturating_add(1);
        let first = limit
            .min(raw.len().saturating_mul(1032))
            .saturating_add(1)
            .min(max);
        self.grow(first)?;
        let mut pos = 0usize;
        loop {
            if self.n == self.buf.len() {
                let more = self.buf.len().max(1 << 16).min(max - self.n);
                self.grow(self.n + more)?;
            }
            let (before_in, before_out) = (self.dec.total_in(), self.dec.total_out());
            let st = self
                .dec
                .decompress(
                    raw.get(pos..).unwrap_or_default(),
                    &mut self.buf[self.n..],
                    FlushDecompress::None,
                )
                .map_err(|_| zlib_error(ErrorKind::InvalidInput, "corrupt deflate stream"))?;
            pos += usize::try_from(self.dec.total_in() - before_in).unwrap_or(raw.len());
            let made = usize::try_from(self.dec.total_out() - before_out).unwrap_or(0);
            self.n += made;
            if self.n > limit {
                return Err(Error::memory_budget());
            }
            match st {
                Status::StreamEnd => return Ok(()),
                _ if made == 0 && self.n < self.buf.len() => {
                    return Err(zlib_error(
                        ErrorKind::UnexpectedEof,
                        "incomplete deflate stream",
                    ))
                }
                _ => {}
            }
        }
    }

    /// `buf` at least `len` bytes long (new bytes zeroed).
    fn grow(&mut self, len: usize) -> Result<()> {
        if self.buf.len() < len {
            self.buf
                .try_reserve_exact(len - self.buf.len())
                .map_err(|_| Error::memory_budget())?;
            self.buf.resize(len, 0);
        }
        Ok(())
    }
}

/// The little-endian values of `W` bytes of `bytes` (a partial last value is dropped), as `f64` and/or narrowed to `f32` as `want` says.
#[inline(always)]
fn convert<const W: usize>(
    bytes: &[u8],
    value: impl Fn([u8; W]) -> f64,
    want: Want,
    f64s: &mut Vec<f64>,
    f32s: &mut Vec<f32>,
) {
    let (chunks, _) = bytes.as_chunks::<W>();
    match want {
        Want::F64 => f64s.extend(chunks.iter().map(|c| value(*c))),
        Want::F32 => f32s.extend(chunks.iter().map(|c| narrow(value(*c)))),
        Want::Both => {
            for c in chunks {
                let v = value(*c);
                f64s.push(v);
                f32s.push(narrow(v));
            }
        }
    }
}

/// The 32-bit intensity of a value (exact for a value read from a 32-bit array).
fn narrow(v: f64) -> f32 {
    v as f32
}

fn zlib_error(kind: ErrorKind, msg: &str) -> Error {
    Error::new("err.file.zlib").with("detail", std::io::Error::new(kind, msg.to_string()))
}

/// The bytes of a base64 text, `None` when it holds nothing but white space. Base64 text rarely holds white space, and any white space makes the
/// plain decode fail: the text is decoded where it is and filtered only after a failure.
fn unbase64(text: &str) -> Result<Option<Vec<u8>>> {
    if text.is_empty() {
        return Ok(None);
    }
    let b64 = &base64::engine::general_purpose::STANDARD;
    match b64.decode(text.as_bytes()) {
        Ok(raw) => Ok(Some(raw)),
        Err(e) => {
            if !text
                .bytes()
                .any(|b| matches!(b, b'\t'..=b'\r' | b' ' | 0x80..))
            {
                return Err(Error::new("err.file.base64").with("detail", e));
            }
            let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            if clean.is_empty() {
                return Ok(None);
            }
            b64.decode(clean.as_bytes())
                .map(Some)
                .map_err(|e| Error::new("err.file.base64").with("detail", e))
        }
    }
}

pub(crate) fn decode_array(
    num: Num,
    zlib: bool,
    text: &str,
    expected: usize,
    inf: &mut Inflate,
    want: Want,
) -> Result<(Vec<f64>, Vec<f32>)> {
    let Some(raw) = unbase64(text)? else {
        return Ok((Vec::new(), Vec::new()));
    };
    let bytes: &[u8] = if zlib {
        // an array inflates to what the spectrum declares (64 MB when it declares nothing): a zlib stream may claim far more than the file holds
        let limit = match expected.saturating_mul(num.width()) {
            0 => 64 << 20,
            n => n,
        };
        inf.run(&raw, limit)?;
        inf.out()
    } else {
        &raw
    };
    let n = bytes.len() / num.width();
    let mut f64s: Vec<f64> = Vec::new();
    let mut f32s: Vec<f32> = Vec::new();
    if want != Want::F32 {
        f64s.try_reserve_exact(n)
            .map_err(|_| Error::memory_budget())?;
    }
    if want != Want::F64 {
        f32s.try_reserve_exact(n)
            .map_err(|_| Error::memory_budget())?;
    }
    // one loop per number type, so the type is not matched again for every value
    match num {
        Num::F32 => convert(
            bytes,
            |c: [u8; 4]| f32::from_le_bytes(c) as f64,
            want,
            &mut f64s,
            &mut f32s,
        ),
        Num::I32 => convert(
            bytes,
            |c: [u8; 4]| i32::from_le_bytes(c) as f64,
            want,
            &mut f64s,
            &mut f32s,
        ),
        Num::F64 => convert(bytes, f64::from_le_bytes, want, &mut f64s, &mut f32s),
        Num::I64 => convert(
            bytes,
            |c: [u8; 8]| i64::from_le_bytes(c) as f64,
            want,
            &mut f64s,
            &mut f32s,
        ),
    }
    Ok((f64s, f32s))
}

/// A chromatogram whose time or intensity holds a NaN or infinite point is a damaged file (`err.file.nonFiniteTrace`, like the readers' guard
/// `mzlab_lettori::finiti`): the parser (`mzml::finish_chromatogram`) and the page's chromatogram reader (`Mzml::chromatogram`) both ask here.
pub(crate) fn punti_finiti(id: &str, t: &[f64], y: &[f64]) -> Result<()> {
    let guasto = t
        .iter()
        .map(|v| ("time", *v))
        .chain(y.iter().map(|v| ("intensity", *v)));
    match guasto.into_iter().find(|(_, v)| !v.is_finite()) {
        Some((cosa, v)) => Err(Error::new("err.file.nonFiniteTrace")
            .with("trace", id)
            .with("what", cosa)
            .with("value", v)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests_decode;
