//! The scan table of an mzML whose fingerprints are still being taken: `prepare_late` and the pieces `upload::prepare` shares with it.

use super::{fingerprint, non_mzml, source_head, Passata};
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::json::Out;
use crate::mzidx::ScanTable;
use crate::scantable::{self, Table};
use crate::window::Windowed;

/// The `source` object of the metadata of an mzML.
pub(super) fn source_text(name: &str, size: u64, probe: &str, sha256: &str) -> String {
    let mut source = Out::default();
    source_head(&mut source, name, size, probe, sha256);
    source.raw("}");
    source.0
}

pub(super) fn tabella_con(mut t: Table, source_json: String) -> ScanTable {
    let (json, c) = t.drain_json();
    let run = t.run_json();
    ScanTable {
        level: c.level,
        polarity: c.polarity,
        rt: c.rt,
        tic: c.tic,
        scans_json: json.into_bytes(),
        source_json,
        run_json: run,
        hr1: t.hr1,
    }
}

/// Native only: the scan table of an mzML (its `source_json` still empty) and the function that gives the `source` object once the fingerprints,
/// which a second thread takes from `r2` meanwhile, are done: `mzidx::build_mzml_late` asks for it only when it writes the metadata.
#[cfg(not(target_arch = "wasm32"))]
pub fn prepare_late<R: ReadAt, R2: ReadAt + Send + 'static>(
    r: &mut R,
    mut r2: R2,
    size: u64,
    name: &str,
) -> Result<(ScanTable, impl FnMut() -> Result<String>)> {
    non_mzml::refuse(r, size, name)?;
    let prints = std::thread::spawn(move || fingerprint(&mut r2, size, Passata::senza_sha1(size)));
    let t = scantable::read(&mut Windowed::new(r, size), size)?;
    let name = name.to_string();
    let mut prints = Some(prints);
    Ok((tabella_con(t, String::new()), move || {
        let h = prints
            .take()
            .ok_or_else(|| Error::new("err.file.unknown"))?;
        let (sha256, _, probe) = h
            .join()
            .unwrap_or_else(|_| Err(Error::new("err.file.unknown")))?;
        Ok(source_text(&name, size, &probe, &sha256))
    }))
}
