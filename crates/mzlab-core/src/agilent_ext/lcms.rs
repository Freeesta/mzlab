//! The LC/MS variants of an Agilent MassHunter `.D` (feature `estesi`, served by the `agilent` module): QqQ runs (multiple reaction monitoring, one record per transition
//! visit) and Q-TOF runs (centroids as flight times, profiles, MS/MS), in either polarity. Formats written in `docs/agenti/agilent.md`; the free reference for the
//! idea of the layout is the `openaraw` crate (Apache-2.0), nothing is copied from it.
//!
//! - `SpectrumFormatID` 3 (QqQ): `points` m/z as `f64`, then `points` intensities as `f32`; 2 (Q-TOF centroid): the same with FLIGHT TIMES instead of m/z;
//!   1 with `UncompressedByteCount = 16 + 4·points` (Q-TOF profile, `MSProfile.bin`): `x0`, `dx` (`f64`) and `points` `i32` intensities, as an LZF stream or as a
//!   run-length stream behind a header that is stored as it is.
//! - m/z of a time `t`: `(A·(t−T0))² − Σ c_k·clamp(t, tmin, tmax)^p_k`, with the powers `p_k` the set bits of the `ValueUseFlags` of the `Polynomial` step of
//!   `DefaultMassCal.xml`; the coefficients of the scan come from its record in `MSMassCal.bin` (72-byte header, then 84 bytes per scan: `u32` 10, then ten `f64`:
//!   A, T0, tmin, tmax, c0…c5), else from the default calibration the record names with `CalibrationID`.

use std::collections::BTreeMap;

pub use super::lccal::Calibration;
use super::lcprofile::{bad, f64_at, lzf, run_length};
use super::scan::ScanRec;
use super::AgilentD;
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::mzml::{ChromKind, Chromatogram};

/// `ScanType` of a QqQ multiple-reaction-monitoring scan and of a Q-TOF product-ion scan.
pub const SCAN_MRM: i32 = 256;
pub const SCAN_PRODUCT: i32 = 512;

/// The most bytes a spectrum block may hold, stored or unpacked.
const MAX_BLOCK: u64 = 1 << 26;

impl AgilentD {
    /// The calibration of the run (a Q-TOF's), from `MSMassCal.bin` and `DefaultMassCal.xml`.
    pub fn with_calibration(mut self, mass_cal: Option<&[u8]>, default_cal: Option<&[u8]>) -> Self {
        self.calibration = Calibration::parse(mass_cal, default_cal);
        self
    }

    /// A QqQ multiple-reaction-monitoring scan: its points are the transitions of the visit, not a spectrum.
    pub(crate) fn is_mrm(&self, r: &ScanRec) -> bool {
        r.scan_type == SCAN_MRM
    }

    /// The scan has a spectrum to list: points, and not a QqQ MRM visit (those make the SRM chromatograms).
    pub(crate) fn has_spectrum(&self, r: &ScanRec) -> bool {
        r.points > 0 && !self.is_mrm(r)
    }

    fn is_gc_block(r: &ScanRec) -> bool {
        r.format == 1
            && r.bytes as u64 == r.points as u64 * 16
            && r.uncompressed.is_none_or(|u| u == r.bytes)
    }

    /// The scan is a profile (`MSProfile.bin` block of the Q-TOF).
    pub(crate) fn is_profile(&self, r: &ScanRec) -> bool {
        r.format == 1 && !Self::is_gc_block(r)
    }

    /// The value of the field `name` of the layout that `ScanRec` keeps in `extra`.
    pub fn extra_of_record(&self, r: &ScanRec, name: &str) -> Option<f64> {
        let k = self.extra_names.iter().position(|n| n == name)?;
        r.extra.get(k).copied()
    }

    /// The position of the record in `records` (scan ids count from 1 in the runs seen; searched when they do not).
    fn position(&self, r: &ScanRec) -> Option<usize> {
        let guess = (r.id as usize).checked_sub(1)?;
        if self.records.get(guess).is_some_and(|x| x.id == r.id) {
            return Some(guess);
        }
        self.records.iter().position(|x| x.id == r.id)
    }

