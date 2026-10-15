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
const MASS_CAL_HEAD: usize = 72;
const MASS_CAL_REC: usize = 84;

fn bad(detail: String) -> Error {
    Error::new("err.file.agilent").with("detail", detail)
}

/// One default calibration of `DefaultMassCal.xml`.
#[derive(Debug, Clone, Default, PartialEq)]
struct DefaultCal {
    id: i32,
    a: f64,
    t0: f64,
    tmin: f64,
    tmax: f64,
    powers: Vec<u32>,
    c: Vec<f64>,
}

/// The mass calibration of a Q-TOF run: the defaults and the coefficients of each scan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Calibration {
    defaults: Vec<DefaultCal>,
    scans: Vec<Option<[f64; 10]>>,
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = s.find(open)? + open.len();
    Some(&s[a..a + s[a..].find(close)?])
}

fn defaults_of(xml: &str) -> Vec<DefaultCal> {
    let mut out = Vec::new();
    for part in xml.split("<DefaultCalibration ").skip(1) {
        let body = part.split("</DefaultCalibration>").next().unwrap_or("");
        let Some(id) = between(body, "DefaultCalibrationID=\"", "\"").and_then(|v| v.parse().ok())
        else {
            continue;
        };
        let mut cal = DefaultCal {
            id,
            ..Default::default()
        };
        for step in body.split("<Step ").skip(1) {
            let formula = between(step, "<CalibrationFormula>", "<").unwrap_or("");
            let flags: u32 = between(step, "<ValueUseFlags>", "<")
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0);
            let values: Vec<f64> = step
                .split("<Value Number=\"")
                .skip(1)
                .filter_map(|v| between(v, ">", "<")?.trim().parse().ok())
                .collect();
            match formula {
                "Traditional" if values.len() >= 2 => {
                    cal.a = values[0];
                    cal.t0 = values[1];
                }
                "Polynomial" if values.len() >= 3 => {
                    cal.tmin = values[0];
                    cal.tmax = values[1];
                    cal.c = values[2..].to_vec();
                    cal.powers = (0..32).filter(|k| flags >> k & 1 == 1).collect();
                }
                _ => {}
            }
        }
        out.push(cal);
    }
    out
}

