//! Reader of SCIEX Analyst `.wiff` + `.wiff.scan` pairs (3200 QTRAP), in pure Rust on top of `ole`: the container gives the index of each
//! sample (`idx`) and the method, the `.scan` gives the points (`scan`). What is and is not known of the format: `docs/agenti/wiff.md`.
//! The container layout was first studied in OpenSXRaw (Sigilweaver, Apache-2.0); the blocks of the 3200 differ from it and
//! were decoded against ProteoWizard mzML of the same files. The code here is written anew.
//! The scans are PROFILE data as the file holds them (no peak picking): `Spectrum::profile` is true.

pub mod idx;
mod run;
pub mod scan;
mod window;

use crate::error::{Error, Result};
use crate::methodinfo::{self, Method};
use crate::ole::Ole;

const SAMPLES: &str = "SampleSubtree/";
/// Offsets inside `ExperimentHeader`: the precursor m/z (f64) and the scan-type code (u32).
const HEADER_PRECURSOR: usize = 42;
const HEADER_CODE: usize = 122;
/// Scan-type codes seen in the headers (full scan, product-ion scan); anything else is read as a full scan.
const CODE_PRODUCT: u32 = 11;

/// One acquisition experiment of the period (the scans of a cycle alternate between them).
#[derive(Debug, Clone, Default)]
pub struct Experiment {
    pub code: Option<u32>,
    /// 1 for a full scan (and unknown types), 2 for a product-ion scan.
    pub level: u8,
    pub precursor: Option<f64>,
}

/// A block of the index placed in the `.wiff.scan`.
#[derive(Debug, Clone, Copy)]
pub struct Record {
    /// Start of the payload and of the next one (the end of the data of this block).
    pub at: u64,
    pub end: u64,
    pub size: u32,
    pub time_min: f64,
    pub tic: f64,
    pub cycle: usize,
    pub experiment: usize,
}

/// One sample (injection) of a `.wiff`.
pub struct Wiff {
    pub sample: u32,
    pub records: Vec<Record>,
    pub experiments: Vec<Experiment>,
    /// +1, -1, or 0 when the method has no ion spray voltage.
    pub polarity: i8,
    pub method: Option<Method>,
    /// `Component ID` of the mass spectrometer in the Log, and the creation time of the file: the header of the mzML.
    pub model: Option<String>,
    pub created: Option<String>,
    /// SHA-1 (hex) of the `.wiff`, for the `sourceFile` of the mzML; the caller that has the bytes sets it (`Ole::sha1_hex`).
    pub wiff_sha1: Option<String>,
    scan_len: u64,
}

fn bad(detail: &str) -> Error {
    Error::new("err.file.wiff").with("detail", detail)
}

/// The container of a `.wiff`.
pub fn container(wiff: Vec<u8>, name: &str) -> Result<Ole> {
    Ole::open(wiff, name).map_err(|_| Error::new("err.file.notWiff").with("name", name))
}

/// The numbers of the samples that have scans, in order.
pub fn samples(ole: &Ole) -> Vec<u32> {
    let mut v: Vec<u32> = ole.read(&format!("{SAMPLES}SampleIdxTable")).map(|t| idx::sections(&t).into_iter().map(|s| s.0).collect()).unwrap_or_default();
    v.retain(|n| ole.has(&format!("{SAMPLES}Sample{n}/Idx")));
    v.sort_unstable();
    v.dedup();
    v
}

fn experiments(ole: &Ole, sample: u32) -> Vec<Experiment> {
    let mut out = Vec::new();
    while let Some(d) = ole.read(&format!("MethodSubtree/Method{sample}/DeviceMethod0/Period0/Experiment{}/ExperimentHeader", out.len())) {
        let code = d.get(HEADER_CODE..HEADER_CODE + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let prec = d.get(HEADER_PRECURSOR..HEADER_PRECURSOR + 8).map(|b| f64::from_le_bytes(b.try_into().unwrap_or([0; 8]))).filter(|p| p.is_finite() && *p > 0.0);
        let product = code == Some(CODE_PRODUCT);
        out.push(Experiment { code, level: if product { 2 } else { 1 }, precursor: if product { prec } else { None } });
    }
    out
}

fn polarity(method: &Option<Method>) -> i8 {
    method.as_ref().and_then(|m| m.source.iter().find(|p| p.id == "IS")).map_or(0, |p| if p.value > 0.0 { 1 } else if p.value < 0.0 { -1 } else { 0 })
}

impl Wiff {
    /// Sample `sample` of the container. `scan_len` is the size of the `.wiff.scan`, `None` when that file is missing
    /// (`err.file.wiffNoScan`).
    pub fn open(ole: &Ole, name: &str, scan_len: Option<u64>, sample: u32) -> Result<Wiff> {
        let scan_len = scan_len.ok_or_else(|| Error::new("err.file.wiffNoScan").with("name", name))?;
        let table = ole.read(&format!("{SAMPLES}SampleIdxTable")).ok_or_else(|| bad("no sample table"))?;
        let section = idx::sections(&table).into_iter().find(|s| s.0 == sample).ok_or_else(|| bad("no such sample"))?.1;
        let entries = idx::parse(&ole.read(&format!("{SAMPLES}Sample{sample}/Idx")).ok_or_else(|| bad("no Idx"))?);
        let experiments = experiments(ole, sample);
        let n_exp = experiments.len().max(1);
        let records = entries
            .iter()
            .enumerate()
            .map(|(k, e)| Record {
                at: section + e.offset as u64 + scan::PAYLOAD_SKIP,
                end: entries.get(k + 1).map_or(scan_len, |n| section + n.offset as u64 + scan::PAYLOAD_SKIP),
                size: e.size,
                time_min: e.time_ms / 60000.0,
                tic: e.tic,
                cycle: k / n_exp,
                experiment: k % n_exp,
            })
            .collect();
        let method = methodinfo::method_of_sample(ole, sample);
        let (model, created) = crate::metodo_wiff::run_facts(ole, sample);
        Ok(Wiff { sample, records, experiments, polarity: polarity(&method), method, model, created, wiff_sha1: None, scan_len })
    }
}
