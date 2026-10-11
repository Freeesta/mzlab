//! What the page needs when a file is uploaded, without Python (`mzidx.builder_inputs`, `source_and_run`, `probe` of `mzlab/reader/mzidx.py`):
//! the arrays of the scan table for the index builder, the `source` and `run` objects of the metadata, and the SHA-256 of the file.

use sha1::{Digest as _, Sha1};
use sha2::Sha256;

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::json::Out;
use crate::mzidx::ScanTable;
use crate::scantable;

const PROBE: u64 = 65536;
const STEP: u64 = 1 << 20;

pub struct Prepared {
    pub table: ScanTable,
    pub sha256: String,
    pub probe: String,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Reads the whole file once for its SHA-256 and keeps the first and last 64 KB for the probe (SHA-1 of the size, the head and the tail).
pub fn fingerprint<R: ReadAt>(r: &mut R, size: u64) -> Result<(String, String)> {
    let mut sha = Sha256::new();
    let mut head = vec![];
    let mut at = 0;
    let mut buf = vec![0u8; STEP as usize];
    while at < size {
        let n = (size - at).min(STEP) as usize;
        r.read_at(at, &mut buf[..n])?;
        sha.update(&buf[..n]);
        if head.len() < PROBE as usize {
            head.extend_from_slice(&buf[..n.min(PROBE as usize - head.len())]);
        }
        at += n as u64;
    }
    let from = size.saturating_sub(PROBE);
    let mut tail = vec![0u8; (size - from) as usize];
    r.read_at(from, &mut tail)?;
    let mut p = Sha1::new();
    p.update(size.to_string().as_bytes());
    p.update(&head);
    p.update(&tail);
    Ok((hex(&sha.finalize()), hex(&p.finalize())))
}

/// Everything the index builder and the loading table need of an mzML: the scan table, the metadata objects and the fingerprints.
pub fn prepare<R: ReadAt>(r: &mut R, size: u64, name: &str) -> Result<Prepared> {
    let t = scantable::read(r, size)?;
    let (sha256, probe) = fingerprint(r, size)?;
    let mut source = Out::default();
    source
        .raw("{\"name\":")
        .str(name)
        .raw(",\"size\":")
        .int(size as i64)
        .raw(",\"mtime_ns\":0,\"probe\":")
        .str(&probe)
        .raw(",\"sha256\":")
        .str(&sha256)
        .raw("}");
    let run = format!(
        "{{\"nce\":{},\"hr1\":{},\"n_chromatograms\":{}}}",
        t.nce, t.hr1, t.n_chromatograms
    );
    let table = ScanTable {
        level: t.rows.iter().map(|s| s.level).collect(),
        polarity: t.rows.iter().map(|s| s.polarity).collect(),
        rt: t.rows.iter().map(|s| s.rt).collect(),
        tic: t.rows.iter().map(|s| s.tic).collect(),
        scans_json: t.rows_json().into_bytes(),
        source_json: source.0,
        run_json: run,
        hr1: t.hr1,
    };
    Ok(Prepared {
        table,
        sha256,
        probe,
    })
}