    fn read_block<R: ReadAt>(&self, peak: &mut R, r: &ScanRec) -> Result<Vec<u8>> {
        if r.bytes as u64 > MAX_BLOCK {
            return Err(Error::memory_budget());
        }
        let mut raw = vec![0u8; r.bytes as usize];
        peak.read_at(r.offset, &mut raw)?;
        Ok(raw)
    }

    fn to_mz(&self, r: &ScanRec, times: &[f64]) -> Result<Vec<f64>> {
        if self.calibration.is_empty() {
            return Err(bad(format!(
                "scan {}: flight times and no mass calibration (DefaultMassCal.xml)",
                r.id
            )));
        }
        let id = self.extra_of_record(r, "CalibrationID").unwrap_or(0.0) as i32;
        let at = self.position(r);
        times
            .iter()
            .map(|&t| {
                self.calibration
                    .mz(id, at, t)
                    .ok_or_else(|| bad(format!("scan {}: no m/z for the time {t}", r.id)))
            })
            .collect()
    }

    /// The points of a scan of an LC/MS run (and of a GC/MS one: the 16-byte layout is read as before). Sorted by m/z.
    pub(crate) fn read_points_ext<R: ReadAt>(
        &self,
        peak: &mut R,
        r: &ScanRec,
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        let n = r.points as usize;
        if Self::is_gc_block(r) {
            return super::peak::read_points(peak, r);
        }
        let (x, y) = match r.format {
            2 | 3 => {
                if r.bytes as u64 != r.points as u64 * 12 {
                    return Err(bad(format!(
                        "scan {}: {} bytes for {} points of format {}",
                        r.id, r.bytes, r.points, r.format
                    )));
                }
                let raw = self.read_block(peak, r)?;
                let x: Vec<f64> = (0..n).map(|i| f64_at(&raw, i * 8)).collect();
                let y: Vec<f64> = (0..n)
                    .map(|i| {
                        // the intensities follow the n × 8 bytes of x
                        let at = n * 8 + i * 4;
                        f32::from_le_bytes([raw[at], raw[at + 1], raw[at + 2], raw[at + 3]]) as f64
                    })
                    .collect();
                (if r.format == 2 { self.to_mz(r, &x)? } else { x }, y)
            }
            1 => self.profile(peak, r)?,
            f => {
                return Err(bad(format!(
                    "scan {}: SpectrumFormatID {f} is not a known layout",
                    r.id
                )))
            }
        };
        if x.iter().chain(&y).any(|v| !v.is_finite()) {
            return Err(bad(format!("scan {}: a value is not a number", r.id)));
        }
        if x.windows(2).all(|w| w[0] <= w[1]) {
            return Ok((x, y));
        }
        let mut ord: Vec<usize> = (0..x.len()).collect();
        crate::sortc::slice_by(&mut ord, |&a, &b| x[a].total_cmp(&x[b]));
        Ok((
            ord.iter().map(|&i| x[i]).collect(),
            ord.iter().map(|&i| y[i]).collect(),
        ))
    }

    /// A profile block: `x0`, `dx` and `points` intensities (see the module comment); the flight time of point `i` is `x0 + i·dx`.
    fn profile<R: ReadAt>(&self, peak: &mut R, r: &ScanRec) -> Result<(Vec<f64>, Vec<f64>)> {
        let n = r.points as usize;
        let size = 16 + 4 * r.points as u64;
        if r.uncompressed.is_some_and(|u| u as u64 != size) || size > MAX_BLOCK {
            return Err(bad(format!(
                "scan {}: {} unpacked bytes for {} profile points",
                r.id,
                r.uncompressed.unwrap_or(0),
                r.points
            )));
        }
        let raw = self.read_block(peak, r)?;
        if raw.len() < 20 {
            return Err(bad(format!(
                "scan {}: a profile block of {} bytes",
                r.id,
                raw.len()
            )));
        }
        let stored = raw[16..20] == [0x00, 0xff, 0x02, 0x90]
            && f64_at(&raw, 0).is_finite()
            && f64_at(&raw, 0) > 0.0;
        let (x0, dx, y): (f64, f64, Vec<i32>) = if stored {
            (f64_at(&raw, 0), f64_at(&raw, 8), run_length(&raw[16..], n)?)
        } else {
            let u = lzf(&raw, size as usize)?;
            let y = (0..n)
                .map(|i| {
                    i32::from_le_bytes([u[16 + 4 * i], u[17 + 4 * i], u[18 + 4 * i], u[19 + 4 * i]])
                })
                .collect();
            (f64_at(&u, 0), f64_at(&u, 8), y)
        };
        let times: Vec<f64> = (0..n).map(|i| x0 + i as f64 * dx).collect();
        Ok((
            self.to_mz(r, &times)?,
            y.iter().map(|&v| v as f64).collect(),
        ))
    }
}

