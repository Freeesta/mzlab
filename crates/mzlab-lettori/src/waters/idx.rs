//! `_FUNCnnn.IDX`: one record per scan, in two layouts. Variant A (22 bytes) holds the offset in the DAT as `u32`; variant B (30 bytes) holds it as `u64` at byte 22. In both, bytes 4-7
//! hold the number of records of the scan in their low 24 bits, then the TIC (`f32`) and the retention time in minutes (`f32`).

use super::cartella::{errore, f32_at, u32_at};
use mzlab_core::error::Result;
use mzlab_core::lettori::Limiti;

const A: usize = 22;
const B: usize = 30;
/// The top byte of the word at byte 4 in every record of the families seen.
const MARCHI: [u8; 3] = [0x18, 0x4c, 0x08];

#[derive(Debug, Clone, Copy)]
pub(super) struct Riga {
    pub(super) offset: u64,
    /// Records of the scan: points of a scan, channels of a SIR or MRM scan.
    pub(super) conta: u32,
    pub(super) tic: f32,
    /// Minutes.
    pub(super) rt: f32,
}

/// Whether `b` is variant A: only when its size fits and, if it fits both, when every record carries a known mark and the offsets never go back.
fn variante_a(b: &[u8]) -> bool {
    match (b.len().is_multiple_of(A), b.len().is_multiple_of(B)) {
        (true, false) => true,
        (false, _) => false,
        (true, true) => {
            let mut prima = 0u32;
            b.chunks_exact(A).all(|r| {
                let o = u32_at(r, 0);
                let ok = MARCHI.contains(&r[7]) && o >= prima;
                prima = o;
                ok
            })
        }
    }
}

pub(super) fn leggi(b: &[u8], limiti: &Limiti) -> Result<Vec<Riga>> {
    if b.is_empty() {
        return Ok(Vec::new());
    }
    if !b.len().is_multiple_of(A) && !b.len().is_multiple_of(B) {
        return Err(errore("IDX size"));
    }
    let a = variante_a(b);
    let passo = if a { A } else { B };
    limiti.voci_ok((b.len() / passo) as u64)?;
    Ok(b.chunks_exact(passo)
        .map(|r| Riga {
            offset: if a {
                u32_at(r, 0) as u64
            } else {
                u32_at(r, 22) as u64 | (u32_at(r, 26) as u64) << 32
            },
            conta: u32_at(r, 4) & 0x00FF_FFFF,
            tic: f32_at(r, 8),
            rt: f32_at(r, 12),
        })
        .collect())
}

/// Bytes per record of a function: the first scan whose extent (up to the offset of the next scan, or up to the end of the DAT for the last) is a whole number of records.
pub(super) fn larghezza(righe: &[Riga], dat_len: u64) -> Option<u64> {
    (0..righe.len()).find_map(|k| {
        let r = righe[k];
        let fine = righe.get(k + 1).map_or(dat_len, |n| n.offset);
        let d = fine.checked_sub(r.offset)?;
        let c = r.conta as u64;
        (c > 0 && d > 0 && d.is_multiple_of(c)).then_some(d / c)
    })
}

/// (offset, length) of scan `k` given the record width, inside a DAT of `dat_len` bytes.
pub(super) fn estensione(
    righe: &[Riga],
    k: usize,
    larghezza: u64,
    dat_len: u64,
) -> Result<(u64, u64)> {
    let r = righe.get(k).ok_or_else(|| errore("scan out of range"))?;
    let len = (r.conta as u64)
        .checked_mul(larghezza)
        .ok_or_else(|| errore("scan size"))?;
    match r.offset.checked_add(len) {
        Some(fine) if fine <= dat_len => Ok((r.offset, len)),
        _ => Err(errore("scan past the end of the DAT")),
    }
}
