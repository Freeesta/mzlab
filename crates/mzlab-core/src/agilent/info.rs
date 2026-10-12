//! The small side files of a `.D` folder: `AcqData/MSTS.xml` (time segments), `Devices.xml`, `Contents.xml` and the strings of `MSTree2.bin`.
//! Every field is optional: a missing or odd file leaves the field empty and never stops the reading.

use crate::xmltree::{self, El};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Segment {
    /// Position in `MSTS.xml`, from 1 (the file's own `TimeSegmentID` attribute is not read).
    pub id: u32,
    pub start_min: f64,
    pub end_min: f64,
    /// Counted by `MSTS.xml` by time: a scan stamped just after the end can still carry the previous segment in its record.
    pub scans: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Device {
    pub name: String,
    pub model: String,
    pub serial: String,
    pub firmware: String,
    pub driver: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Contents {
    /// ISO 8601 with offset, as the file writes it.
    pub acquired: String,
    pub instrument: String,
    /// 0 unspecified, 1 none, 2 GC, 3 LC, 4 CE (the comment in the file).
    pub separation: u8,
    pub software: String,
}

/// What `MSTree2.bin` says in words: the same for every scan of the run when there is one entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tree {
    pub device: String,
    /// "Scan", "SelectedIon", ...
    pub scan_type: String,
    /// "Positive" or "Negative".
    pub polarity: String,
    pub scans: usize,
}

fn root(bytes: &[u8]) -> Option<El> {
    let t = String::from_utf8_lossy(bytes);
    xmltree::parse(t.trim_start_matches('\u{feff}'))
}

fn num(e: &El, name: &str) -> f64 {
    e.findtext(name)
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0.0)
}

pub fn segments(xml: &[u8]) -> Vec<Segment> {
    let Some(r) = root(xml) else {
        return Vec::new();
    };
    r.findall("TimeSegment")
        .enumerate()
        .map(|(i, e)| Segment {
            id: i as u32 + 1,
            start_min: num(e, "StartTime"),
            end_min: num(e, "EndTime"),
            scans: num(e, "NumOfScans") as usize,
        })
        .collect()
}

pub fn devices(xml: &[u8]) -> Vec<Device> {
    let Some(r) = root(xml) else {
        return Vec::new();
    };
    let t = |e: &El, n: &str| e.findtext(n).unwrap_or("").trim().to_string();
    r.findall("Device")
        .map(|e| Device {
            name: t(e, "Name"),
            model: t(e, "ModelNumber"),
            serial: t(e, "SerialNumber"),
            firmware: t(e, "FirmwareVersion"),
            driver: t(e, "DriverVersion"),
        })
        .collect()
}

pub fn contents(xml: &[u8]) -> Option<Contents> {
    let r = root(xml)?;
    let t = |n: &str| r.findtext(n).unwrap_or("").trim().to_string();
    Some(Contents {
        acquired: t("AcquiredTime"),
        instrument: t("InstrumentName"),
        separation: t("SeparationTechnique").parse().unwrap_or(0),
        software: t("AcqSoftwareVersion"),
    })
}

/// Length-prefixed text (`u8` length, then the bytes) at `at`.
fn text(b: &[u8], at: usize) -> Option<(String, usize)> {
    let n = *b.get(at)? as usize;
    let s = b.get(at + 1..at + 1 + n)?;
    std::str::from_utf8(s)
        .ok()
        .map(|s| (s.to_string(), at + 1 + n))
}

/// The layout seen in the three runs: `u32` 86, `u32` 5, texts (device, separation, instrument), four `f64` (first and last scan time, lowest and
/// highest m/z), the bytes `00 01 <u32 1>`, the scan type, `u32` 1, the polarity, `u32` scan count. Anything else gives `None`.
pub fn tree(b: &[u8]) -> Option<Tree> {
    let u32_at = |o: usize| {
        b.get(o..o + 4)
            .map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]))
    };
    if u32_at(0)? != 86 || u32_at(4)? != 5 {
        return None;
    }
    let (device, p) = text(b, 8)?;
    let (_, p) = text(b, p)?;
    let (_, p) = text(b, p)?;
    let p = p + 32;
    if b.get(p..p + 6)? != [0, 1, 1, 0, 0, 0] {
        return None;
    }
    let (scan_type, p) = text(b, p + 6)?;
    if u32_at(p)? != 1 {
        return None;
    }
    let (polarity, p) = text(b, p + 4)?;
    Some(Tree {
        device,
        scan_type,
        polarity,
        scans: u32_at(p)? as usize,
    })
}
