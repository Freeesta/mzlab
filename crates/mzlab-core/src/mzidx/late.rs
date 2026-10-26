//! `build_mzml_spill` with the `source` object of the metadata given at the end (`upload::prepare_late`): used by the CLI index builder.

use std::io::{Read, Seek, SeekFrom};

use super::{build_con_spill, io_error, Output, ScanTable, Stats};
use crate::error::Result;
use crate::mzidx_spill::Spill;
use crate::mzml::{self, Item};

/// `build_mzml_spill` whose `source` object (`table.source_json`) is given by `late_source` when the metadata is written, so that the SHA-256 of
/// the file can be taken while the peaks are decoded.
pub fn build_mzml_late<R: Read + Seek>(
    table: &ScanTable,
    mut src: R,
    out: &mut dyn Output,
    budget: usize,
    spill: Option<&mut dyn Spill>,
    late_source: Option<&mut dyn FnMut() -> Result<String>>,
) -> Result<Stats> {
    build_con_spill(
        table,
        &mut |on| {
            src.seek(SeekFrom::Start(0)).map_err(io_error)?;
            mzml::parse(&mut src, |item| {
                if let Item::Spectrum(s) = item {
                    on(s)
                }
            })
        },
        out,
        budget,
        false,
        spill,
        late_source,
    )
}
