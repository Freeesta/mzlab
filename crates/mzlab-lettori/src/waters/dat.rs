//! The records of `_FUNCnnn.DAT` of a scanning function (the MRM and SIR channels are in `mrm.rs`). Three encodings, told apart by the width of the record, which follows from the
//! IDX (bytes of the scan / records of the scan): 6 bytes (ion counts), 8 bytes and 12 bytes (the same floating intensity word; the 12-byte record has 4 more bytes that are not read). The m/z of a
//! record is a small float whose exponent sits in the top bits; the calibration polynomial of `_HEADER.TXT` is applied to its square root (proportional to the flight time).

use super::cartella::{errore, u16_at, u32_at};
use mzlab_core::error::Result;

/// The `Cal Function N` polynomial of `_HEADER.TXT`. `T0` is no correction.
#[derive(Debug, Clone, Default)]
pub(super) struct Taratura {
    pub(super) coef: Vec<f64>,
    pub(super) t1: bool,
}

impl Taratura {
    /// The `c0,c1,...,ck,T1` of a `Cal Function` line; `None` when the text is not that.
    pub(super) fn da_testo(v: &str) -> Option<Taratura> {
        let mut parti: Vec<&str> = v.split(',').map(str::trim).collect();
        let tipo = parti.pop()?;
        if tipo != "T0" && tipo != "T1" {
            return None;
        }
        let coef: Option<Vec<f64>> = parti.iter().map(|p| p.parse().ok()).collect();
        Some(Taratura {
            coef: coef?,
            t1: tipo == "T1",
        })
    }

    pub(super) fn applica(&self, mz: f64) -> f64 {
        if !self.t1 || self.coef.is_empty() || mz <= 0.0 {
            return mz;
        }
        let t = mz.sqrt();
        let v = self.coef.iter().rev().fold(0.0, |acc, &c| acc * t + c);
        v * v
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Codifica {
    /// 6 bytes: ion count (`u16`) and the m/z word.
    A,
    /// 8 bytes: intensity word (21-bit mantissa, 5-bit exponent) and the m/z word.
    D,
    /// 12 bytes: as the 8-byte record, then 4 bytes that are not read.
    E,
}

impl Codifica {
    pub(super) fn da_larghezza(w: u64) -> Option<Codifica> {
        match w {
            6 => Some(Codifica::A),
            8 => Some(Codifica::D),
            12 => Some(Codifica::E),
            _ => None,
        }
    }

    pub(super) fn larghezza(self) -> usize {
        match self {
            Codifica::A => 6,
            Codifica::D => 8,
            Codifica::E => 12,
        }
    }

    /// Whether the points are a profile (the 12-byte records of a centroided acquisition are peaks).
    pub(super) fn profilo(self) -> bool {
        self != Codifica::E
    }
}

/// m/z of the word of the encodings D and E: bit 26 is the leading one of a 27-bit mantissa, the top 5 bits the exponent.
fn mz_d(u: u32) -> Result<f64> {
    if u & (1 << 26) == 0 {
        return Err(errore("m/z word without the leading bit"));
    }
    Ok(f64::from(u & 0x07FF_FFFF) / f64::from(1u32 << 27) * 2f64.powi((u >> 27) as i32))
}

/// m/z of the 4 bytes of the encoding A (from byte 2): high nibble the exponent, then a normalised 24-bit mantissa.
fn mz_a(r: &[u8]) -> Result<f64> {
    let m = u32::from_le_bytes([r[3], r[4], r[5], 0]);
    if r[2] & 0x0F != 0 || m & 0x80_0000 == 0 {
        return Err(errore("m/z word of the 6-byte record"));
    }
    Ok(f64::from(m) * 2f64.powi(i32::from(r[2] >> 4) - 24))
}

/// The points of one scan: calibrated m/z and intensity. Records without intensity are left out.
pub(super) fn decodifica(c: Codifica, b: &[u8], t: &Taratura) -> Result<(Vec<f64>, Vec<f32>)> {
    let w = c.larghezza();
    if !b.len().is_multiple_of(w) {
        return Err(errore("scan size is no multiple of the record"));
    }
    let (mut mz, mut it) = (
        Vec::with_capacity(b.len() / w),
        Vec::with_capacity(b.len() / w),
    );
    for r in b.chunks_exact(w) {
        let (x, i) = match c {
            Codifica::A => {
                let n = u16_at(r, 0);
                if n == 0 {
                    continue;
                }
                (mz_a(r)?, f32::from(n))
            }
            Codifica::D | Codifica::E => {
                let word = u32_at(r, 0);
                let m = word & 0x001F_FFFF;
                if m == 0 {
                    continue;
                }
                if m & (1 << 20) == 0 || word & (1 << 21) != 0 {
                    return Err(errore("intensity word of the record"));
                }
                let e = ((word >> 22) & 0x1F) as i32;
                (
                    mz_d(u32_at(r, 4))?,
                    (f64::from(m) * 2f64.powi(e - 21)) as f32,
                )
            }
        };
        mz.push(t.applica(x));
        it.push(i);
    }
    Ok((mz, it))
}
