//! The binary transport between a readers module (`wasm-lettori/<family>/`) and the engine that reads it through `ExternalReader` (docs/agenti/lettori-modulo.md): the index of a sample
//! and a scan travel as one byte array each, little-endian, written by the module (`encode_*`) and read by the engine (`decode_*`). Nothing is text but the strings of the index.
//!
//! Index (`PROTOCOLLO`, then in this order; the columns are 8-byte numbers, so the whole block of numbers is aligned):
//! `u32 version, u32 n, u32 filters, u32 filter bytes, u32 id bytes, u32 0`; `f64 rt[n]`, `f64 tic[n]`, `f64 base[n]` (NaN: none), `f64 precursor[n]` (NaN: none),
//! `f64 pos[3n]` (member, offset, length; member -1: none); `u32 filter_of[n]` (index in the table of the filters, each text once), `u32 filter_off[filters+1]`, `u32 id_off[n+1]`;
//! `u8 level[n]`, `i8 polarity[n]`; the filter texts, the id texts (UTF-8, offsets in bytes from the start of their block).
//!
//! Scan: `u8 level, i8 polarity, u8 flags (1 profile, 2 intensities are f64, 4 has a precursor), u8 0, u32 points, f64 rt, f64 precursor`, then `f64 mz[points]` and
//! the intensities (`f32[points]`, or `f64[points]` with flag 2: the file holds 64-bit ones). The id and the filter are not sent: the engine has them in the index.
use crate::error::{Error, Result};
use crate::mzml::Spectrum;
use crate::readers::{Position, ScanEntry};

/// Version of this format; the module answers it (`protocollo()`) and the first word of an index repeats it.
pub const PROTOCOLLO: u32 = 2;

const INDEX_HEAD: usize = 24;
/// Bytes of the fixed head of a scan.
pub const SCAN_HEAD: usize = 24;

const PROFILO: u8 = 1;
const WIDE: u8 = 2;
const WITH_PRECURSOR: u8 = 4;

/// An answer of the module the engine cannot decode: a fault of the module or a module of another version, never the user's file type.
fn guasto(what: &str) -> Error {
    Error::new("err.module.reply").with("detail", what)
}

fn u32_di(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn column(o: &mut Vec<u8>, v: impl Iterator<Item = f64>) {
    for x in v {
        o.extend_from_slice(&x.to_le_bytes());
    }
}

/// The index of a sample as a block of bytes.
pub fn encode_index(v: &[ScanEntry]) -> Vec<u8> {
    let mut filters: Vec<&str> = Vec::new();
    let mut of_filter: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    let mut which: Vec<u32> = Vec::with_capacity(v.len());
    for r in v {
        let k = *of_filter.entry(r.filtro.as_str()).or_insert_with(|| {
            filters.push(r.filtro.as_str());
            u32_di(filters.len() - 1)
        });
        which.push(k);
    }
    let filter_bytes: usize = filters.iter().map(|f| f.len()).sum();
    let id_bytes: usize = v.iter().map(|r| r.id.len()).sum();
    let n = v.len();
    let mut o: Vec<u8> = Vec::with_capacity(
        INDEX_HEAD
            + n * (7 * 8 + 4 + 4 + 2)
            + (filters.len() + 1) * 4
            + 4
            + filter_bytes
            + id_bytes,
    );
    for x in [
        PROTOCOLLO,
        u32_di(n),
        u32_di(filters.len()),
        u32_di(filter_bytes),
        u32_di(id_bytes),
        0,
    ] {
        o.extend_from_slice(&x.to_le_bytes());
    }
    column(&mut o, v.iter().map(|r| r.rt));
    column(&mut o, v.iter().map(|r| r.tic));
    column(&mut o, v.iter().map(|r| r.base_peak.unwrap_or(f64::NAN)));
    column(&mut o, v.iter().map(|r| r.precursore.unwrap_or(f64::NAN)));
    for r in v {
        let (m, off, len) = match r.position {
            Some(p) => (f64::from(p.membro), p.offset as f64, f64::from(p.lunghezza)),
            None => (-1.0, 0.0, 0.0),
        };
        column(&mut o, [m, off, len].into_iter());
    }
    for k in &which {
        o.extend_from_slice(&k.to_le_bytes());
    }
    let mut a = 0u32;
    o.extend_from_slice(&a.to_le_bytes());
    for f in &filters {
        a = a.saturating_add(u32_di(f.len()));
        o.extend_from_slice(&a.to_le_bytes());
    }
    a = 0;
    o.extend_from_slice(&a.to_le_bytes());
    for r in v {
        a = a.saturating_add(u32_di(r.id.len()));
        o.extend_from_slice(&a.to_le_bytes());
    }
    o.extend(v.iter().map(|r| r.livello));
    o.extend(v.iter().map(|r| r.polarita as u8));
    for f in &filters {
        o.extend_from_slice(f.as_bytes());
    }
    for r in v {
        o.extend_from_slice(r.id.as_bytes());
    }
    o
}

/// A cursor over the blocks of an index, each read checked against the end.
struct Reading<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Reading<'a> {
    fn prendi(&mut self, n: usize) -> Result<&'a [u8]> {
        let fine = self.p.checked_add(n).filter(|&f| f <= self.b.len());
        let fine = fine.ok_or_else(|| guasto("index"))?;
        let s = &self.b[self.p..fine];
        self.p = fine;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32> {
        let s = self.prendi(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn f64s(&mut self, n: usize) -> Result<Vec<f64>> {
        let s = self.prendi(n.checked_mul(8).ok_or_else(|| guasto("index"))?)?;
        Ok(s.chunks_exact(8).map(f64_di).collect())
    }

    fn u32s(&mut self, n: usize) -> Result<Vec<u32>> {
        let s = self.prendi(n.checked_mul(4).ok_or_else(|| guasto("index"))?)?;
        Ok(s.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect())
    }
}

fn f64_di(c: &[u8]) -> f64 {
    f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]])
}

