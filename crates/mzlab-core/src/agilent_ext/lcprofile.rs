//! Profile decoding of the LC/MS blocks (run-length and LZF streams), see `lcms.rs`.

use crate::error::{Error, Result};

pub(super) fn bad(detail: String) -> Error {
    Error::new("err.file.agilent").with("detail", detail)
}

/// Run-length stream of the profile: `[.., i32 −first index, ..]` then integers of the current width; a negative value `v` skips `!v >> 2` points and sets the
/// width to `1 << (!v & 3)` bytes, a non-negative one is the intensity of the next point.
pub(super) fn run_length(s: &[u8], n: usize) -> Result<Vec<i32>> {
    let first = s
        .get(4..8)
        .map(|b| -(i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as i64))
        .filter(|f| *f >= 0)
        .ok_or_else(|| bad("profile: no start index".into()))? as usize;
    let mut out = vec![0i32; n];
    let (mut pos, mut w, mut i) = (first, 4usize, 8usize);
    while i < s.len() {
        let Some(raw) = s.get(i..i + w) else { break };
        i += w;
        let v = match w {
            1 => raw[0] as i8 as i64,
            2 => i16::from_le_bytes([raw[0], raw[1]]) as i64,
            4 => i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as i64,
            _ => i64::from_le_bytes(raw.try_into().unwrap_or([0; 8])),
        };
        if v >= 0 {
            *out.get_mut(pos)
                .ok_or_else(|| bad("profile: a point past the end".into()))? =
                v.min(i32::MAX as i64) as i32;
            pos += 1;
        } else {
            let u = !v as u64;
            pos = pos.saturating_add((u >> 2) as usize);
            w = 1 << (u & 3);
        }
    }
    Ok(out)
}

/// LZF (liblzf format) of `s`, expected to give `size` bytes.
pub(super) fn lzf(s: &[u8], size: usize) -> Result<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let mut i = 0;
    let end = || bad("profile: the LZF stream is cut".into());
    while i < s.len() && out.len() < size {
        let c = s[i] as usize;
        i += 1;
        if c < 32 {
            let run = c + 1;
            out.extend_from_slice(s.get(i..i + run).ok_or_else(end)?);
            i += run;
        } else {
            let mut len = c >> 5;
            if len == 7 {
                len += *s.get(i).ok_or_else(end)? as usize;
                i += 1;
            }
            let back = ((c & 31) << 8) + *s.get(i).ok_or_else(end)? as usize + 1;
            i += 1;
            let from = out
                .len()
                .checked_sub(back)
                .ok_or_else(|| bad("profile: the LZF reference is before the start".into()))?;
            for k in 0..len + 2 {
                let b = out[from + k];
                out.push(b);
            }
        }
    }
    if out.len() != size {
        return Err(bad(format!(
            "profile: {} bytes unpacked, {size} expected",
            out.len()
        )));
    }
    Ok(out)
}

pub(super) fn f64_at(b: &[u8], at: usize) -> f64 {
    f64::from_le_bytes(b[at..at + 8].try_into().unwrap_or([0; 8]))
}