impl AgilentD {
    /// userParams of a spectrum of an LC/MS run: the collision energy and the fragmentor voltage of its record, when it says them.
    pub(crate) fn energies_of(&self, s: &crate::mzml::Spectrum) -> Vec<(String, String)> {
        let Some(r) =
            s.id.strip_prefix("scan=")
                .and_then(|n| n.parse::<usize>().ok())
                .and_then(|n| self.records.get(n.checked_sub(1)?))
                .filter(|r| format!("scan={}", r.id) == s.id)
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if r.collision_energy > 0.0 {
            out.push((
                "collision energy".to_string(),
                format!("{}", r.collision_energy),
            ));
        }
        if r.fragmentor > 0.0 {
            out.push((
                "fragmentor voltage".to_string(),
                format!("{}", r.fragmentor),
            ));
        }
        out
    }
}

impl Srm {
    pub(crate) fn add(&mut self, r: &ScanRec, mz: &[f64], it: &[f64]) {
        for (&q3, &y) in mz.iter().zip(it) {
            let (t, v) = self
                .traces
                .entry((thousandths(r.mz_of_interest), thousandths(q3), r.method_id))
                .or_default();
            t.push(r.time_min);
            v.push(y);
        }
    }

    /// The chromatograms, numbered from `first`; the id is `SRM SIC Q1=<q1> Q3=<q3>`, with ` method=<n>` added when two rows of the method share both.
    pub(crate) fn finish(self, first: usize) -> Vec<Chromatogram> {
        let mut shared: BTreeMap<(i64, i64), usize> = BTreeMap::new();
        for (q1, q3, _) in self.traces.keys() {
            *shared.entry((*q1, *q3)).or_default() += 1;
        }
        self.traces
            .into_iter()
            .enumerate()
            .map(|(k, ((q1, q3, method), (time, intensity)))| {
                let mut id = format!("SRM SIC Q1={} Q3={}", plain(q1), plain(q3));
                if shared.get(&(q1, q3)).is_some_and(|n| *n > 1) {
                    id += &format!(" method={method}");
                }
                Chromatogram {
                    index: first + k,
                    id,
                    kind: ChromKind::Srm,
                    q1: Some(q1 as f64 / 1000.0),
                    q3: Some(q3 as f64 / 1000.0),
                    unit: "number of detector counts".into(),
                    time,
                    intensity,
                }
            })
            .collect()
    }
}

impl AgilentD {
    /// All the SRM chromatograms of the run, read from the MRM visits.
    pub(crate) fn srm_chromatograms<R: ReadAt>(
        &self,
        peak: &mut R,
        first: usize,
    ) -> Result<Vec<Chromatogram>> {
        let mut srm = Srm::default();
        for r in self
            .records
            .iter()
            .filter(|r| self.is_mrm(r) && r.points > 0)
        {
            let (mz, it) = self.points_of(peak, r)?;
            srm.add(r, &mz, &it);
        }
        Ok(srm.finish(first))
    }
}

fn thousandths(v: f64) -> i64 {
    (v * 1000.0).round() as i64
}

fn plain(k: i64) -> String {
    format!("{}", k as f64 / 1000.0)
}

/// Times and intensities of one SRM chromatogram.
type Trace = (Vec<f64>, Vec<f64>);

/// The SRM chromatograms of the MRM visits: one per (Q1, Q3, `ScanMethodID`), the Q1 being `MzOfInterest` and the Q3 the m/z of a point of the visit.
#[derive(Default)]
pub(crate) struct Srm {
    traces: BTreeMap<(i64, i64, i32), Trace>,
}

#[cfg(test)]
#[path = "lcms_tests.rs"]
mod tests;
