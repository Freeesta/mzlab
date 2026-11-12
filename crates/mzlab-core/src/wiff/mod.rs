//! Reader of SCIEX Analyst `.wiff` + `.wiff.scan` pairs (3200 QTRAP), in pure Rust on top of `ole`: the container gives the index of each
//! sample (`idx`) and the method, the `.scan` gives the points (`scan`). What is and is not known of the format: `docs/agenti/wiff.md`.
//! The container layout was first studied in OpenSXRaw (Sigilweaver, Apache-2.0); the blocks of the 3200 differ from it and
//! were decoded against ProteoWizard mzML of the same files. The code here is written anew.
//! The scans are PROFILE data as the file holds them (no peak picking): `Spectrum::profile` is true.

mod bpc;
mod chromatograms;
mod devices;
pub mod dad;
#[cfg(feature = "mappa-dad")]
pub mod dad_map;
#[cfg(feature = "mappa-dad")]
pub mod dad_uv;
mod dde;
pub mod idx;
mod losses;
mod mass_check;
mod method_name;
pub mod modes;
pub mod names;
pub mod reader;
mod reader_extra;
mod run;
#[cfg(not(feature = "estesi"))]
pub mod scan;
mod unindexed;
mod window;

use crate::error::{Error, Result};
use crate::methodinfo::{self, Method};
use crate::ole::Ole;

pub(super) const SAMPLES: &str = "SampleSubtree/";
/// Offsets inside `ExperimentHeader`: the precursor m/z (f64) and the scan-type code (u32).
const HEADER_PRECURSOR: usize = 42;
const HEADER_CODE: usize = 122;
const HEADER_POLARITY: usize = 86; // the polarity in `ExperimentHeader`
use modes::{Output, CODE_PRODUCT};
/// The method that holds the experiments of `sample`: its own `MethodN`, or, when the file stores fewer methods than it has samples (a batch run with one
/// method), the nearest one before it if it acquires what the sample's own blocks hold (scans or MRM); otherwise none is borrowed.
pub(crate) fn method_no(ole: &Ole, sample: u32) -> u32 {
    if let Some(k) = method_name::by_name(ole, sample) {
        return k;
    }
    // one pass over the paths, not one search per method number below `sample`: a table of 2 000 samples was 2 000 x 10 000 path searches
    let nearest = ole
        .paths_list()
        .filter_map(|p| {
            let d = p
                .strip_prefix("MethodSubtree/Method")?
                .strip_suffix("/DeviceMethod0/Period0/Experiment0/ExperimentHeader")?;
            d.parse::<u32>().ok().filter(|k| k.to_string() == d)
        })
        .filter(|&k| (1..=sample.min(10_000)).contains(&k))
        .max();
    match nearest {
        Some(k) if k == sample || borrow_agrees(ole, sample, k) => k,
        _ => sample,
    }
}

/// Whether the method `k` fits the blocks of `sample`: an MRM sample never takes a scan method, nor a scan sample an MRM one. The Idx rows say which
/// blocks are scans (a point count above 0); a sample without rows takes the method as it is.
fn borrow_agrees(ole: &Ole, sample: u32, k: u32) -> bool {
    let rows = ole
        .read(&format!("{SAMPLES}Sample{sample}/Idx"))
        .map(|b| idx::parse(&b))
        .unwrap_or_default();
    if rows.is_empty() {
        return true;
    }
    let own_scans = rows.iter().any(|r| r.points > 0.0);
    let method_scans = experiments(ole, k).iter().any(|e| {
        e.code
            .and_then(modes::mode)
            .is_some_and(|m| m.output == Output::Spettro)
    });
    own_scans == method_scans
}

impl Wiff {
    /// The method says experiment `e` acquires scans: a block of it that is not a readable scan is not an MRM block.
    pub(crate) fn scan_experiment(&self, e: usize) -> bool {
        let code = self.experiments.get(e).and_then(|x| x.code);
        modes::nativo(code)
            && code
                .and_then(modes::mode)
                .is_some_and(|m| m.output == Output::Spettro)
    }

