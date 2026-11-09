//! MRM of the Shimadzu files as chromatograms: the points of a `.qgd` MRM record and of the records of `TLM Raw Data` (`.lcd` of a triple quadrupole) are (Q1, Q3, counts) at the time of
//! the record; one chromatogram per (Q1, Q3, polarity), with the id `SRM SIC Q1=<q1> Q3=<q3> sample=1 period=1 experiment=1 transition=<j>` as the `.wiff` reader writes it.
//! A `TLM Raw Data` record is a 12-byte head (`FFFFFFFF`-style marker, sizes) and a zlib stream of `u32` pairs (Q1 × 100, Q3 × 100) and counts; `Spectrum Index` has 24 bytes per record.

use std::collections::HashMap;
use std::io::Read;

use flate2::read::ZlibDecoder;
use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::Limits;
use mzlab_core::mzml::{ChromKind, Chromatogram};

const UNITS: &str = "number of detector counts";
/// The most bytes of one decompressed record (a record holds a few transitions).
const RECORD_MAX: u64 = 1 << 24;
/// Bytes of the head of a decompressed record, and of one point in it.
const HEAD: usize = 44;
const POINT: usize = 12;

/// One transition and its points.
pub(super) struct Series {
    pub q1: f64,
    pub q3: f64,
    pub pol: i8,
    pub t: Vec<f64>,
    pub y: Vec<f64>,
}

/// The series in the order of first appearance.
#[derive(Default)]
pub(super) struct Traces {
    chiavi: HashMap<(u32, u32, i8), usize>,
    pub series: Vec<Series>,
    points: usize,
}

impl Traces {
    /// Adds a point; `q1`/`q3` are the integers of the file and `scala` what divides them to get m/z.
    pub(super) fn add(
        &mut self,
        (q1, q3): (u32, u32),
        scale: f64,
        pol: i8,
        t_min: f64,
        y: f64,
        limits: &Limits,
    ) -> Result<()> {
        self.points += 1;
        limits.ok_entries(self.points as u64)?;
        let j = match self.chiavi.get(&(q1, q3, pol)) {
            Some(&j) => j,
            None => {
                limits.ok_entries(self.series.len() as u64 + 1)?;
                self.series.push(Series {
                    q1: f64::from(q1) / scale,
                    q3: f64::from(q3) / scale,
                    pol,
                    t: Vec::new(),
                    y: Vec::new(),
                });
                self.chiavi.insert((q1, q3, pol), self.series.len() - 1);
                self.series.len() - 1
            }
        };
        self.series[j].t.push(t_min);
        self.series[j].y.push(y);
        Ok(())
    }

    /// The chromatograms, numbered from `da` (the TIC takes the ones before).
    pub(super) fn chromatograms(&self, da: usize) -> Vec<Chromatogram> {
        self.series
            .iter()
            .enumerate()
            .map(|(j, s)| Chromatogram {
                index: da + j,
                id: format!(
                    "SRM SIC Q1={} Q3={} sample=1 period=1 experiment=1 transition={j}{}",
                    s.q1,
                    s.q3,
                    if s.pol < 0 { " polarity=negative" } else { "" }
                ),
                kind: ChromKind::Srm,
                q1: Some(s.q1),
                q3: Some(s.q3),
                unit: UNITS.into(),
                time: s.t.clone(),
                intensity: s.y.clone(),
            })
            .collect()
    }
}

/// The polarity a record of `TLM Raw Data` says in the byte at 33 of its decompressed head: both bits 6–7 set is negative, both clear positive (4 files: one negative, three positive).
pub(super) fn polarity_tlm(r: &[u8]) -> i8 {
    match r.get(33).map(|b| b & 0xc0) {
        Some(0xc0) => -1,
        Some(0) => 1,
        _ => 0,
    }
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// The decompressed record at `off` of `raw`, `None` when it is not a record (the marker, the sizes or the stream do not agree).
pub(super) fn record(raw: &[u8], off: usize) -> Option<Vec<u8>> {
    let (us, cs) = (u32_at(raw, off + 4)?, u32_at(raw, off + 8)?);
    if u64::from(us) > RECORD_MAX {
        return None;
    }
    let z = raw.get(off + 12..off + 12 + usize::try_from(cs).ok()?)?;
    let mut out = Vec::new();
    ZlibDecoder::new(z)
        .take(u64::from(us) + 1)
        .read_to_end(&mut out)
        .ok()?;
    (out.len() == usize::try_from(us).ok()?).then_some(out)
}

/// The points (Q1 × 100, Q3 × 100, counts) of a decompressed record; `None` when its length is not `head + n × 12`.
pub(super) fn points(r: &[u8]) -> Option<Vec<(u32, u32, u64)>> {
    let n = usize::try_from(u32_at(r, 40)?).ok()?;
    if n.checked_mul(POINT)?.checked_add(HEAD)? != r.len() {
        return None;
    }
    Some(
        (0..n)
            .filter_map(|j| {
                let p = HEAD + j * POINT;
                Some((
                    u32_at(r, p)?,
                    u32_at(r, p + 4)?,
                    u64::from(u32_at(r, p + 8)?),
                ))
            })
            .collect(),
    )
}

/// The entries of `Spectrum Index` of `TLM Raw Data`: (offset of the record in `MS Raw Data`) of each, 24 bytes per record `(length, scan, offset, 0, a, b)`.
pub(super) fn offset_tlm(idx: &[u8], n: usize, limits: &Limits) -> Result<Vec<usize>> {
    limits.ok_entries(n as u64)?;
    if idx.len() < 24 * n {
        return Err(Error::new("err.file.shimadzu").with("detail", "Spectrum Index is short"));
    }
    Ok((0..n)
        .map(|i| u32_at(idx, 24 * i + 8).map_or(usize::MAX, |o| o as usize))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn record_round_trip() {
        let mut r = vec![0u8; HEAD];
        r[40..44].copy_from_slice(&1u32.to_le_bytes());
        r[33] = 0xcd;
        for v in [17610u32, 15910, 42] {
            r.extend_from_slice(&v.to_le_bytes());
        }
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        z.write_all(&r).unwrap();
        let z = z.finish().unwrap();
        let mut raw = vec![0xffu8; 4];
        raw.extend_from_slice(&(r.len() as u32).to_le_bytes());
        raw.extend_from_slice(&(z.len() as u32).to_le_bytes());
        raw.extend_from_slice(&z);
        let d = record(&raw, 0).expect("record");
        assert_eq!(points(&d), Some(vec![(17610, 15910, 42)]));
        assert_eq!(polarity_tlm(&d), -1);
        assert!(record(&raw[..raw.len() - 3], 0).is_none());
    }
}
