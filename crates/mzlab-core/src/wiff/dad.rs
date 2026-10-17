//! Optical DAD / PDA detector streams (`SampleSubtree/SampleN/Devices/Device_K/DAD/`) and real-time device analog logs (`Channel`, `DevRealTimeHeader`, `DevData`),
//! plus high-resolution TOF / SCIEX OS sample streams (`Wiff2SampleInfo`, `DDERealTimeData`, `TDCInfo`, `TDCStatistics`, `TOFCalibrationData`, `Itc`, and `.wiff.scan`
//! `0xffffffff` TDC blocks). Spec: `docs/agenti/wiff_specifica_completa.md`.

use super::bad;
use crate::error::Result;
use crate::mzml::{ChromKind, Chromatogram};
use crate::ole::Ole;

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn f32_at(b: &[u8], at: usize) -> Option<f32> {
    u32_at(b, at).map(f32::from_bits).filter(|v| v.is_finite())
}

fn f64_at(b: &[u8], at: usize) -> Option<f64> {
    b.get(at..at + 8)
        .map(|s| f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]))
        .filter(|v| v.is_finite())
}

/// Complete 3D optical DAD / PDA dataset of one sample (`DADSampleHeader`, `DADIdx`, `DADRealTimeData`).
#[derive(Debug, Clone, PartialEq)]
pub struct DadData {
    pub time_min: Vec<f64>,
    pub twc_mau: Vec<f64>,
    pub wavelengths_nm: Vec<f64>,
    pub wl_start_nm: f64,
    pub wl_stop_nm: f64,
    pub wl_step_nm: f64,
    pub duration_min: f64,
    /// Row-major 2D absorbance matrix in $\text{mAU}$ ($N_t \times N_\lambda$ elements).
    pub matrix_mau: Vec<f32>,
}

impl DadData {
    pub fn read(ole: &Ole, sample: u32) -> Option<DadData> {
        read_dad(ole, sample)
    }
    pub fn points(&self) -> usize {
        self.time_min.len()
    }
    pub fn channels(&self) -> usize {
        self.wavelengths_nm.len()
    }
    pub fn spectrum_at(&self, t_idx: usize) -> Option<&[f32]> {
        let n_ch = self.channels();
        let start = t_idx.checked_mul(n_ch)?;
        self.matrix_mau.get(start..start + n_ch)
    }
    pub fn twc_chromatogram(&self, index: usize) -> Chromatogram {
        Chromatogram {
            index,
            id: "DAD TWC".into(),
            kind: ChromKind::Pda,
            q1: None,
            q3: None,
            unit: "mAU".into(),
            time: self.time_min.clone(),
            intensity: self.twc_mau.clone(),
        }
    }
    pub fn wavelength_chromatogram(&self, nm: f64, bandwidth_nm: Option<f64>, index: usize) -> Option<Chromatogram> {
        dad_wl_chrom(self, nm, bandwidth_nm, index)
    }
}

fn read_dad(ole: &Ole, sample: u32) -> Option<DadData> {
    let prefix = format!("SampleSubtree/Sample{sample}/Devices/");
    let h_path = ole.paths_list().find(|p| p.starts_with(&prefix) && p.ends_with("/DAD/DADSampleHeader"))?.to_string();
    let h = ole.read(&h_path)?;
    let (n_pts, n_ch) = (u32_at(&h, 34)? as usize, u32_at(&h, 38)? as usize);
    let (dur, wl0, wl1, step) = (f32_at(&h, 46)? as f64, f32_at(&h, 50)? as f64, f32_at(&h, 54)? as f64, f32_at(&h, 58)? as f64);
    if n_pts == 0 || n_ch == 0 || step <= 0.0 || wl1 < wl0 || ((wl1 - wl0) / step).round() as usize + 1 != n_ch {
        return None;
    }
    let idx = ole.read(&h_path.replace("DADSampleHeader", "DADIdx"))?;
    if idx.len() < 32 + 8 * n_pts {
        return None;
    }
    let (mut time_min, mut twc_mau) = (Vec::with_capacity(n_pts), Vec::with_capacity(n_pts));
    for i in 0..n_pts {
        time_min.push(f32_at(&idx, 32 + 8 * i)? as f64);
        twc_mau.push(f32_at(&idx, 36 + 8 * i)? as f64);
    }
    let rtd = ole.read(&h_path.replace("DADSampleHeader", "DADRealTimeData"))?;
    let total = n_pts.checked_mul(n_ch)?;
    let matrix_mau: Vec<f32> = rtd
        .get(32..32 + 4 * total)?
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let wavelengths_nm = (0..n_ch).map(|j| wl0 + j as f64 * step).collect();
    Some(DadData { time_min, twc_mau, wavelengths_nm, wl_start_nm: wl0, wl_stop_nm: wl1, wl_step_nm: step, duration_min: dur, matrix_mau })
}