    /// The engine (no `estesi`) reads the native scan types only: a sample with an experiment of another code is refused when its data are read, never
    /// taken for level-1 scans (the `sciex` module reads it; `Wiff::variante` routes the pair there). The other samples of the pair stay readable.
    pub(crate) fn readable(&self) -> Result<()> {
        if let Some(e) = &self.failure {
            return Err(e.clone());
        }
        #[cfg(not(feature = "estesi"))]
        if let Some((k, e)) = self
            .experiments
            .iter()
            .enumerate()
            .find(|(_, e)| e.code.is_some() && !modes::nativo(e.code))
        {
            return Err(scan_type_error(e.code.unwrap_or(0), k + 1));
        }
        Ok(())
    }

    #[cfg(not(feature = "estesi"))]
    /// A row of a SIM experiment without values: left out, so that it does not cut the columns of the ions.
    pub(crate) fn empty_sim_row(&self, r: &Record) -> bool {
        r.size == 0 && modes::sim(self.experiments.get(r.experiment).and_then(|x| x.code))
    }
}

/// Half of the 0.7 m/z (FWHM) that Q1 transmits at unit resolution, the only setting the method is assumed to hold (the resolution is not read per experiment).
const UNIT_HALF_WIDTH: f64 = 0.35;

/// One acquisition experiment of the period (the scans of a cycle alternate between them).
#[derive(Debug, Clone, Default)]
pub struct Experiment {
    pub code: Option<u32>,
    /// 1 for a full scan (and unknown types), 2 for a product-ion scan.
    pub level: u8,
    pub precursor: Option<f64>,
    /// The first precursor of an MS3 experiment (`precursor` is the second one, isolated from the fragments of the first).
    pub parent: Option<f64>,
    /// The fixed mass of a precursor-ion scan (the product m/z Q3 stays on) or of a neutral-loss scan (the loss, Da): not a precursor.
    pub fixed: Option<f64>,
    /// Collision energy (eV) of a product-ion experiment, when the method gives one value for it.
    pub ce: Option<f64>,
    /// Half-width (m/z) of the isolation window of a product-ion experiment.
    pub iso_half: Option<f64>,
    /// Detection window (s) of a scheduled MRM experiment (sMRM), when the method says it is scheduled.
    pub window_s: Option<u32>,
    /// Polarity of the scans from the header (1 positive, -1 negative); `None` when the header does not say.
    pub polarity: Option<i8>,
}

/// A block of the index placed in the `.wiff.scan`.
#[derive(Debug, Clone, Copy)]
pub struct Record {
    /// Start of the payload and of the next one (the end of the data of this block).
    pub at: u64,
    pub end: u64,
    /// Length of the payload as the index row tells it.
    pub size: u32,
    pub time_min: f64,
    pub tic: f64,
    pub cycle: usize,
    pub experiment: usize,
    /// The point count of the scan as the index row tells it (0 for an MRM block or an empty scan, NaN when the row is not known or holds none: IDA).
    pub points: f64,
    /// The precursor m/z and charge of a dependent (IDA) scan from its `DDERealTimeData` row (`dde.rs`); `None` for the other scans.
    pub precursor: Option<f64>,
    pub charge: Option<u8>,
}

impl Record {
    /// The block does not lie whole in a `.wiff.scan` of `scan_len` bytes (a copy cut short): the last block ends at the end of the file, so its own
    /// length tells whether it was cut.
    pub fn cut(&self, scan_len: u64) -> bool {
        self.at >= scan_len
            || self.end > scan_len
            || self.at.saturating_add(u64::from(self.size)) > scan_len
    }
    /// The index row says the block holds points (+26 > 0) and it has no scan head (`head` false): left out, counted (`warn.wiff.damaged`). The value
    /// at +26 is not compared with the head's point count: on sparse product-ion scans it is not one; `scan::read_scan_block` checks the block's bytes.
    pub fn damaged(&self, head: bool) -> bool {
        !head && self.points > 0.0
    }
    /// Where the block's bytes end: its own length when the index row gives one, else the start of the next block.
    pub fn data_end(&self) -> u64 {
        match self.size {
            0 => self.end,
            n => self.end.min(self.at.saturating_add(u64::from(n))),
        }
    }
}