impl Calibration {
    /// `mass_cal`: `MSMassCal.bin`, `default_cal`: `DefaultMassCal.xml` (BOM allowed); either may be missing.
    pub fn parse(mass_cal: Option<&[u8]>, default_cal: Option<&[u8]>) -> Calibration {
        let defaults = default_cal
            .map(|b| defaults_of(&String::from_utf8_lossy(b)))
            .unwrap_or_default();
        let scans = mass_cal
            .filter(|b| b.len() >= MASS_CAL_HEAD)
            .map(|b| {
                b[MASS_CAL_HEAD..]
                    .chunks_exact(MASS_CAL_REC)
                    .map(|r| {
                        let n = u32::from_le_bytes([r[0], r[1], r[2], r[3]]);
                        (n == 10).then(|| {
                            let mut v = [0f64; 10];
                            for (k, x) in v.iter_mut().enumerate() {
                                let o = 4 + k * 8;
                                *x = f64::from_le_bytes(r[o..o + 8].try_into().unwrap_or([0; 8]));
                            }
                            v
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Calibration { defaults, scans }
    }

    pub fn is_empty(&self) -> bool {
        self.defaults.is_empty()
    }

    /// m/z of the flight time `t` of the scan record number `index`, whose `CalibrationID` is `id`.
    fn mz(&self, id: i32, index: Option<usize>, t: f64) -> Option<f64> {
        let def = self
            .defaults
            .iter()
            .find(|d| d.id == id)
            .or(self.defaults.first())?;
        let own = index.and_then(|i| self.scans.get(i).copied().flatten());
        let (a, t0, tmin, tmax, c): (f64, f64, f64, f64, &[f64]) = match &own {
            Some(s) => (s[0], s[1], s[2], s[3], &s[4..]),
            None => (def.a, def.t0, def.tmin, def.tmax, &def.c[..]),
        };
        let tt = t.max(tmin).min(tmax);
        let corr: f64 = def
            .powers
            .iter()
            .zip(c)
            .map(|(p, c)| c * tt.powi(*p as i32))
            .sum();
        let x = a * (t - t0);
        Some(x * x - corr).filter(|v| v.is_finite())
    }
}

/// Run-length stream of the profile: `[.., i32 −first index, ..]` then integers of the current width; a negative value `v` skips `!v >> 2` points and sets the
/// width to `1 << (!v & 3)` bytes, a non-negative one is the intensity of the next point.
fn run_length(s: &[u8], n: usize) -> Result<Vec<i32>> {
    let first = s
        .get(4..8)
        .map(|b| -(i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as i64))
        .filter(|f| *f >= 0)
        .ok_or_else(|| bad("profile: no start index".into()))? as usize;
    let mut out = vec![0i32; n];
    let (mut pos, mut w, mut i) = (first, 4usize, 8usize);
    while i < s.len() {
        let Some(raw) = s.get(i..i + w) else { break };
        i += w;
        let v = match w {
            1 => raw[0] as i8 as i64,
            2 => i16::from_le_bytes([raw[0], raw[1]]) as i64,
            4 => i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as i64,
            _ => i64::from_le_bytes(raw.try_into().unwrap_or([0; 8])),
        };
        if v >= 0 {
            *out.get_mut(pos)
                .ok_or_else(|| bad("profile: a point past the end".into()))? =
                v.min(i32::MAX as i64) as i32;
            pos += 1;
        } else {
            let u = !v as u64;
            pos = pos.saturating_add((u >> 2) as usize);
            w = 1 << (u & 3);
        }
    }
    Ok(out)
}

/// LZF (liblzf format) of `s`, expected to give `size` bytes.
fn lzf(s: &[u8], size: usize) -> Result<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let mut i = 0;
    let end = || bad("profile: the LZF stream is cut".into());
    while i < s.len() && out.len() < size {
        let c = s[i] as usize;
        i += 1;
        if c < 32 {
            let run = c + 1;
            out.extend_from_slice(s.get(i..i + run).ok_or_else(end)?);
            i += run;
        } else {
            let mut len = c >> 5;
            if len == 7 {
                len += *s.get(i).ok_or_else(end)? as usize;
                i += 1;
            }
            let back = ((c & 31) << 8) + *s.get(i).ok_or_else(end)? as usize + 1;
            i += 1;
            let from = out
                .len()
                .checked_sub(back)
                .ok_or_else(|| bad("profile: the LZF reference is before the start".into()))?;
            for k in 0..len + 2 {
                let b = out[from + k];
                out.push(b);
            }
        }
    }
    if out.len() != size {
        return Err(bad(format!(
            "profile: {} bytes unpacked, {size} expected",
            out.len()
        )));
    }
    Ok(out)
}

fn f64_at(b: &[u8], at: usize) -> f64 {
    f64::from_le_bytes(b[at..at + 8].try_into().unwrap_or([0; 8]))
}

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

/// The SRM chromatograms of the MRM visits: one per (Q1, Q3, `ScanMethodID`), the Q1 being `MzOfInterest` and the Q3 the m/z of a point of the visit.
#[derive(Default)]
pub(crate) struct Srm {
    traces: BTreeMap<(i64, i64, i32), (Vec<f64>, Vec<f64>)>,
}

fn thousandths(v: f64) -> i64 {
    (v * 1000.0).round() as i64
}

fn plain(k: i64) -> String {
    format!("{}", k as f64 / 1000.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_length_skips_and_changes_width() {
        // header 4 bytes, −first = −2 (points 0 and 1 skipped), then width 4: 7; −(1<<2|1)−1: skip 1, width 2: 300
        let mut s = vec![0u8; 4];
        s.extend((-2i32).to_le_bytes());
        s.extend(7i32.to_le_bytes());
        s.extend((!(1i32 << 2 | 1)).to_le_bytes());
        s.extend(300i16.to_le_bytes());
        let y = run_length(&s, 6).unwrap();
        assert_eq!(y, vec![0, 0, 7, 0, 300, 0]);
    }

    #[test]
    fn lzf_literal_and_back_reference() {
        // literal run of 3 bytes, then a back reference of length 4 at distance 3
        let s = [2u8, 1, 2, 3, (2 << 5) | 0, 2];
        assert_eq!(lzf(&s, 7).unwrap(), vec![1, 2, 3, 1, 2, 3, 1]);
        assert!(lzf(&s, 9).is_err());
    }

    #[test]
    fn calibration_uses_the_scan_coefficients_before_the_default() {
        let xml = "<DefaultCalibration DefaultCalibrationID=\"1\"><Step Number=\"1\"><CalibrationFormula>Traditional</CalibrationFormula><ValueUseFlags>0</ValueUseFlags><Values><Value Number=\"1\">0.5</Value><Value Number=\"2\">2</Value></Values></Step><Step Number=\"2\"><CalibrationFormula>Polynomial</CalibrationFormula><ValueUseFlags>1</ValueUseFlags><Values><Value Number=\"1\">0</Value><Value Number=\"2\">100</Value><Value Number=\"3\">3</Value></Values></Step></DefaultCalibration>";
        let c = Calibration::parse(None, Some(xml.as_bytes()));
        // (0.5·(10−2))² − 3·10⁰ = 16 − 3
        assert_eq!(c.mz(1, None, 10.0), Some(13.0));
        let mut cal = vec![0u8; MASS_CAL_HEAD];
        cal.extend(10u32.to_le_bytes());
        for v in [1.0f64, 0.0, 0.0, 100.0, 1.0] {
            cal.extend(v.to_le_bytes());
        }
        cal.extend([0u8; 40]);
        let c = Calibration::parse(Some(&cal), Some(xml.as_bytes()));
        assert_eq!(c.mz(1, Some(0), 10.0), Some(99.0));
        assert_eq!(c.mz(1, Some(5), 10.0), Some(13.0));
    }
}
