//! The scans of a SIR or MRM function: one value per channel, no m/z. Two widths, told by the IDX like the scanning records: 4 bytes (22-bit mantissa, 10-bit exponent: `m * 2^(e-21)`,
//! exact against the TIC of the index) and 2 bytes (13-bit mantissa, 3-bit exponent in base 4: `m * 4^e`, within its quantisation).

use super::cartella::errore;
use mzlab_core::error::Result;

/// The `n` channel values of a scan of `b`, whose records are `larghezza` bytes wide.
pub(super) fn valori(b: &[u8], larghezza: u64, n: usize) -> Result<Vec<f64>> {
    let w = usize::try_from(larghezza).map_err(|_| errore("channel width"))?;
    if !(w == 2 || w == 4) || b.len() < n.saturating_mul(w) {
        return Err(errore("channel record width"));
    }
    Ok(b.chunks_exact(w)
        .take(n)
        .map(|r| {
            if w == 4 {
                let v = u32::from_le_bytes([r[0], r[1], r[2], r[3]]);
                f64::from(v & 0x003F_FFFF) * 2f64.powi((v >> 22) as i32 - 21)
            } else {
                let v = u16::from_le_bytes([r[0], r[1]]);
                f64::from(v >> 3) * 4f64.powi(i32::from(v & 7))
            }
        })
        .collect())
}