fn dad_wl_chrom(dad: &DadData, nm: f64, bandwidth_nm: Option<f64>, index: usize) -> Option<Chromatogram> {
    let n_ch = dad.channels();
    if n_ch == 0 {
        return None;
    }
    let half = bandwidth_nm.unwrap_or(0.0).max(0.0) * 0.5;
    let cols: Vec<usize> = if half > 0.0 {
        dad.wavelengths_nm.iter().enumerate().filter(|(_, &w)| (w - nm).abs() <= half + 1e-6).map(|(j, _)| j).collect()
    } else {
        let j = ((nm - dad.wl_start_nm) / dad.wl_step_nm).round();
        if j < 0.0 || (j as usize) >= n_ch {
            return None;
        }
        vec![j as usize]
    };
    if cols.is_empty() {
        return None;
    }
    let inv = 1.0 / cols.len() as f64;
    let intensity = (0..dad.points())
        .map(|i| {
            let row = dad.matrix_mau.get(i * n_ch..(i + 1) * n_ch).unwrap_or_default();
            cols.iter().filter_map(|&j| row.get(j)).map(|&v| f64::from(v)).sum::<f64>() * inv
        })
        .collect();
    Some(Chromatogram {
        index,
        id: format!("DAD {:.0} nm", nm),
        kind: ChromKind::Pda,
        q1: None,
        q3: None,
        unit: "mAU".into(),
        time: dad.time_min.clone(),
        intensity,
    })
}

/// Real-time auxiliary device channel (pressure, flow rate, oven temperature) from `Devices/Device_K/{Channel, DevRealTimeHeader, DevData}`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceChannel {
    pub device_index: u32,
    pub channel_index: u16,
    pub name: String,
    pub unit: String,
    pub step_s: f64,
    pub scale_min: f32,
    pub scale_max: f32,
    pub time_min: Vec<f64>,
    pub values: Vec<f64>,
}

/// Reads all real-time auxiliary device channels (`Column Pressure`, `Pump A Flowrate`, `Oven: Temperature`, etc.) of `sample`.
pub fn device_channels(ole: &Ole, sample: u32) -> Vec<DeviceChannel> {
    let mut out = Vec::new();
    for dev in 0..8u32 {
        let base = format!("SampleSubtree/Sample{sample}/Devices/Device_{dev}/");
        let Some(ch) = ole.read(&format!("{base}Channel")) else {
            continue;
        };
        let Some(n_ch) = u16_at(&ch, 36).map(usize::from).filter(|&n| n > 0) else {
            continue;
        };
        let mut metas = Vec::with_capacity(n_ch);
        let mut p = 38usize;
        for _ in 0..n_ch {
            let Some(nlen) = u16_at(&ch, p + 4).map(usize::from) else {
                break;
            };
            if p + 6 + nlen + 2 > ch.len() {
                break;
            }
            let name = utf16_str(&ch[p + 6..p + 6 + nlen]);
            p += 6 + nlen;
            let Some(ulen) = u16_at(&ch, p).map(usize::from) else {
                break;
            };
            if p + 2 + ulen + 22 > ch.len() {
                break;
            }
            let unit = utf16_str(&ch[p + 2..p + 2 + ulen])
                .trim_matches(['(', ')'])
                .to_string();
            p += 2 + ulen;
            let Some(dt_s) = f64_at(&ch, p) else { break };
            let idx = u16_at(&ch, p + 8).unwrap_or(0);
            let smin = f32_at(&ch, p + 10).unwrap_or(0.0);
            let smax = f32_at(&ch, p + 14).unwrap_or(0.0);
            p += 22;
            metas.push((idx, name, unit, dt_s, smin, smax));
        }
        if metas.len() != n_ch {
            continue;
        }
        let total = ole
            .read(&format!("{base}DevRealTimeHeader"))
            .and_then(|h| u32_at(&h, 32))
            .unwrap_or(0) as usize;
        let data = ole.read(&format!("{base}DevData")).unwrap_or_default();
        let n_pts = if total >= n_ch && data.len() >= 32 + 8 * total {
            total / n_ch
        } else {
            0
        };
        for (c, (idx, name, unit, dt_s, scale_min, scale_max)) in metas.into_iter().enumerate() {
            let mut time_min = Vec::with_capacity(n_pts);
            let mut values = Vec::with_capacity(n_pts);
            for k in 0..n_pts {
                if let Some(v) = f64_at(&data, 32 + 8 * (k * n_ch + c)) {
                    time_min.push((k as f64 * dt_s) / 60.0);
                    values.push(v);
                }
            }
            out.push(DeviceChannel {
                device_index: dev,
                channel_index: idx,
                name,
                unit,
                step_s: dt_s,
                scale_min,
                scale_max,
                time_min,
                values,
            });
        }
    }
    out
}

