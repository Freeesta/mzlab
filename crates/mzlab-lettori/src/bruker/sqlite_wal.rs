//! The write-ahead log of SQLite (`analysis.tdf-wal`).
use super::sqlite_min::{guasto, leggi, u32_be};
use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;
use std::collections::HashMap;

/// The checksum of SQLite's log over `b` (a multiple of 8 bytes), continuing from `(s0, s1)`; the words are read little-endian when `le`.
fn somma(b: &[u8], le: bool, mut s0: u32, mut s1: u32) -> (u32, u32) {
    for w in b.chunks_exact(8) {
        let p = |o: usize| {
            let x = [w[o], w[o + 1], w[o + 2], w[o + 3]];
            if le {
                u32::from_le_bytes(x)
            } else {
                u32::from_be_bytes(x)
            }
        };
        s0 = s0.wrapping_add(p(0)).wrapping_add(s1);
        s1 = s1.wrapping_add(p(4)).wrapping_add(s0);
    }
    (s0, s1)
}

/// The write-ahead log: (page size, offset of the latest committed frame of each page, pages of the database at the last commit). `None` for a log without frames; frames after the last
/// commit, with another salt or a wrong checksum are left out, as SQLite does.
pub(super) type Wal = (usize, HashMap<u32, u64>, u64);
pub(super) fn leggi_wal(w: &mut dyn ReadAt, dim: u64, limiti: &Limiti) -> Result<Option<Wal>> {
    if dim < 32 {
        return Ok(None);
    }
    let h = leggi(w, 0, 32)?;
    let magic = u32_be(&h, 0).unwrap_or(0);
    if magic != 0x377f_0682 && magic != 0x377f_0683 {
        return Ok(None);
    }
    let le = magic & 1 == 0;
    let (s0, s1) = somma(&h[..24], le, 0, 0);
    if (u32_be(&h, 24), u32_be(&h, 28)) != (Some(s0), Some(s1)) {
        return Ok(None);
    }
    let raw = u32_be(&h, 8).ok_or_else(|| guasto("log header"))? as usize;
    let pagina = if raw == 1 { 65536 } else { raw };
    if !(512..=65536).contains(&pagina) || !pagina.is_power_of_two() {
        return Err(guasto("log page size"));
    }
    let passo = 24 + pagina as u64;
    limiti.voci_ok((dim - 32) / passo)?;
    let (mut c0, mut c1) = (s0, s1);
    let mut mappa: HashMap<u32, u64> = HashMap::new();
    let mut in_attesa: Vec<(u32, u64)> = Vec::new();
    let mut pagine = 0u64;
    let mut off = 32u64;
    while off + passo <= dim {
        let f = leggi(w, off, passo as usize)?;
        if f[8..16] != h[16..24] {
            break;
        }
        let (a, b) = somma(&f[..8], le, c0, c1);
        let (a, b) = somma(&f[24..], le, a, b);
        if (u32_be(&f, 16), u32_be(&f, 20)) != (Some(a), Some(b)) {
            break;
        }
        (c0, c1) = (a, b);
        let n = u32_be(&f, 0).ok_or_else(|| guasto("log frame"))?;
        in_attesa.push((n, off + 24));
        let dopo = u32_be(&f, 4).unwrap_or(0);
        if dopo != 0 {
            mappa.extend(in_attesa.drain(..));
            pagine = u64::from(dopo);
        }
        off += passo;
    }
    Ok((!mappa.is_empty()).then_some((pagina, mappa, pagine)))
}