/// One sample (injection) of a `.wiff`.
pub struct Wiff {
    pub sample: u32,
    pub records: Vec<Record>,
    pub experiments: Vec<Experiment>,
    /// +1, -1, or 0 when the method has no ion spray voltage.
    pub polarity: i8,
    pub method: Option<Method>,
    /// Optical DAD / PDA 3D matrix and TWC when the sample has a `DAD` storage.
    pub dad: Option<dad::DadData>,
    pub devices: Vec<crate::mzml::Chromatogram>, // pressure, flow, ... of `Devices/Device_N/DevData` (`devices.rs`)
    /// `Component ID` of the mass spectrometer in the Log, and the creation time of the file: the header of the mzML.
    pub model: Option<String>,
    pub serial: Option<String>,
    pub source_type: Option<String>,
    pub created: Option<String>,
    /// SHA-1 (hex) of the `.wiff`, for the `sourceFile` of the mzML; the caller that has the bytes sets it (`Ole::sha1_hex`).
    pub wiff_sha1: Option<String>,
    /// Off by default. When set, a scheduled transition keeps only the points within its window around the target RT (as msconvert writes it).
    pub trim_scheduled: bool,
    /// Set when the sample is a section of the `.wiff.scan` that the `.wiff` index does not list: its name, method and times are those of this indexed sample.
    pub inferred_from: Option<u32>,
    scan_len: u64,
    /// Acquired dependent (IDA) scans whose precursor could not be read (`dde::assign`): written with none, counted in `warn.wiff.idaPrecursor`.
    pub unread_precursors: usize,
    /// Why this sample could not be opened (its method is refused): it has no records, and reading its data gives this error while the other samples stay readable.
    failure: Option<Error>,
}

/// A scan type the reader does not read: the code, the experiment and the name of the type (`modi`), so that the message says what it is.
pub(crate) fn scan_type_error(code: u32, experiment: usize) -> Error {
    let kind = modes::mode(code).map_or("unknown", |m| m.tag);
    Error::new("err.file.wiffScanType").with("code", code).with("experiment", experiment).with("type", kind)
}

/// `head + width * n` bytes, `None` when it does not fit a `usize`: on a 32-bit target (the site) a count of 2^29 rows would wrap to a few bytes,
/// pass a length check that the 64-bit app fails, and start a loop of 2^29 turns.
pub(crate) fn span(head: usize, width: usize, n: usize) -> Option<usize> {
    n.checked_mul(width)?.checked_add(head)
}

fn bad(detail: &str) -> Error {
    Error::new("err.file.wiff").with("detail", detail)
}

/// The container of a `.wiff`; one cut short (sectors its header or its FAT use lie past the end) is refused whole: its streams would read empty or short,
/// so the samples, the index or the method would be missing unsaid.
pub fn container(wiff: Vec<u8>, name: &str) -> Result<Ole> {
    container_within(wiff, name, &crate::readers::Limits::held())
}

/// `container` bounded by `limits`: a file past the memory budget is told as such, not as a file that is not a `.wiff`.
pub fn container_within(wiff: Vec<u8>, name: &str, limits: &crate::readers::Limits) -> Result<Ole> {
    let size = wiff.len();
    let cut = Ole::header_cut(&wiff);
    let ole = Ole::open(wiff, name, limits).map_err(|e| match cut {
        _ if e.key == Error::memory_budget().key => e,
        Some(needed) => truncated(name, size, needed),
        None => Error::new("err.file.notWiff").with("name", name),
    })?;
    match ole.cut_short() {
        Some(needed) => Err(truncated(name, size, needed)),
        None => Ok(ole),
    }
}

fn truncated(name: &str, size: usize, needed: u64) -> Error {
    Error::new("err.file.wiffTruncated")
        .with("name", name)
        .with("size", size)
        .with("needed", needed)
}

/// The numbers of the samples of the sample table, in order: an injection that acquired nothing (no `Idx` stream) is a sample with no scans,
/// listed like the others so that the site and `mzlab-cli convert` both show it as empty instead of dropping it unsaid.
pub fn samples(ole: &Ole) -> Vec<u32> {
    let mut v: Vec<u32> = ole
        .read(&format!("{SAMPLES}SampleIdxTable"))
        .map(|t| idx::sections(&t).into_iter().map(|s| s.0).collect())
        .unwrap_or_default();
    crate::sortc::slice_ord(&mut v);
    v.dedup();
    v
}

