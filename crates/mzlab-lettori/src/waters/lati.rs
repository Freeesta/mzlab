//! The side tables of a function: `_FUNCnnn.EE` (cone voltage and collision energy of each channel of an MRM function), `_FUNCnnn.STS` (the housekeeping values of each scan: collision
//! energy, set mass, lock mass correction...) and `_FUNCnnn.CMP` (the compound names of the channels). `EE` and `STS` share one layout: a 32-byte preamble (offset of the records `u16`,
//! bytes per record `u16` at 4, number of descriptors `u16` at 6), 48-byte descriptors (sequence, type, offset, name of 42 bytes), then the records.

use super::cartella::{f32_at, u16_at, u32_at};
use mzlab_core::lettori::Limiti;

const PREAMBOLO: usize = 32;
const DESCR: usize = 48;

#[derive(Debug, Clone)]
pub(super) struct Canale {
    pub(super) nome: String,
    /// 0: `u8`, 1: `i16`, 2: `u32`, 3: `f32`.
    tipo: u16,
    offset: usize,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Tabella {
    pub(super) canali: Vec<Canale>,
    rec: usize,
    dati: Vec<u8>,
}

impl Tabella {
    /// The table of `b`; `None` for a file that is not one (the table is optional: the scans do not depend on it).
    pub(super) fn leggi(b: &[u8], limiti: &Limiti) -> Option<Tabella> {
        let (inizio, rec, n) = (
            u16_at(b, 0) as usize,
            u16_at(b, 4) as usize,
            u16_at(b, 6) as usize,
        );
        if rec == 0 || inizio != PREAMBOLO + n * DESCR || b.len() < inizio {
            return None;
        }
        limiti.voci_ok(n as u64).ok()?;
        let canali: Vec<Canale> = (0..n)
            .map(|i| {
                let d = &b[PREAMBOLO + i * DESCR..PREAMBOLO + (i + 1) * DESCR];
                let nome: Vec<u8> = d[6..].iter().copied().take_while(|&c| c != 0).collect();
                Canale {
                    nome: super::cartella::testo(&nome).trim().to_string(),
                    tipo: u16_at(d, 2),
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
            .any(|c| c.tipo > 3 || c.offset + larg(c.tipo) > rec)
        {
            return None;
        }
        let nrec = (b.len() - inizio) / rec;
        Some(Tabella {
            canali,
            rec,
            dati: b[inizio..inizio + nrec * rec].to_vec(),
        })
    }

    fn valore_di(&self, c: &Canale, k: usize) -> Option<f64> {
        let r = self.dati.get(k * self.rec..(k + 1) * self.rec)?;
        Some(match c.tipo {
            0 => f64::from(*r.get(c.offset)?),
            1 => f64::from(u16_at(r, c.offset) as i16),
            2 => f64::from(u32_at(r, c.offset)),
            _ => f64::from(f32_at(r, c.offset)),
        })
    }

    /// The value of channel `nome` in record `k`.
    pub(super) fn valore(&self, nome: &str, k: usize) -> Option<f64> {
        let c = self.canali.iter().find(|c| c.nome == nome)?;
        self.valore_di(c, k)
    }

    /// Every channel of record `k` as (name, value): what `campi` keeps of a scan.
    pub(super) fn tutti(&self, k: usize) -> Vec<(String, f64)> {
        self.canali
            .iter()
            .filter_map(|c| Some((c.nome.clone(), self.valore_di(c, k)?)))
            .filter(|(n, v)| !n.is_empty() && v.is_finite())
            .collect()
    }
}

/// The names of `_FUNCnnn.CMP`: 12 bytes of header, then records of 1024 bytes that begin with the name.
pub(super) fn nomi_composti(b: &[u8], limiti: &Limiti) -> Vec<String> {
    if b.len() < 12 || limiti.voci_ok(((b.len() - 12) / 1024) as u64).is_err() {
        return Vec::new();
    }
    b[12..]
        .chunks_exact(1024)
        .map(|r| {
            let n: Vec<u8> = r.iter().copied().take_while(|&c| c != 0).collect();
            super::cartella::testo(&n).trim().to_string()
        })
        .collect()
}
