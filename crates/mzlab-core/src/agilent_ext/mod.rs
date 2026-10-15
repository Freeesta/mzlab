//! Reader of Agilent MassHunter `.D` folders (GC/MS single quadrupole 5977, scan and SIM): `AcqData/MSScan.bin` is the index with one record per
//! scan, `AcqData/MSPeak.bin` the points. What was decoded from real files, how it was checked and what is NOT known (other detectors, LC/MS, MS/MS,
//! mobility, negative polarity) is written in `docs/agenti/agilent.md`. The folder is given as bytes of the small files and a reader of `MSPeak.bin`,
//! so the same code serves the browser (files picked from a folder) and the desktop.

mod datams;
mod doc;
#[path = "../agilent/info.rs"]
pub mod info;
mod lccal;
mod lcdoc;
mod lcmethod;
mod lcmodules;
pub mod lcms;
mod lcprofile;
pub mod lettore;
#[path = "../agilent/meta.rs"]
pub mod meta;
#[path = "../agilent/method.rs"]
pub mod method;
#[path = "../agilent/peak.rs"]
mod peak;
#[path = "../agilent/quant.rs"]
pub mod quant;
mod run;
#[path = "../agilent/scan.rs"]
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
    /// `data.ms`, the ChemStation file: the data of an older `.D` that has no `AcqData` (`ms_scan` empty); with `AcqData` it only adds its header to the metadata.
    pub data_ms: Option<&'a [u8]>,
    /// `AcqData/MSScan.xsd`: the record layout of this file (the built-in layout of the GC/MS runs when it is missing).
    pub ms_scan_xsd: Option<&'a [u8]>,
    pub ms_peak_len: u64,
    pub ms_ts: Option<&'a [u8]>,
    pub devices: Option<&'a [u8]>,
    pub contents: Option<&'a [u8]>,
    pub ms_tree: Option<&'a [u8]>,
    pub acqmeth: Option<&'a [u8]>,
    /// Other small files of the folder, (path relative to the `.D`, bytes): `sample_info.xml`, `sequence.xml`, audit trail, `GC.ini`, `runstart.txt`... kept in `AgilentD::metadata`.
    pub others: &'a [(&'a str, &'a [u8])],
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
    /// Every name/value of the side files, with its origin (`meta::collect`), `Devices.xml`, `Contents.xml`, `MSTS.xml` and the method report included.
    pub metadata: Vec<meta::Item>,
    /// Names of `ScanRec::extra`.
    pub extra_names: Vec<String>,
    /// SHA-1 (hex) of each file given to `open`, by its path in the folder (`MSPeak.bin` is hashed while converting).
    pub file_sha1: Vec<(String, String)>,
    /// The points of every scan when the run comes from `data.ms` (m/z and abundances; `MSPeak.bin` does not exist then).
    pub(crate) inline: Option<Vec<(Vec<f64>, Vec<f64>)>>,
    /// The mass calibration of a Q-TOF run (`with_calibration`).
    pub(crate) calibration: lcms::Calibration,
    /// The LC/MS method of `AcqData/AcqMethod.xml` (`with_lc_files`).
    pub(crate) lcm: Option<lcmethod::LcMethod>,
    /// Every device of `Devices.xml` (the first is the mass spectrometer).
    pub(crate) devices_all: Vec<Device>,
    /// The member that holds the points (`MSPeak.bin`, or `MSProfile.bin` of a profile run).
    pub data_file: &'static str,
}

