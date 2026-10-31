//! `build_mzml_spill` with the `source` object of the metadata given at the end (`upload::prepare_late`): used by the CLI index builder; and with the
//! arrays decoded by other instances (`build_mzml_corriere`: the helper workers of the browser).

use std::io::{Read, Seek, SeekFrom};

use super::{build_con_spill, io_error, Output, ScanTable, Stats};
use crate::error::Result;
use crate::mzidx_spill::Spill;
use crate::mzml::{self, Corriere, Item};

/// `build_mzml_spill` whose `source` object (`table.source_json`) is given by `late_source` when the metadata is written, so that the SHA-256 of
/// the file can be taken while the peaks are decoded.
pub fn build_mzml_late<R: Read + Seek>(
    table: &ScanTable,
    src: R,
    out: &mut dyn Output,
    budget: usize,
    spill: Option<&mut dyn Spill>,
    late_source: Option<&mut dyn FnMut() -> Result<String>>,
) -> Result<Stats> {
    con_parse(table, src, out, budget, spill, late_source, None)
}

/// `build_mzml_spill` with the arrays of every pass decoded through `c` (`mzml::parse_corriere`): the same bytes as without it.
pub fn build_mzml_corriere<R: Read + Seek>(
    table: &ScanTable,
    src: R,
    out: &mut dyn Output,
    budget: usize,
    spill: Option<&mut dyn Spill>,
    c: &mut dyn Corriere,
) -> Result<Stats> {
    con_parse(table, src, out, budget, spill, None, Some(c))
}

fn con_parse<R: Read + Seek>(
    table: &ScanTable,
    mut src: R,
    out: &mut dyn Output,
    budget: usize,
    spill: Option<&mut dyn Spill>,
    late_source: Option<&mut dyn FnMut() -> Result<String>>,
    mut c: Option<&mut dyn Corriere>,
) -> Result<Stats> {
    build_con_spill(
        table,
        &mut |on| {
            src.seek(SeekFrom::Start(0)).map_err(io_error)?;
            let f = |item| {
                if let Item::Spectrum(s) = item {
                    on(s)
                }
            };
            match c.as_deref_mut() {
                Some(c) => mzml::parse_corriere(&mut src, f, c),
                None => mzml::parse(&mut src, f),
            }
        },
        out,
        budget,
        false,
        spill,
        late_source,
    )
}
