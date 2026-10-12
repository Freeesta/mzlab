//! Reader of SCIEX Analyst `.wiff` + `.wiff.scan` pairs (3200 QTRAP and relatives), in pure Rust on top of `ole`: the container
//! gives the scan index (`idx`), the `.scan` gives the points (`scan`). What is and is not known of the format: `docs/agenti/wiff.md`.
//! The format was studied in OpenSXRaw (Sigilweaver, Apache-2.0); the code here is written anew.

pub mod idx;
pub mod scan;

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::ole::Ole;
pub use idx::Record;
pub use scan::Point;

const SAMPLES: &str = "SampleSubtree/";

/// One sample of a `.wiff`: its scan index. The points are read on demand from the `.wiff.scan`.
pub struct Wiff {
    pub sample: String,
    pub scans: Vec<Record>,
    scan_len: u64,
}

fn bad(detail: &str) -> Error {
    Error::new("err.file.wiff").with("detail", detail)
}

fn sort_key(n: &str) -> (u32, String) {
    (n.strip_prefix("Sample").and_then(|d| d.parse().ok()).unwrap_or(u32::MAX), n.to_string())
}

/// The samples of a `.wiff` (`Sample1`, `Sample2`, ... in numeric order).
pub fn samples(ole: &Ole) -> Vec<String> {
    let mut v: Vec<String> = ole.paths_list().filter_map(|p| p.strip_prefix(SAMPLES)?.split('/').next().map(str::to_string)).collect();
    v.sort_by_key(|n| sort_key(n));
    v.dedup();
    v
}

impl Wiff {
    /// Opens `sample` (`None`: the only one; several samples without a choice are an error). `scan_len` is the size of the `.wiff.scan`,
    /// `None` when that file is missing, which is `err.file.wiffNoScan`.
    pub fn open(wiff: Vec<u8>, name: &str, scan_len: Option<u64>, sample: Option<&str>) -> Result<Wiff> {
        let ole = Ole::open(wiff, name).map_err(|_| Error::new("err.file.notWiff").with("name", name))?;
        let scan_len = scan_len.ok_or_else(|| Error::new("err.file.wiffNoScan").with("name", name))?;
        let all = samples(&ole);
        let sample = match (sample, all.as_slice()) {
            (Some(s), _) => s.to_string(),
            (None, [only]) => only.clone(),
            (None, []) => return Err(bad("no sample")),
            _ => return Err(bad("several samples")),
        };
        let data = ole.read(&format!("{SAMPLES}{sample}/Idx")).ok_or_else(|| bad("no Idx"))?;
        let scans = idx::parse(&data).ok_or_else(|| bad("short Idx"))?;
        Ok(Wiff { sample, scans, scan_len })
    }

    /// The points of scan `i` (0-based).
    pub fn points<R: ReadAt>(&self, scan: &mut R, i: usize) -> Result<Vec<Point>> {
        let r = self.scans.get(i).ok_or_else(|| bad("scan number"))?;
        let next = self.scans.get(i + 1).map_or(self.scan_len, |n| n.offset as u64);
        scan::read_block(scan, self.scan_len, r.offset, r.size, next)
    }

    /// The total ion chromatogram of the index: (RT in minutes, TIC in cps).
    pub fn tic(&self) -> (Vec<f64>, Vec<f64>) {
        (self.scans.iter().map(|r| r.rt as f64).collect(), self.scans.iter().map(|r| r.tic).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::scan::{decode, Point};

    #[test]
    fn tokens() {
        assert_eq!(decode(&[0x29, 0x81]), vec![Point { bin: 41, count: 1 }]);
        assert_eq!(decode(&[0, 0xfd, 0x55, 1]), vec![Point { bin: 0, count: 341 }]);
        assert_eq!(decode(&[0x29, 0x81, 0xff, 0xff, 0xff, 0xff, 0x29, 0x81]).len(), 1);
        let v = decode(&[5, 0xfd, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(v, vec![Point { bin: 5, count: 0xffff }, Point { bin: 5, count: 0xff }]);
    }
}