impl AgilentD {
    /// `name`: the folder name (`xxx.D`). A folder without `AcqData/MSScan.bin` is `err.file.notAgilentD`, an index that is not understood is `err.file.agilent`.
    pub fn open(name: &str, f: &Files) -> Result<AgilentD> {
        if f.ms_scan.is_empty() {
            return match f.data_ms {
                Some(d) => Self::open_data_ms(name, f, d),
                None => Err(Error::new("err.file.notAgilentD").with("name", name)),
            };
        }
        let segments = f.ms_ts.map(info::segments).unwrap_or_default();
        let layout = match f.ms_scan_xsd {
            Some(x) => scan::Layout::from_xsd_ext(&String::from_utf8_lossy(x))?.fit(f.ms_scan),
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
            metadata: metadata(f),
            extra_names: layout.extra_names(),
            file_sha1: hashes(f),
            inline: None,
            calibration: Default::default(),
            lcm: None,
            devices_all: Vec::new(),
            data_file: "AcqData/MSPeak.bin",
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

/// False for a document nested deeper than 256 elements: no instrument writes one, and a tree of that depth is read recursively.
pub(super) fn shallow(b: &[u8]) -> bool {
    let (mut depth, mut i) = (0usize, 0usize);
    while let Some(k) = b.get(i..).and_then(|r| r.iter().position(|&c| c == b'<')) {
        i += k + 1;
        match b.get(i) {
            Some(b'/') => depth = depth.saturating_sub(1),
            Some(c) if c.is_ascii_alphabetic() => {
                let end = b
                    .get(i..)
                    .and_then(|r| r.iter().position(|&c| c == b'>'))
                    .map_or(b.len(), |e| i + e);
                if b.get(end.wrapping_sub(1)) != Some(&b'/') {
                    depth += 1;
                    if depth > 256 {
                        return false;
                    }
                }
            }
            _ => {}
        }
    }
    true
}

fn metadata(f: &Files) -> Vec<meta::Item> {
    let head = f
        .ms_scan
        .get(..scan::HEADER_LEN)
        .filter(|_| !f.ms_scan.is_empty());
    let named: [(&str, Option<&[u8]>); 7] = [
        ("AcqData/MSScan.bin#header", head),
        ("AcqData/MSTree2.bin", f.ms_tree),
        ("AcqData/MSTS.xml", f.ms_ts),
        ("AcqData/Devices.xml", f.devices),
        ("AcqData/Contents.xml", f.contents),
        ("acqmeth.txt", f.acqmeth),
        ("AcqData/MSScan.xsd", f.ms_scan_xsd),
    ];
    let mut list: Vec<(&str, &[u8])> = named
        .iter()
        .filter_map(|(n, b)| b.map(|b| (*n, b)))
        .collect();
    list.extend(f.others.iter().copied());
    list.retain(|(n, b)| !n.ends_with(".xml") || shallow(b));
    let mut items = meta::collect(&list);
    if let (false, Some(b)) = (f.ms_scan.is_empty(), f.data_ms) {
        if let Ok(d) = datams::parse(b) {
            items.extend(datams::items(&d, b));
        }
    }
    items
}

/// SHA-1 of the files given to `open` (path in the folder, hex).
fn hashes(f: &Files) -> Vec<(String, String)> {
    use sha1::{Digest as _, Sha1};
    let named: [(&str, Option<&[u8]>); 8] = [
        (
            "AcqData/MSScan.bin",
            Some(f.ms_scan).filter(|b| !b.is_empty()),
        ),
        ("data.ms", f.data_ms),
        ("AcqData/MSScan.xsd", f.ms_scan_xsd),
        ("AcqData/MSTS.xml", f.ms_ts),
        ("AcqData/Devices.xml", f.devices),
        ("AcqData/Contents.xml", f.contents),
        ("AcqData/MSTree2.bin", f.ms_tree),
        ("acqmeth.txt", f.acqmeth),
    ];
    let mut list: Vec<(&str, &[u8])> = named
        .iter()
        .filter_map(|(n, b)| b.map(|b| (*n, b)))
        .collect();
    list.extend(f.others.iter().copied());
    list.iter()
        .map(|(n, b)| {
            (
                n.to_string(),
                Sha1::digest(b).iter().map(|x| format!("{x:02x}")).collect(),
            )
        })
        .collect()
}

impl AgilentD {
    /// The value of a field of a side file (`metadata`), by origin and name.
    pub fn field(&self, origin: &str, name: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|i| i.origin == origin && i.name == name)
            .map(|i| i.value.as_str())
    }

    /// Acquisition time as the instrument wrote it, with its UTC offset and without fractions of a second (`2025-05-15T16:00:45+01:00`).
    pub fn acquired(&self) -> Option<String> {
        let t = self.contents.as_ref()?.acquired.trim();
        if t.len() < 19 {
            return None;
        }
        let (head, tail) = t.split_at(19);
        let zone = tail.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
        Some(format!("{head}{zone}"))
    }

    /// `Blank`, `Sample`, ... from `sequence.xml`.
    pub fn sample_type(&self) -> Option<&str> {
        self.field(
            "AcqData/sequence.xml",
            "SequenceTableDataSet/Sequence/SampleType",
        )
    }

    pub fn sample_name(&self) -> Option<&str> {
        self.field(
            "AcqData/sequence.xml",
            "SequenceTableDataSet/Sequence/SampleName",
        )
    }
}
