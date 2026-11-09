//! `build_mzml_spill` with the `source` object of the metadata given at the end (`upload::prepare_late`): used by the CLI index builder; and with the
//! arrays decoded by other instances (`build_mzml_corriere`: the helper workers of the browser).

use std::cell::Cell;
use std::io::{Read, Seek, SeekFrom};

use super::{build_with_spill, io_error, Output, ScanTable, Stats};
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
    with_parse(table, src, out, budget, spill, late_source, None)
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
    with_parse(table, src, out, budget, spill, None, Some(c))
}

fn with_parse<R: Read + Seek>(
    table: &ScanTable,
    mut src: R,
    out: &mut dyn Output,
    budget: usize,
    spill: Option<&mut dyn Spill>,
    late_source: Option<&mut dyn FnMut() -> Result<String>>,
    mut c: Option<&mut dyn Corriere>,
) -> Result<Stats> {
    let stop = Cell::new(false);
    build_with_spill(
        table,
        &mut |on| {
            src.seek(SeekFrom::Start(0)).map_err(io_error)?;
            let src = Fermabile {
                inner: &mut src,
                stop: &stop,
            };
            let f = |item| {
                if let Item::Spectrum(s) = item {
                    on(s)
                }
            };
            match c.as_deref_mut() {
                Some(c) => mzml::parse_corriere(src, f, c),
                None => mzml::parse(src, f),
            }
        },
        out,
        budget,
        false,
        spill,
        late_source,
        Some(&stop),
    )
}

/// The mzML of a pass, which stops being read once pass 1 has failed (`build_with_spill`): the parser ends with an error that is not the answer.
struct Fermabile<'a, R> {
    inner: &'a mut R,
    stop: &'a Cell<bool>,
}

impl<R: Read> Read for Fermabile<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.stop.get() {
            return Err(std::io::Error::other("index build stopped"));
        }
        self.inner.read(buf)
    }
}
