//! The side tables of a function: `_FUNCnnn.EE` (cone voltage and collision energy of each channel of an MRM function), `_FUNCnnn.STS` (the housekeeping values of each scan: collision
//! energy, set mass, lock mass correction...) and `_FUNCnnn.CMP` (the compound names of the channels). `EE` and `STS` share one layout: a 32-byte preamble (offset of the records `u16`,
//! bytes per record `u16` at 4, number of descriptors `u16` at 6), 48-byte descriptors (sequence, type, offset, name of 42 bytes), then the records.

use super::cartella::{f32_at, u16_at, u32_at};
use mzlab_core::lettori::Limits;

const PREAMBLE: usize = 32;
const DESCR: usize = 48;

#[derive(Debug, Clone)]
pub(super) struct Channel {
    pub(super) name: String,
    /// 0: `u8`, 1: `i16`, 2: `u32`, 3: `f32`.
    kind: u16,
    offset: usize,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Table {
    pub(super) channels: Vec<Channel>,
    rec: usize,
    data: Vec<u8>,
}

impl Table {
    /// The table of `b`; `None` for a file that is not one (the table is optional: the scans do not depend on it).
    pub(super) fn read(b: &[u8], limits: &Limits) -> Option<Table> {
        let (start, rec, n) = (
            u16_at(b, 0) as usize,
            u16_at(b, 4) as usize,
            u16_at(b, 6) as usize,
        );
        if rec == 0 || start != PREAMBLE + n * DESCR || b.len() < start {
            return None;
        }
        limits.ok_entries(n as u64).ok()?;
        let canali: Vec<Channel> = (0..n)
            .map(|i| {
                let d = &b[PREAMBLE + i * DESCR..PREAMBLE + (i + 1) * DESCR];
                let nome: Vec<u8> = d[6..].iter().copied().take_while(|&c| c != 0).collect();
                Channel {
                    name: super::cartella::text(&nome).trim().to_string(),
                    kind: u16_at(d, 2),
                    offset: u16_at(d, 4) as usize,
                }
            })
            .collect();
        let larg = |t: u16| match t {
            0 => 1,
            1 => 2,
            _ => 4,
        };
        if canali
            .iter()
            .any(|c| c.kind > 3 || c.offset + larg(c.kind) > rec)
        {
            return None;
        }
        let nrec = (b.len() - start) / rec;
        Some(Table {
            channels: canali,
            rec,
            data: b[start..start + nrec * rec].to_vec(),
        })
    }

    fn value_of(&self, c: &Channel, k: usize) -> Option<f64> {
        let r = self.data.get(k * self.rec..(k + 1) * self.rec)?;
        Some(match c.kind {
            0 => f64::from(*r.get(c.offset)?),
            1 => f64::from(u16_at(r, c.offset) as i16),
            2 => f64::from(u32_at(r, c.offset)),
            _ => f64::from(f32_at(r, c.offset)),
        })
    }

    /// The value of channel `nome` in record `k`.
    pub(super) fn value(&self, name: &str, k: usize) -> Option<f64> {
        let c = self.channels.iter().find(|c| c.name == name)?;
        self.value_of(c, k)
    }

    /// Every channel of record `k` as (name, value): what `campi` keeps of a scan.
    pub(super) fn all(&self, k: usize) -> Vec<(String, f64)> {
        self.channels
            .iter()
            .filter_map(|c| Some((c.name.clone(), self.value_of(c, k)?)))
            .filter(|(n, v)| !n.is_empty() && v.is_finite())
            .collect()
    }
}

/// The names of `_FUNCnnn.CMP`: 12 bytes of header, then records of 1024 bytes that begin with the name.
pub(super) fn compound_names(b: &[u8], limits: &Limits) -> Vec<String> {
    if b.len() < 12 || limits.ok_entries(((b.len() - 12) / 1024) as u64).is_err() {
        return Vec::new();
    }
    b[12..]
        .chunks_exact(1024)
        .map(|r| {
            let n: Vec<u8> = r.iter().copied().take_while(|&c| c != 0).collect();
            super::cartella::text(&n).trim().to_string()
        })
        .collect()
}
