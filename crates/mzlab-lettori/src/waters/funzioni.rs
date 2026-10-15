//! `_FUNCTNS.INF`: one 416-byte record per acquisition function (scan time, dwell times, the acquisition range or the 32 slots of channels).

use super::cartella::{errore, f32_at, u16_at};
use mzlab_core::error::Result;
use mzlab_core::lettori::Limiti;

const RECORD: usize = 416;
const SLOTS: usize = 32;

#[derive(Debug, Clone)]
pub(super) struct Funzione {
    /// 1-based, the `nnn` of `_FUNCnnn.*`.
    pub(super) indice: u32,
    pub(super) tipo: u8,
    pub(super) sotto: u8,
    /// Seconds per scan cycle (`+0x002`).
    pub(super) ciclo_s: f32,
    pub(super) tof_depth: u16,
    /// Slots of channels: Q1 (`+0xA0`), Q3 (`+0x120`) and dwell time in seconds (`+0x20`); only the slots with Q1 > 0 are channels.
    pub(super) q1: Vec<f32>,
    pub(super) q3: Vec<f32>,
    pub(super) dwell_s: Vec<f32>,
}

impl Funzione {
    /// Channels of a function of SIR or MRM (the low nibble of the type is 9).
    pub(super) fn mirato(&self) -> bool {
        self.tipo & 0x0F == 9
    }

    /// Bit 7 of the subtype marks the function of the lock mass (the reference).
    pub(super) fn riferimento(&self) -> bool {
        !self.mirato() && self.sotto & 0x80 != 0
    }

    /// Acquisition range of a scanning function.
    pub(super) fn intervallo(&self) -> Option<(f64, f64)> {
        let (a, b) = (*self.q1.first()?, *self.q3.first()?);
        (a.is_finite() && b.is_finite() && a > 0.0 && b > a).then_some((a as f64, b as f64))
    }

    /// The channels as (Q1, Q3, dwell s), slots with Q1 > 0 in order.
    pub(super) fn canali(&self) -> Vec<(f32, f32, f32)> {
        (0..self.q1.len())
            .filter(|&i| self.q1[i] > 0.0)
            .map(|i| (self.q1[i], self.q3[i], self.dwell_s[i]))
            .collect()
    }
}

/// The functions of `bytes`. A size that is no multiple of the record is a damaged file.
pub(super) fn leggi(b: &[u8], limiti: &Limiti) -> Result<Vec<Funzione>> {
    if b.is_empty() {
        return Err(errore("_FUNCTNS.INF is empty"));
    }
    if b.len() % RECORD != 0 {
        return Err(errore("_FUNCTNS.INF size"));
    }
    let n = limiti.voci_ok((b.len() / RECORD) as u64)?;
    Ok((0..n)
        .map(|i| {
            let r = &b[i * RECORD..(i + 1) * RECORD];
            let slot =
                |base: usize| -> Vec<f32> { (0..SLOTS).map(|k| f32_at(r, base + 4 * k)).collect() };
            Funzione {
                indice: i as u32 + 1,
                tipo: r[0],
                sotto: r[1],
                ciclo_s: f32_at(r, 2),
                tof_depth: u16_at(r, 0x10),
                q1: slot(0xA0),
                q3: slot(0x120),
                dwell_s: slot(0x20),
            }
        })
        .collect())
}
