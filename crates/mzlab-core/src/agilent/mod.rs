//! Reader of Agilent MassHunter `.D` folders (GC/MS single quadrupole 5977, scan and SIM): `AcqData/MSScan.bin` is the index with one record per
//! scan, `AcqData/MSPeak.bin` the points. What was decoded from real files, how it was checked and what is NOT known (other detectors, LC/MS, MS/MS,
//! mobility, negative polarity) is written in `docs/agenti/agilent.md`. The folder is given as bytes of the small files and a reader of `MSPeak.bin`,
//! so the same code serves the browser (files picked from a folder) and the desktop.

pub mod info;
pub mod method;
mod peak;
mod run;
pub mod scan;

use crate::error::{Error, Result};
use info::{Contents, Device, Segment, Tree};
use method::AcqMethod;
pub use scan::ScanRec;

/// `ScanType` values seen: 1 full scan, 2 selected ions.
pub const SCAN_FULL: i32 = 1;
pub const SCAN_SIM: i32 = 2;

/// The files of the folder that are read whole (all small); `MSPeak.bin` is read by offset and only its size is needed here.
#[derive(Debug, Clone, Copy, Default)]
pub struct Files<'a> {
    pub ms_scan: &'a [u8],
    /// `AcqData/MSScan.xsd`: the record layout of this file (the built-in layout of the GC/MS runs when it is missing).
    pub ms_scan_xsd: Option<&'a [u8]>,
    pub ms_peak_len: u64,
    pub ms_ts: Option<&'a [u8]>,
    pub devices: Option<&'a [u8]>,
    pub contents: Option<&'a [u8]>,
    pub ms_tree: Option<&'a [u8]>,
    pub acqmeth: Option<&'a [u8]>,
}

/// One run (one `.D` folder).
pub struct AgilentD {
    pub name: String,
    pub records: Vec<ScanRec>,
    pub segments: Vec<Segment>,
    pub device: Option<Device>,
    pub contents: Option<Contents>,
    pub tree: Option<Tree>,
    pub method: Option<AcqMethod>,
}

impl AgilentD {
    /// `name`: the folder name (`xxx.D`). A folder without `AcqData/MSScan.bin` is `err.file.notAgilentD`, an index that is not understood is `err.file.agilent`.
    pub fn open(name: &str, f: &Files) -> Result<AgilentD> {
        if f.ms_scan.is_empty() {
            return Err(Error::new("err.file.notAgilentD").with("name", name));
        }
        let segments = f.ms_ts.map(info::segments).unwrap_or_default();
        let layout = match f.ms_scan_xsd {
            Some(x) => scan::Layout::from_xsd(&String::from_utf8_lossy(x))?,
            None => scan::Layout::builtin(),
        };
        let announced = Some(segments.iter().map(|s| s.scans).sum::<usize>());
        let records = scan::parse(f.ms_scan, &layout, f.ms_peak_len, announced)?;
        if records.is_empty() {
            return Err(Error::new("err.file.agilent").with("detail", "no scans"));
        }
        Ok(AgilentD {
            name: name.to_string(),
            records,
            segments,
            device: f.devices.and_then(|d| info::devices(d).into_iter().next()),
            contents: f.contents.and_then(info::contents),
            tree: f.ms_tree.and_then(info::tree),
            method: f.acqmeth.map(method::parse),
        })
    }

    /// +1 / -1 / 0 (not said). Code 0 is "positive" in the three runs (the tree says "Positive" for it); code 1 as negative is the vendor convention
    /// and has NOT been seen in a file; any other code is "not said".
    pub fn polarity_of(&self, r: &ScanRec) -> i8 {
        match r.polarity {
            0 => 1,
            1 => -1,
            _ => 0,
        }
    }

    pub fn is_gc(&self) -> bool {
        self.contents.as_ref().is_some_and(|c| c.separation == 2)
    }
}
