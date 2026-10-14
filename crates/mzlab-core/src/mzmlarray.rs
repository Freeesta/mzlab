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

/// The inflater of the parse: one `Decompress` and one output buffer reused by every array.
pub(crate) struct Inflate {
    dec: Decompress,
    out: Vec<u8>,
}

impl Inflate {
    pub(crate) fn new() -> Self {
        Inflate {
            dec: Decompress::new(true),
            out: Vec::new(),
        }
    }

    /// Inflates the zlib stream into `self.out` (at most `limit` bytes), with the errors of `flate2::read::ZlibDecoder`.
    fn run(&mut self, raw: &[u8], limit: usize) -> Result<()> {
        self.dec.reset(true);
        self.out.clear();
        self.out
            .try_reserve(limit.min(raw.len().saturating_mul(1032)))
            .map_err(|_| Error::memory_budget())?;
        let mut pos = 0usize;
        loop {
            // never more than `limit + 1` bytes: one over the limit is the proof of a bomb
            let room = limit.saturating_add(1) - self.out.len();
            if self.out.capacity() == self.out.len() {
                let more = self.out.capacity().max(1 << 16).min(room);
                self.out
                    .try_reserve(more)
                    .map_err(|_| Error::memory_budget())?;
            }
            let (before_in, before_out) = (self.dec.total_in(), self.dec.total_out());
            let st = self
                .dec
                .decompress_vec(
                    raw.get(pos..).unwrap_or_default(),
                    &mut self.out,
                    FlushDecompress::None,
                )
                .map_err(|_| zlib_error(ErrorKind::InvalidInput, "corrupt deflate stream"))?;
            pos += usize::try_from(self.dec.total_in() - before_in).unwrap_or(raw.len());
            let made = self.dec.total_out() - before_out;
            if self.out.len() > limit {
                return Err(Error::memory_budget());
            }
            match st {
                Status::StreamEnd => return Ok(()),
                _ if made == 0 && self.out.capacity() > self.out.len() => {
                    return Err(zlib_error(
                        ErrorKind::UnexpectedEof,
                        "incomplete deflate stream",
                    ))
                }
                _ => {}
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

pub(crate) fn decode_array(
    num: Num,
    zlib: bool,
    text: &str,
    expected: usize,
    inf: &mut Inflate,
    want: Want,
) -> Result<(Vec<f64>, Vec<f32>)> {
    // Base64 text rarely holds white space: decode it where it is, filter only when a byte could be white space.
    let spaced = text
        .bytes()
        .any(|b| matches!(b, b'\t'..=b'\r' | b' ' | 0x80..));
    let clean: String;
    let bytes_in: &[u8] = if spaced {
        clean = text.chars().filter(|c| !c.is_whitespace()).collect();
        clean.as_bytes()
    } else {
        text.as_bytes()
    };
    if bytes_in.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let raw = base64::engine::general_purpose::STANDARD
        .decode(bytes_in)
        .map_err(|e| Error::new("err.file.base64").with("detail", e))?;
    let bytes: &[u8] = if zlib {
        // an array inflates to what the spectrum declares (64 MB when it declares nothing): a zlib stream may claim far more than the file holds
        let limit = match expected.saturating_mul(num.width()) {
            0 => 64 << 20,
            n => n,
        };
        inf.run(&raw, limit)?;
        &inf.out
    } else {
        &raw
    };
    let width = num.width();
    let n = bytes.len() / width;
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
    let chunks = bytes.chunks_exact(width);
    match (num, want) {
        (Num::F32, Want::F64) => f64s.extend(chunks.map(|c| num.value(c))),
        (_, Want::F64) => f64s.extend(chunks.map(|c| num.value(c))),
        (_, Want::F32) => f32s.extend(chunks.map(|c| narrow(num.value(c)))),
        (_, Want::Both) => {
            for c in chunks {
                let v = num.value(c);
                f64s.push(v);
                f32s.push(narrow(v));
            }
        }
    }
    Ok((f64s, f32s))
}