fn text(block: &[u8], off: &[u32], i: usize) -> Result<String> {
    let (a, b) = (off[i] as usize, off[i + 1] as usize);
    let s = block.get(a..b).ok_or_else(|| guasto("index"))?;
    Ok(String::from_utf8_lossy(s).into_owned())
}

/// The rows of an index written by `encode_index`.
pub fn decode_index(b: &[u8]) -> Result<Vec<ScanEntry>> {
    let mut l = Reading { b, p: 0 };
    if l.u32()? != PROTOCOLLO {
        return Err(guasto("protocol"));
    }
    let n = l.u32()? as usize;
    let nf = l.u32()? as usize;
    let (byte_f, byte_i) = (l.u32()? as usize, l.u32()? as usize);
    l.u32()?;
    // the declared sizes must fit the block before anything proportional to them is allocated
    if n.saturating_mul(6 * 8 + 4 + 4 + 2) > b.len() {
        return Err(guasto("index"));
    }
    let rt = l.f64s(n)?;
    let tic = l.f64s(n)?;
    let base = l.f64s(n)?;
    let prec = l.f64s(n)?;
    let pos = l.f64s(n * 3)?;
    let which = l.u32s(n)?;
    let off_f = l.u32s(nf.checked_add(1).ok_or_else(|| guasto("index"))?)?;
    let off_i = l.u32s(n + 1)?;
    let liv = l.prendi(n)?;
    let pol = l.prendi(n)?;
    let t_f = l.prendi(byte_f)?;
    let t_i = l.prendi(byte_i)?;
    let filters = (0..nf)
        .map(|i| text(t_f, &off_f, i))
        .collect::<Result<Vec<_>>>()?;
    let opz = |x: f64| (!x.is_nan()).then_some(x);
    (0..n)
        .map(|i| {
            Ok(ScanEntry {
                id: text(t_i, &off_i, i)?,
                rt: rt[i],
                livello: liv[i],
                polarita: pol[i] as i8,
                tic: tic[i],
                base_peak: opz(base[i]),
                filtro: filters
                    .get(which[i] as usize)
                    .ok_or_else(|| guasto("index"))?
                    .clone(),
                precursore: opz(prec[i]),
                position: (pos[3 * i] >= 0.0).then(|| Position {
                    membro: pos[3 * i] as u16,
                    offset: pos[3 * i + 1] as u64,
                    lunghezza: pos[3 * i + 2] as u32,
                }),
            })
        })
        .collect()
}