fn utf16_str(b: &[u8]) -> String {
    let u: Vec<u16> = b
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&u)
        .trim_end_matches('\0')
        .to_string()
}

/// One 32-byte IDA/DDA dynamic precursor selection entry from `SampleSubtree/SampleN/DDERealTimeData`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DdeEntry {
    pub cycle: usize,
    pub experiment: usize,
    /// Dynamically triggered precursor $m/z$ (`0.0` when the dependent slot was not triggered).
    pub precursor_mz: f64,
    /// Triggered MS1 survey peak apex intensity (counts) on SCIEX OS / TripleTOF.
    pub precursor_intensity: f64,
    pub charge_state: u16,
    pub flags: u16,
}

/// Reads `SampleSubtree/SampleN/DDERealTimeData` (32 bytes per dependent scan slot `cycle * (n_exp - 1) + (exp - 1)`).
pub fn dde_records(ole: &Ole, sample: u32, n_exp: usize) -> Vec<DdeEntry> {
    let Some(b) = ole.read(&format!("SampleSubtree/Sample{sample}/DDERealTimeData")) else {
        return Vec::new();
    };
    let dep = n_exp.saturating_sub(1);
    if dep == 0 || b.len() < 32 {
        return Vec::new();
    }
    let n = (b.len() - 32) / 32;
    (0..n)
        .map(|k| {
            let at = 32 + 32 * k;
            DdeEntry {
                cycle: k / dep,
                experiment: (k % dep) + 1,
                precursor_mz: f64_at(&b, at).unwrap_or(0.0),
                precursor_intensity: f64_at(&b, at + 16).unwrap_or(0.0),
                charge_state: u16_at(&b, at + 24).unwrap_or(0),
                flags: u16_at(&b, at + 26).unwrap_or(0),
            }
        })
        .collect()
}

/// `Wiff2SampleInfo` (`WiffNSampleInfo`, 126 bytes): links `.wiff` sample `N` to its SCIEX OS `.wiff2` acquisition GUID and timestamp ID.
#[derive(Debug, Clone, PartialEq)]
pub struct Wiff2SampleInfo {
    pub sample_number: u32,
    pub sample_guid: [u8; 16],
    pub link_id: String,
    pub batch_guid: [u8; 16],
}

pub fn wiff2_sample_info(ole: &Ole, sample: u32) -> Option<Wiff2SampleInfo> {
    let prefix = format!("SampleSubtree/Sample{sample}/");
    let path = ole
        .paths_list()
        .find(|p| p.starts_with(&prefix) && p.ends_with("SampleInfo"))?;
    let b = ole.read(path)?;
    if b.len() < 126 {
        return None;
    }
    let sample_number = u32_at(&b, 32)?;
    let sample_guid: [u8; 16] = b.get(36..52)?.try_into().ok()?;
    let len = (u16_at(&b, 52)? as usize).min(56);
    let link_id = String::from_utf8_lossy(b.get(54..54 + len)?).to_string();
    let batch_guid: [u8; 16] = b.get(110..126)?.try_into().ok()?;
    Some(Wiff2SampleInfo {
        sample_number,
        sample_guid,
        link_id,
        batch_guid,
    })
}

/// High-resolution TOF calibration and TDC metadata (`TDCInfo`, `TOFCalibrationData`, `Itc`, `TDCStatistics`).
#[derive(Debug, Clone, PartialEq)]
pub struct TofSampleData {
    /// Fundamental TDC tick period in nanoseconds (`0.026041666666666668` ns $= 1 / 38.4\text{ GHz}$).
    pub tdc_bin_time_ns: f64,
    pub initial_a: f64,
    pub initial_t0: f64,
    /// Per-scan `(cal_a, cal_t0, flags)` from `TOFCalibrationData`.
    pub scan_calibration: Vec<(f64, f64, u32)>,
    /// Per-scan Ion Transmission Control (`Itc`) scaling factor.
    pub itc: Vec<f64>,
    /// Number of 20-byte records in `TDCStatistics`.
    pub tdc_statistics_count: usize,
}

