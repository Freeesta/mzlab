//! The check that a file is an mzML before its scan table is read (`prepare_in`, `prepare_parallelo`).

use crate::error::{Error, Result};
use crate::idxread::ReadAt;

/// The mzML reader of the Python engine refuses a file without "mzML" in its first 4096 bytes (`err.file.notMzml`): the scan table of a text that is
/// no mzML would be empty and the file would load as «no data» with no word on why. Readers of vendor sets (`lettore`) hold no mzML bytes and pass.
pub(super) fn refuse<R: ReadAt>(r: &mut R, size: u64, name: &str) -> Result<()> {
    if r.lettore().is_some() {
        return Ok(());
    }
    let mut head = vec![0u8; size.min(4096) as usize];
    r.read_at(0, &mut head)?;
    if head.windows(4).any(|w| w == b"mzML") {
        return Ok(());
    }
    Err(Error::new("err.file.notMzml").with("name", name))
}