/// The fixed head of a scan: level, polarity, flags, the number of points, rt and precursor (NaN: none).
pub fn testa_scansione(s: &Spectrum) -> [u8; SCAN_HEAD] {
    let mut flag = 0u8;
    if s.profile {
        flag |= PROFILO;
    }
    if s.wide.is_some() {
        flag |= WIDE;
    }
    if s.precursor.is_some() {
        flag |= WITH_PRECURSOR;
    }
    let mut t = [0u8; SCAN_HEAD];
    t[..4].copy_from_slice(&[s.level, s.polarity as u8, flag, 0]);
    t[4..8].copy_from_slice(&u32_di(s.mz.len()).to_le_bytes());
    t[8..16].copy_from_slice(&s.rt.to_le_bytes());
    t[16..24].copy_from_slice(&s.precursor.unwrap_or(f64::NAN).to_le_bytes());
    t
}

/// A scan as a block of bytes: the fixed head, the m/z, the intensities.
pub fn encode_scansione(s: &Spectrum) -> Vec<u8> {
    let n = s.mz.len();
    let mut o: Vec<u8> = Vec::with_capacity(SCAN_HEAD + n * if s.wide.is_some() { 16 } else { 12 });
    o.extend_from_slice(&testa_scansione(s));
    column(&mut o, s.mz.iter().copied());
    match &s.wide {
        Some(w) => column(&mut o, w.iter().take(n).copied()),
        None => {
            for y in s.intensity.iter().take(n) {
                o.extend_from_slice(&y.to_le_bytes());
            }
        }
    }
    o
}

/// The intensities of a scan as the module sends them: 32-bit, or 64-bit (`Larghe`) when the file stores them so.
pub enum Intensity {
    Strette(Vec<f32>),
    Larghe(Vec<f64>),
}

/// The scan `k` from its head and its columns, as `decode_scansione` gives it from the block (the columns cross separately, without the block).
pub fn scan_from_parts(k: usize, t: &[u8], mz: Vec<f64>, y: Intensity) -> Result<Spectrum> {
    if t.len() != SCAN_HEAD {
        return Err(guasto("scan"));
    }
    let flag = t[2];
    let n = u32::from_le_bytes([t[4], t[5], t[6], t[7]]) as usize;
    let (intensity, wide) = match y {
        Intensity::Larghe(y) if flag & WIDE != 0 && y.len() == n => larghe(y),
        Intensity::Strette(f) if flag & WIDE == 0 && f.len() == n => (f, None),
        _ => return Err(guasto("scan")),
    };
    if mz.len() != n {
        return Err(guasto("scan"));
    }
    let prec = f64_di(&t[16..24]);
    Ok(Spectrum {
        index: k,
        id: String::new(),
        level: t[0],
        rt: f64_di(&t[8..16]),
        polarity: t[1] as i8,
        profile: flag & PROFILO != 0,
        filter: String::new(),
        precursor: (flag & WITH_PRECURSOR != 0).then_some(prec),
        mz,
        intensity,
        wide,
    })
}

/// 64-bit intensities as 32-bit ones, kept as they are only when a 32-bit one cannot hold them.
fn larghe(y: Vec<f64>) -> (Vec<f32>, Option<Vec<f64>>) {
    let f: Vec<f32> = y.iter().map(|&x| x as f32).collect();
    let without_loss = y
        .iter()
        .zip(&f)
        .all(|(&a, &c)| a == f64::from(c) || (a.is_nan() && c.is_nan()));
    (f, (!without_loss).then_some(y))
}

/// The scan `k` of a block written by `encode_scansione`; its `id` and `filter` are empty (the index has them).
pub fn decode_scansione(k: usize, b: &[u8]) -> Result<Spectrum> {
    if b.len() < SCAN_HEAD {
        return Err(guasto("scan"));
    }
    let larghe = b[2] & WIDE != 0;
    let n = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize;
    if n.checked_mul(if larghe { 16 } else { 12 })
        .and_then(|x| x.checked_add(SCAN_HEAD))
        != Some(b.len())
    {
        return Err(guasto("scan"));
    }
    let mz: Vec<f64> = b[SCAN_HEAD..SCAN_HEAD + 8 * n]
        .chunks_exact(8)
        .map(f64_di)
        .collect();
    let rest = &b[SCAN_HEAD + 8 * n..];
    let y = if larghe {
        Intensity::Larghe(rest.chunks_exact(8).map(f64_di).collect())
    } else {
        Intensity::Strette(
            rest.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        )
    };
    scan_from_parts(k, &b[..SCAN_HEAD], mz, y)
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