impl TofSampleData {
    pub fn read(ole: &Ole, sample: u32) -> Option<TofSampleData> {
        let base = format!("SampleSubtree/Sample{sample}/");
        let tdc = ole.read(&format!("{base}TDCInfo"))?;
        let tdc_bin_time_ns = f64_at(&tdc, 32)?;
        let cal = ole.read(&format!("{base}TOFCalibrationData"))?;
        let (initial_a, initial_t0, n_scans) = (
            f64_at(&cal, 32)?,
            f64_at(&cal, 40)?,
            u32_at(&cal, 52)? as usize,
        );
        if cal.len() < 56 + 20 * n_scans {
            return None;
        }
        let scan_calibration = (0..n_scans)
            .filter_map(|i| {
                let at = 56 + 20 * i;
                Some((
                    f64_at(&cal, at)?,
                    f64_at(&cal, at + 8)?,
                    u32_at(&cal, at + 16)?,
                ))
            })
            .collect();
        let itc = ole
            .read(&format!("{base}Itc"))
            .map(|b| {
                (0..b.len().saturating_sub(32) / 8)
                    .filter_map(|i| f64_at(&b, 32 + 8 * i))
                    .collect()
            })
            .unwrap_or_default();
        let tdc_statistics_count = ole
            .read(&format!("{base}TDCStatistics"))
            .and_then(|b| u32_at(&b, 32))
            .unwrap_or(0) as usize;
        Some(TofSampleData {
            tdc_bin_time_ns,
            initial_a,
            initial_t0,
            scan_calibration,
            itc,
            tdc_statistics_count,
        })
    }

    /// Converts a TDC tick index `bin` into $m/z$ using scan `scan_idx`'s `(a, t0)` calibration: $m/z = ((\text{bin} \times \Delta t_{\text{TDC}} - t_0) \times a)^2$.
    pub fn bin_to_mz(&self, scan_idx: usize, bin: u32) -> f64 {
        let (a, t0) = self
            .scan_calibration
            .get(scan_idx)
            .map_or((self.initial_a, self.initial_t0), |&(a, t0, _)| (a, t0));
        let root = (bin as f64 * self.tdc_bin_time_ns - t0) * a;
        root * root
    }
}

/// Decodes a `.wiff.scan` TOF `0xffffffff` TDC block (`size` bytes starting at `block`), returning `(bin, count)` pairs.
/// Verifies the `0xffffffff` marker; every step advances `curr_bin` by multiples of 4 TDC ticks.
pub fn decode_tof_block(block: &[u8]) -> Result<Vec<(u32, u32)>> {
    if block.len() < 9 || block[..4] != [0xff; 4] {
        return Err(bad("not a TOF 0xffffffff block"));
    }
    let mut curr_bin = u32_at(block, 4).unwrap_or(0);
    let mut pos = 9usize;
    let mut pts = Vec::new();
    while pos < block.len() {
        let b = block[pos];
        if b == 0xff {
            break;
        } else if (0x81..=0xfb).contains(&b) {
            curr_bin = curr_bin.wrapping_add(4 * u32::from(b - 0x80));
            pos += 1;
        } else if b == 0xfc && pos + 2 <= block.len() {
            curr_bin = curr_bin.wrapping_add(4 * u32::from(block[pos + 1]));
            pos += 2;
        } else if b == 0xfd && pos + 3 <= block.len() {
            let z = u32::from(u16::from_le_bytes([block[pos + 1], block[pos + 2]]));
            curr_bin = curr_bin.wrapping_add(4 * z);
            pos += 3;
        } else if b == 0xfe && pos + 5 <= block.len() {
            let z = u32_at(block, pos + 1).unwrap_or(0);
            curr_bin = curr_bin.wrapping_add(4 * z);
            pos += 5;
        } else if (0x01..=0x7b).contains(&b) {
            pts.push((curr_bin, u32::from(b)));
            curr_bin = curr_bin.wrapping_add(4);
            pos += 1;
        } else if b == 0x7c && pos + 2 <= block.len() {
            pts.push((curr_bin, u32::from(block[pos + 1])));
            curr_bin = curr_bin.wrapping_add(4);
            pos += 2;
        } else if b == 0x7d && pos + 3 <= block.len() {
            let c = u32::from(u16::from_le_bytes([block[pos + 1], block[pos + 2]]));
            pts.push((curr_bin, c));
            curr_bin = curr_bin.wrapping_add(4);
            pos += 3;
        } else if b == 0x7e && pos + 5 <= block.len() {
            let c = u32_at(block, pos + 1).unwrap_or(0);
            pts.push((curr_bin, c));
            curr_bin = curr_bin.wrapping_add(4);
            pos += 5;
        } else {
            return Err(bad("invalid TOF TDC token"));
        }
    }
    Ok(pts)
}