/// The number of periods of method `method` (the `PeriodN` storages of its first device method).
pub(crate) fn period_count(ole: &Ole, method: u32) -> usize {
    let prefix = format!("MethodSubtree/Method{method}/DeviceMethod0/Period");
    let mut n = 0;
    for p in ole.paths_list() {
        if let Some(rest) = p.strip_prefix(&prefix) {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(k) = digits.parse::<usize>() {
                n = n.max(k + 1);
            }
        }
    }
    n
}

/// The `ExperimentHeader` of experiment `i` of method `no` (also read by `dde.rs`).
fn header(ole: &Ole, no: u32, i: usize) -> Option<Vec<u8>> {
    ole.read(&format!("MethodSubtree/Method{no}/DeviceMethod0/Period0/Experiment{i}/ExperimentHeader"))
}

fn experiments(ole: &Ole, sample: u32) -> Vec<Experiment> {
    let mut out = Vec::new();
    while let Some(d) = header(ole, sample, out.len()) {
        let code = d
            .get(HEADER_CODE..HEADER_CODE + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let prec = d
            .get(HEADER_PRECURSOR..HEADER_PRECURSOR + 8)
            .map(|b| f64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
            .filter(|p| p.is_finite() && *p > 0.0);
        let product = code == Some(CODE_PRODUCT);
        out.push(Experiment {
            code,
            level: if product { 2 } else { 1 },
            precursor: if product { prec } else { None },
            parent: None,
            fixed: None,
            ce: None,
            iso_half: None,
            window_s: None,
            polarity: d
                .get(HEADER_POLARITY..HEADER_POLARITY + 4)
                .and_then(|b| <[u8; 4]>::try_from(b).ok())
                .and_then(|b| match u32::from_le_bytes(b) {
                    0 => Some(1),
                    1 => Some(-1),
                    _ => None,
                }),
        });
    }
    out
}

/// The collision energy and the isolation window of the MS2 experiments: the method lists the energies of each scan experiment, and one value is the energy.
fn activation(experiments: &mut [Experiment], method: &Option<Method>) {
    let Some(m) = method else { return };
    for (i, e) in experiments
        .iter_mut()
        .enumerate()
        .filter(|(_, e)| e.level >= 2)
    {
        e.iso_half = Some(UNIT_HALF_WIDTH);
        if let Some(methodinfo::Kind::Scan { ce, .. }) =
            m.experiments.iter().find(|x| x.index == i).map(|x| &x.kind)
        {
            e.ce = match ce[..] {
                [v] => Some(v),
                _ => None,
            };
        }
    }
}

/// A SIM method of several periods is `err.file.wiffPeriods`, as the readers module refuses it: only Period0 is read, so its cycles would be laid out wrong.
fn sim_periods(ole: &Ole, sample: u32, experiments: &[Experiment]) -> Result<()> {
    if experiments.iter().any(|e| modes::sim(e.code)) {
        let p = period_count(ole, sample);
        if p > 1 {
            return Err(Error::new("err.file.wiffPeriods").with("periods", p));
        }
    }
    Ok(())
}

fn open_wiff(ole: &Ole, name: &str, scan_len: Option<u64>, sample: u32) -> Result<Wiff> {
    let scan_len = scan_len.ok_or_else(|| Error::new("err.file.wiffNoScan").with("name", name))?;
    let table = ole
        .read(&format!("{SAMPLES}SampleIdxTable"))
        .ok_or_else(|| bad("no sample table"))?;
    let section = idx::sections(&table)
        .into_iter()
        .find(|s| s.0 == sample)
        .ok_or_else(|| bad("no such sample"))?
        .1;
    let entries = ole
        .read(&format!("{SAMPLES}Sample{sample}/Idx"))
        .map(|b| idx::parse(&b))
        .unwrap_or_default();
    let mut experiments = self::experiments(ole, method_no(ole, sample));
    #[cfg(not(feature = "estesi"))]
    if experiments.is_empty() {
        experiments = self::experiments(ole, sample);
    }
    #[cfg(feature = "estesi")]
    {
        experiments = extended::refine(ole, sample, experiments)?;
        extended::mark_windows(ole, sample, &mut experiments);
    }
    sim_periods(ole, sample, &experiments)?;
    let ida = dde::ida_of(ole, sample, &mut experiments);
    let n_exp = experiments.len().max(1);
    let mut records: Vec<Record> = entries
        .iter()
        .enumerate()
        .map(|(k, e)| Record {
            at: section + e.offset as u64 + scan::PAYLOAD_SKIP,
            end: entries
                .get(k + 1)
                .map_or(scan_len, |n| section + n.offset as u64 + scan::PAYLOAD_SKIP),
            size: e.size,
            time_min: e.time_ms / 60000.0,
            tic: e.tic,
            cycle: k / n_exp,
            experiment: k % n_exp,
            points: if ida == dde::Ida::Stream { f64::NAN } else { e.points },
            precursor: None,
            charge: None,
        })
        .collect();
    let no = method_no(ole, sample);
    let no = if ole.has(&format!(
        "MethodSubtree/Method{no}/DeviceMethod0/Period0/Experiment0/ExperimentHeader"
    )) {
        no
    } else {
        sample
    };
    mass_check::check(ole, no, &experiments)?;
    let method = methodinfo::method_of_sample(ole, method_no(ole, sample));
    let method = method.or_else(|| methodinfo::method_of_sample(ole, sample));
    activation(&mut experiments, &method);
    let unread_precursors = dde::assign(ole, sample, ida, &experiments, &mut records, &method);
    let dad = dad::DadData::read(ole, sample);
    let (model, created, serial, source_type) = crate::method_wiff::run_facts_full(ole, sample);
    Ok(Wiff {
        sample,
        records,
        experiments,
        polarity: modes::source_polarity(method.as_ref()),
        method,
        dad,
        devices: devices::traces(ole, sample),
        model,
        serial,
        source_type,
        created,
        wiff_sha1: None,
        trim_scheduled: false,
        inferred_from: None,
        scan_len,
        unread_precursors,
        failure: None,
    })
}

impl Wiff {
    /// A sample whose method the reader refuses, kept in the list with no scans: `readable` gives `failure` when its data are asked for, so that one sample
    /// never makes the whole file fail.
    pub fn refused(sample: u32, scan_len: u64, failure: Error) -> Wiff {
        Wiff {
            sample,
            records: Vec::new(),
            experiments: Vec::new(),
            polarity: 0,
            method: None,
            dad: None,
            devices: Vec::new(),
            model: None,
            serial: None,
            source_type: None,
            created: None,
            wiff_sha1: None,
            trim_scheduled: false,
            inferred_from: None,
            scan_len,
            unread_precursors: 0,
            failure: Some(failure),
        }
    }

    /// Sample `sample` of the container, or (a scan type the reader does not know, several periods, a `.wiff2`) the sample kept with its reason: only a file-level error ends the open.
    pub fn open_or_refuse(ole: &Ole, name: &str, scan_len: u64, sample: u32) -> Result<Wiff> {
        match open_wiff(ole, name, Some(scan_len), sample) {
            Err(e) if e.key.starts_with("err.file.wiff") && e.key != "err.file.wiffNoScan" => {
                Ok(Wiff::refused(sample, scan_len, e))
            }
            other => other,
        }
    }

    /// A pair where no sample opens is refused with the reason of the first one; with one readable (or empty) sample the others stay listed.
    pub fn all_refused<'a>(samples: impl Iterator<Item = &'a Wiff>) -> Result<()> {
        let mut first = None;
        for w in samples {
            match &w.failure {
                Some(e) => first = first.or(Some(e)),
                None => return Ok(()),
            }
        }
        first.map_or(Ok(()), |e| Err(e.clone()))
    }

    /// Sample `sample` of the container. `scan_len` is the size of the `.wiff.scan`, `None` when that file is missing (`err.file.wiffNoScan`).
    pub fn open(ole: &Ole, name: &str, scan_len: Option<u64>, sample: u32) -> Result<Wiff> {
        open_wiff(ole, name, scan_len, sample)
    }

    /// The polarity of the scans of experiment `e`: the one of its header (offset 86, u32: 0 positive, 1 negative), else the sign of the ion spray voltage of the method.
    pub(crate) fn polarity_of(&self, e: &Experiment) -> i8 {
        e.polarity.unwrap_or(self.polarity)
    }
}

#[cfg(feature = "estesi")]
pub(crate) mod extended;
#[cfg(feature = "estesi")]
#[path = "scan_extended.rs"]
pub mod scan;
