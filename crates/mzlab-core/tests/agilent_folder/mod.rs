//! The real `.D` folders of the private data repository (`GC-MS/*.D`) loaded into memory, shared by the Agilent tests that read them.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use mzlab_core::agilent::{AgilentD, Files};

#[path = "../../../../tools/dati_percorsi.rs"]
pub mod dati_percorsi;

/// The `GC-MS` folder of the data repository, if there is one (see `tools/dati_percorsi.rs`).
pub fn gcms() -> Option<PathBuf> {
    dati_percorsi::percorso("dati/GC-MS")
}

pub fn read(d: &Path, f: &str) -> Option<Vec<u8>> {
    std::fs::read(d.join(f)).ok()
}

/// The bytes of the files of one `.D` folder.
pub struct Folder {
    pub name: String,
    pub scan: Vec<u8>,
    pub xsd: Vec<u8>,
    pub peak: Vec<u8>,
    pub small: Vec<(String, Vec<u8>)>,
}

pub const SMALL: [&str; 13] = [
    "AcqData/MSTS.xml",
    "AcqData/Devices.xml",
    "AcqData/Contents.xml",
    "AcqData/MSTree2.bin",
    "acqmeth.txt",
    "AcqData/sample_info.xml",
    "AcqData/sequence.xml",
    "AcqData-CumulativeAuditTrail.xml",
    "checksum.xml",
    "GC.ini",
    "PRE_POST.INI",
    "runstart.txt",
    "IndexedDataConverter.log",
];

impl Folder {
    pub fn load(f: &Path) -> Folder {
        let acq = f.join("AcqData");
        Folder {
            name: f.file_name().unwrap().to_string_lossy().into_owned(),
            scan: read(&acq, "MSScan.bin").unwrap(),
            xsd: read(&acq, "MSScan.xsd").unwrap(),
            peak: std::fs::read(acq.join("MSPeak.bin")).unwrap(),
            small: SMALL
                .iter()
                .filter_map(|n| read(f, n).map(|b| (n.to_string(), b)))
                .collect(),
        }
    }

    pub fn open(&self) -> AgilentD {
        let get = |n: &str| {
            self.small
                .iter()
                .find(|(k, _)| k == n)
                .map(|(_, b)| b.as_slice())
        };
        let others: Vec<(&str, &[u8])> = self
            .small
            .iter()
            .skip(5)
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        let files = Files {
            ms_scan: &self.scan,
            data_ms: None,
            ms_scan_xsd: Some(&self.xsd),
            ms_peak_len: self.peak.len() as u64,
            ms_ts: get(SMALL[0]),
            devices: get(SMALL[1]),
            contents: get(SMALL[2]),
            ms_tree: get(SMALL[3]),
            acqmeth: get(SMALL[4]),
            others: &others,
        };
        AgilentD::open(&self.name, &files).unwrap()
    }
}

pub fn folders() -> Option<Vec<PathBuf>> {
    let data = gcms()?;
    let mut v: Vec<PathBuf> = std::fs::read_dir(data)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "D"))
        .collect();
    v.sort();
    assert!(!v.is_empty());
    Some(v)
}
